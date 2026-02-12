use std::collections::HashMap;

use maple_mwl_types::{TemporalAnchor, WorldlineId};
use tracing::debug;

use crate::error::FinancialError;
use crate::types::{AssetId, ProjectedBalance, SettlementEvent};

/// EVOS — Balance-as-Projection engine.
///
/// Per I.ME-FIN-1: "Balance is not a stored number. It is a projection
/// computed by replaying the committed settlement trajectory."
///
/// EVOS maintains the settlement trajectory (an append-only sequence of
/// settlement events) and computes balances by replaying that trajectory.
/// There is NO stored balance — it is always recomputed.
pub struct BalanceProjection {
    /// Settlement trajectory: worldline -> list of settlement events (append-only)
    trajectories: HashMap<WorldlineId, Vec<SettlementEvent>>,
}

impl BalanceProjection {
    /// Create a new balance projection engine.
    pub fn new() -> Self {
        Self {
            trajectories: HashMap::new(),
        }
    }

    /// Record a settlement event in the trajectory.
    ///
    /// This is append-only — events cannot be modified or removed.
    pub fn record_settlement(&mut self, event: SettlementEvent) {
        debug!(
            worldline = %event.counterparty,
            asset = %event.asset,
            amount = event.amount_minor,
            "Recording settlement event in trajectory"
        );

        // Record for the counterparty involved
        // The event's amount_minor is from the perspective of whoever we're tracking
        self.trajectories
            .entry(event.counterparty.clone())
            .or_default()
            .push(event);
    }

    /// Record a settlement event for a specific worldline.
    pub fn record_for_worldline(
        &mut self,
        worldline: WorldlineId,
        event: SettlementEvent,
    ) {
        self.trajectories
            .entry(worldline)
            .or_default()
            .push(event);
    }

    /// Project the current balance for a worldline+asset.
    ///
    /// This replays the ENTIRE settlement trajectory to compute the balance.
    /// Per I.ME-FIN-1: balance is NEVER a stored number.
    pub fn project(
        &self,
        worldline: &WorldlineId,
        asset: &AssetId,
    ) -> Result<ProjectedBalance, FinancialError> {
        let trajectory = self.trajectories.get(worldline).ok_or_else(|| {
            FinancialError::EmptyTrajectory(asset.clone())
        })?;

        // Filter to this asset
        let asset_events: Vec<&SettlementEvent> = trajectory
            .iter()
            .filter(|e| e.asset == *asset)
            .collect();

        if asset_events.is_empty() {
            return Err(FinancialError::EmptyTrajectory(asset.clone()));
        }

        // Replay trajectory to compute balance
        let balance_minor: i64 = asset_events.iter().map(|e| e.amount_minor).sum();

        // Compute a simple hash of the trajectory for verification
        let trajectory_hash = Self::hash_trajectory(&asset_events);

        debug!(
            worldline = %worldline,
            asset = %asset,
            balance = balance_minor,
            events = asset_events.len(),
            "Balance projected from trajectory"
        );

        Ok(ProjectedBalance {
            worldline: worldline.clone(),
            asset: asset.clone(),
            balance_minor,
            trajectory_length: asset_events.len(),
            projected_at: TemporalAnchor::now(0),
            trajectory_hash,
        })
    }

    /// Project balance at a specific point in time.
    ///
    /// Only includes settlements that occurred at or before the given timestamp.
    pub fn project_at(
        &self,
        worldline: &WorldlineId,
        asset: &AssetId,
        at: &TemporalAnchor,
    ) -> Result<ProjectedBalance, FinancialError> {
        let trajectory = self.trajectories.get(worldline).ok_or_else(|| {
            FinancialError::EmptyTrajectory(asset.clone())
        })?;

        let asset_events: Vec<&SettlementEvent> = trajectory
            .iter()
            .filter(|e| e.asset == *asset && e.settled_at.physical_ms <= at.physical_ms)
            .collect();

        if asset_events.is_empty() {
            return Err(FinancialError::EmptyTrajectory(asset.clone()));
        }

        let balance_minor: i64 = asset_events.iter().map(|e| e.amount_minor).sum();
        let trajectory_hash = Self::hash_trajectory(&asset_events);

        Ok(ProjectedBalance {
            worldline: worldline.clone(),
            asset: asset.clone(),
            balance_minor,
            trajectory_length: asset_events.len(),
            projected_at: at.clone(),
            trajectory_hash,
        })
    }

    /// Get the trajectory length for a worldline+asset.
    pub fn trajectory_length(&self, worldline: &WorldlineId, asset: &AssetId) -> usize {
        self.trajectories
            .get(worldline)
            .map(|events| events.iter().filter(|e| e.asset == *asset).count())
            .unwrap_or(0)
    }

    /// Get all assets with trajectories for a worldline.
    pub fn assets_for_worldline(&self, worldline: &WorldlineId) -> Vec<AssetId> {
        self.trajectories
            .get(worldline)
            .map(|events| {
                let mut assets: Vec<AssetId> = events
                    .iter()
                    .map(|e| e.asset.clone())
                    .collect();
                assets.sort_by(|a, b| a.0.cmp(&b.0));
                assets.dedup();
                assets
            })
            .unwrap_or_default()
    }

    /// Compute a simple hash of the trajectory for integrity verification.
    fn hash_trajectory(events: &[&SettlementEvent]) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        for event in events {
            event.settlement_id.hash(&mut hasher);
            event.amount_minor.hash(&mut hasher);
        }
        format!("{:016x}", hasher.finish())
    }
}

impl Default for BalanceProjection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::SettlementType;
    use maple_mwl_types::{CommitmentId, IdentityMaterial};

    fn wid_a() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn wid_b() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
    }

    fn usd() -> AssetId {
        AssetId::new("USD")
    }

    fn btc() -> AssetId {
        AssetId::new("BTC")
    }

    fn settlement_event(
        worldline: WorldlineId,
        asset: AssetId,
        amount: i64,
        id: &str,
    ) -> SettlementEvent {
        SettlementEvent {
            settlement_id: id.into(),
            commitment_id: CommitmentId::new(),
            asset,
            amount_minor: amount,
            counterparty: worldline,
            settled_at: TemporalAnchor::now(0),
            settlement_type: SettlementType::FreeOfPayment,
        }
    }

    #[test]
    fn project_simple_balance() {
        let mut evos = BalanceProjection::new();

        // Record credits
        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), 100_000, "s1"),
        );
        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), 50_000, "s2"),
        );

        let balance = evos.project(&wid_a(), &usd()).unwrap();
        assert_eq!(balance.balance_minor, 150_000);
        assert_eq!(balance.trajectory_length, 2);
    }

    #[test]
    fn project_balance_with_debits() {
        let mut evos = BalanceProjection::new();

        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), 100_000, "s1"),
        );
        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), -30_000, "s2"),
        );

        let balance = evos.project(&wid_a(), &usd()).unwrap();
        assert_eq!(balance.balance_minor, 70_000);
    }

    #[test]
    fn project_balance_can_be_negative() {
        let mut evos = BalanceProjection::new();

        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), -50_000, "s1"),
        );

        let balance = evos.project(&wid_a(), &usd()).unwrap();
        assert_eq!(balance.balance_minor, -50_000);
    }

    #[test]
    fn project_filters_by_asset() {
        let mut evos = BalanceProjection::new();

        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), 100_000, "s1"),
        );
        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), btc(), 50_000_000, "s2"),
        );
        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), 25_000, "s3"),
        );

        let usd_balance = evos.project(&wid_a(), &usd()).unwrap();
        assert_eq!(usd_balance.balance_minor, 125_000);
        assert_eq!(usd_balance.trajectory_length, 2);

        let btc_balance = evos.project(&wid_a(), &btc()).unwrap();
        assert_eq!(btc_balance.balance_minor, 50_000_000);
        assert_eq!(btc_balance.trajectory_length, 1);
    }

    #[test]
    fn project_fails_for_unknown_worldline() {
        let evos = BalanceProjection::new();
        assert!(matches!(
            evos.project(&wid_a(), &usd()),
            Err(FinancialError::EmptyTrajectory(_))
        ));
    }

    #[test]
    fn project_fails_for_no_events_for_asset() {
        let mut evos = BalanceProjection::new();
        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), 100_000, "s1"),
        );

        assert!(matches!(
            evos.project(&wid_a(), &btc()),
            Err(FinancialError::EmptyTrajectory(_))
        ));
    }

    #[test]
    fn balance_is_always_recomputed() {
        let mut evos = BalanceProjection::new();

        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), 100_000, "s1"),
        );

        let balance1 = evos.project(&wid_a(), &usd()).unwrap();
        assert_eq!(balance1.balance_minor, 100_000);

        // Add more events
        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), 50_000, "s2"),
        );

        // Balance is recomputed, NOT cached
        let balance2 = evos.project(&wid_a(), &usd()).unwrap();
        assert_eq!(balance2.balance_minor, 150_000);
        assert_eq!(balance2.trajectory_length, 2);

        // Trajectory hashes differ because the trajectory changed
        assert_ne!(balance1.trajectory_hash, balance2.trajectory_hash);
    }

    #[test]
    fn trajectory_length_is_correct() {
        let mut evos = BalanceProjection::new();
        assert_eq!(evos.trajectory_length(&wid_a(), &usd()), 0);

        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), 100_000, "s1"),
        );
        assert_eq!(evos.trajectory_length(&wid_a(), &usd()), 1);

        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), 50_000, "s2"),
        );
        assert_eq!(evos.trajectory_length(&wid_a(), &usd()), 2);
    }

    #[test]
    fn assets_for_worldline_returns_unique_sorted() {
        let mut evos = BalanceProjection::new();

        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), 100_000, "s1"),
        );
        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), btc(), 50_000_000, "s2"),
        );
        evos.record_for_worldline(
            wid_a(),
            settlement_event(wid_b(), usd(), 25_000, "s3"),
        );

        let assets = evos.assets_for_worldline(&wid_a());
        assert_eq!(assets.len(), 2);
        assert_eq!(assets[0], btc()); // BTC sorts before USD
        assert_eq!(assets[1], usd());
    }

    #[test]
    fn project_at_filters_by_time() {
        let mut evos = BalanceProjection::new();

        // Create events with different timestamps
        let mut event1 = settlement_event(wid_b(), usd(), 100_000, "s1");
        event1.settled_at = TemporalAnchor::now(0);
        event1.settled_at.physical_ms = 1000;

        let mut event2 = settlement_event(wid_b(), usd(), 50_000, "s2");
        event2.settled_at = TemporalAnchor::now(0);
        event2.settled_at.physical_ms = 2000;

        let mut event3 = settlement_event(wid_b(), usd(), 25_000, "s3");
        event3.settled_at = TemporalAnchor::now(0);
        event3.settled_at.physical_ms = 3000;

        evos.record_for_worldline(wid_a(), event1);
        evos.record_for_worldline(wid_a(), event2);
        evos.record_for_worldline(wid_a(), event3);

        // Project at time 1500 — should only see event1
        let mut at = TemporalAnchor::now(0);
        at.physical_ms = 1500;
        let balance = evos.project_at(&wid_a(), &usd(), &at).unwrap();
        assert_eq!(balance.balance_minor, 100_000);
        assert_eq!(balance.trajectory_length, 1);

        // Project at time 2500 — should see event1 + event2
        at.physical_ms = 2500;
        let balance = evos.project_at(&wid_a(), &usd(), &at).unwrap();
        assert_eq!(balance.balance_minor, 150_000);
        assert_eq!(balance.trajectory_length, 2);

        // Project at time 3000 — should see all
        at.physical_ms = 3000;
        let balance = evos.project_at(&wid_a(), &usd(), &at).unwrap();
        assert_eq!(balance.balance_minor, 175_000);
        assert_eq!(balance.trajectory_length, 3);
    }
}
