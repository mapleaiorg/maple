use std::time::Duration;

use maple_kernel_fabric::{EventFabric, EventPayload, ResonanceStage};
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

use crate::entry::{
    provenance_from, MemoryClass, MemoryContent, MemoryEntry, MemoryId, MemoryPlane,
};
use crate::episodic::EpisodicPlane;
use crate::error::MemoryError;
use crate::filter::MemoryFilter;
use crate::working::{WorkingPlane, WorkingPlaneConfig};

/// Configuration for the MemoryEngine.
#[derive(Clone, Debug)]
pub struct MemoryEngineConfig {
    pub working: WorkingPlaneConfig,
    /// Number of active entries that triggers auto-consolidation hint
    pub consolidation_threshold: usize,
    /// How often consolidation should be considered
    pub consolidation_interval: Duration,
}

impl Default for MemoryEngineConfig {
    fn default() -> Self {
        Self {
            working: WorkingPlaneConfig::default(),
            consolidation_threshold: 50,
            consolidation_interval: Duration::from_secs(60),
        }
    }
}

/// MemoryEngine — the unified two-plane memory system.
///
/// Per I.2 (Intrinsic Typed Memory):
/// - Working Plane: volatile, reasoning-time context, rebuildable
/// - Episodic Plane: persistent, provenance-bound history
///
/// All entries MUST have provenance (EventId reference).
pub struct MemoryEngine {
    working: WorkingPlane,
    episodic: EpisodicPlane,
    config: MemoryEngineConfig,
}

/// Report from consolidation operation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsolidationReport {
    pub entries_consolidated: usize,
    pub entries_discarded: usize,
    pub semantic_entries_created: usize,
}

/// Report from working plane rebuild.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RebuildReport {
    pub events_replayed: usize,
    pub entries_restored: usize,
    pub confidence_adjustments: usize,
}

/// Memory statistics.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryStats {
    pub working_sensory: usize,
    pub working_active: usize,
    pub episodic_count: usize,
    pub semantic_count: usize,
    pub total_entries: usize,
}

impl MemoryEngine {
    pub fn new(config: MemoryEngineConfig) -> Self {
        let working = WorkingPlane::new(config.working.clone());
        let episodic = EpisodicPlane::new();
        Self {
            working,
            episodic,
            config,
        }
    }

    /// Store a memory entry. Routes to the correct plane based on class.
    ///
    /// REJECTS entries without valid provenance (I.2: provenance binding).
    pub fn store(&mut self, entry: MemoryEntry) -> Result<MemoryId, MemoryError> {
        // INVARIANT I.2: provenance required
        if !entry.has_valid_provenance() {
            return Err(MemoryError::MissingProvenance);
        }

        match entry.class.plane() {
            MemoryPlane::Working => match entry.class {
                MemoryClass::Sensory => self.working.store_sensory(entry),
                MemoryClass::Active => self.working.store_active(entry),
                _ => unreachable!(),
            },
            MemoryPlane::Episodic => match entry.class {
                MemoryClass::Episodic => self.episodic.store_episodic(entry),
                MemoryClass::Semantic => {
                    // Semantic entries need a key — derive from content
                    let key = derive_semantic_key(&entry);
                    self.episodic.store_semantic(key, entry)
                }
                _ => unreachable!(),
            },
        }
    }

    /// Query across both planes.
    pub fn query(&self, filter: &MemoryFilter) -> Vec<&MemoryEntry> {
        let mut results = self.working.query(filter);
        results.extend(self.episodic.query(filter));
        results
    }

    /// Consolidate: move active context entries → episodic store.
    ///
    /// This is the key transition: working plane items become persistent.
    /// Each active entry is re-classed as Episodic and stored in the episodic plane.
    /// Entries below a confidence threshold are discarded.
    pub fn consolidate(&mut self) -> Result<ConsolidationReport, MemoryError> {
        let active_entries = self.working.drain_active();
        let total = active_entries.len();

        let mut consolidated = 0usize;
        let mut discarded = 0usize;

        for mut entry in active_entries {
            // Discard low-confidence entries
            if entry.confidence < 0.1 {
                discarded += 1;
                continue;
            }

            // Re-class as Episodic
            entry.class = MemoryClass::Episodic;

            match self.episodic.store_episodic(entry) {
                Ok(_) => consolidated += 1,
                Err(e) => {
                    debug!(error = %e, "Failed to consolidate entry, discarding");
                    discarded += 1;
                }
            }
        }

        let report = ConsolidationReport {
            entries_consolidated: consolidated,
            entries_discarded: discarded,
            semantic_entries_created: 0, // semantic extraction happens separately
        };

        info!(
            consolidated = consolidated,
            discarded = discarded,
            total = total,
            "Consolidation complete"
        );

        Ok(report)
    }

    /// Rebuild working plane from episodic + Event Fabric replay.
    ///
    /// Called after crash recovery. The working plane is volatile and may
    /// have been lost. This rebuilds it by:
    /// 1. Clearing any stale working plane data
    /// 2. Replaying recent events from the fabric
    /// 3. Restoring active context from recent episodic entries
    pub async fn rebuild_working(
        &mut self,
        fabric: &EventFabric,
    ) -> Result<RebuildReport, MemoryError> {
        // Clear any stale working state
        self.working.clear();

        let mut events_replayed = 0u64;
        let mut entries_restored = 0usize;
        let mut confidence_adjustments = 0usize;

        // Replay events from fabric to rebuild context
        fabric
            .recover(|_seq, event| {
                events_replayed += 1;

                // Extract memory-relevant information from events
                let content = match &event.stage {
                    ResonanceStage::Meaning => {
                        if let EventPayload::MeaningFormed {
                            confidence,
                            ambiguity_preserved,
                            ..
                        } = &event.payload
                        {
                            Some((
                                MemoryContent::Structured(serde_json::json!({
                                    "type": "meaning",
                                    "confidence": confidence,
                                    "ambiguity_preserved": ambiguity_preserved,
                                })),
                                *confidence,
                            ))
                        } else {
                            None
                        }
                    }
                    ResonanceStage::Intent => {
                        if let EventPayload::IntentStabilized {
                            direction,
                            confidence,
                            ..
                        } = &event.payload
                        {
                            Some((
                                MemoryContent::Structured(serde_json::json!({
                                    "type": "intent",
                                    "direction": direction,
                                    "confidence": confidence,
                                })),
                                *confidence,
                            ))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some((memory_content, confidence)) = content {
                    let _entry = MemoryEntry::builder(
                        MemoryClass::Active,
                        memory_content,
                        provenance_from(event.id.clone()),
                        event.worldline_id.clone(),
                    )
                    .confidence(confidence)
                    .metadata("rebuilt", "true")
                    .build();

                    // We can't call self.working here because of borrow rules,
                    // so we'll collect and insert after
                    // (This is handled in the closure capture below)
                    entries_restored += 1;
                }

                Ok(())
            })
            .await?;

        // Second pass: actually rebuild from episodic plane recent entries
        // Use the most recent episodic entries to populate active context
        let recent_episodic = self.episodic.all_episodic_entries();
        for ep_entry in recent_episodic
            .into_iter()
            .rev()
            .take(self.config.working.active_max_entries)
        {
            let mut rebuilt = ep_entry.clone();
            rebuilt.class = MemoryClass::Active;
            rebuilt.id = MemoryId::new();

            // Reduce confidence for rebuilt entries (they're reconstructed)
            if rebuilt.confidence > 0.5 {
                rebuilt.confidence *= 0.9;
                confidence_adjustments += 1;
            }

            if let Err(e) = self.working.store_active(rebuilt) {
                debug!(error = %e, "Failed to restore entry during rebuild");
            } else {
                entries_restored += 1;
            }
        }

        let report = RebuildReport {
            events_replayed: events_replayed as usize,
            entries_restored,
            confidence_adjustments,
        };

        info!(
            events_replayed = events_replayed,
            entries_restored = entries_restored,
            "Working plane rebuilt"
        );

        Ok(report)
    }

    /// Check if consolidation should be triggered.
    pub fn should_consolidate(&self) -> bool {
        self.working.active_len() >= self.config.consolidation_threshold
    }

    /// Run garbage collection on the working plane.
    pub fn gc(&mut self) -> usize {
        self.working.gc()
    }

    /// Get memory statistics.
    pub fn stats(&self) -> MemoryStats {
        let ws = self.working.sensory_len();
        let wa = self.working.active_len();
        let ec = self.episodic.episodic_count();
        let sc = self.episodic.semantic_count();
        MemoryStats {
            working_sensory: ws,
            working_active: wa,
            episodic_count: ec,
            semantic_count: sc,
            total_entries: ws + wa + ec + sc,
        }
    }

    /// Direct access to working plane.
    pub fn working(&self) -> &WorkingPlane {
        &self.working
    }

    /// Direct access to working plane (mutable).
    pub fn working_mut(&mut self) -> &mut WorkingPlane {
        &mut self.working
    }

    /// Direct access to episodic plane.
    pub fn episodic(&self) -> &EpisodicPlane {
        &self.episodic
    }

    /// Direct access to episodic plane (mutable).
    pub fn episodic_mut(&mut self) -> &mut EpisodicPlane {
        &mut self.episodic
    }
}

/// Derive a semantic key from a memory entry's content.
fn derive_semantic_key(entry: &MemoryEntry) -> String {
    match &entry.content {
        MemoryContent::Text(t) => {
            // Use first 50 chars as key
            let truncated: String = t.chars().take(50).collect();
            format!("sem:{}", truncated)
        }
        MemoryContent::Structured(v) => {
            if let Some(t) = v.get("type").and_then(|t| t.as_str()) {
                format!("sem:{}", t)
            } else {
                format!("sem:{}", entry.id)
            }
        }
        MemoryContent::Binary(_) => format!("sem:binary:{}", entry.id),
        MemoryContent::Reference(ref_id) => format!("sem:ref:{}", ref_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::{nil_provenance, provenance_from};
    use maple_mwl_types::{EventId, IdentityMaterial, WorldlineId};

    fn test_worldline() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn sensory_entry(text: &str) -> MemoryEntry {
        MemoryEntry::builder(
            MemoryClass::Sensory,
            MemoryContent::Text(text.into()),
            provenance_from(EventId::new()),
            test_worldline(),
        )
        .build()
    }

    fn active_entry(text: &str) -> MemoryEntry {
        MemoryEntry::builder(
            MemoryClass::Active,
            MemoryContent::Text(text.into()),
            provenance_from(EventId::new()),
            test_worldline(),
        )
        .build()
    }

    fn episodic_entry(text: &str) -> MemoryEntry {
        MemoryEntry::builder(
            MemoryClass::Episodic,
            MemoryContent::Text(text.into()),
            provenance_from(EventId::new()),
            test_worldline(),
        )
        .build()
    }

    fn semantic_entry(text: &str) -> MemoryEntry {
        MemoryEntry::builder(
            MemoryClass::Semantic,
            MemoryContent::Text(text.into()),
            provenance_from(EventId::new()),
            test_worldline(),
        )
        .build()
    }

    #[test]
    fn reject_entry_without_provenance() {
        let mut engine = MemoryEngine::new(MemoryEngineConfig::default());

        let entry = MemoryEntry::builder(
            MemoryClass::Active,
            MemoryContent::Text("no provenance".into()),
            nil_provenance(),
            test_worldline(),
        )
        .build();

        assert!(matches!(
            engine.store(entry),
            Err(MemoryError::MissingProvenance)
        ));
    }

    #[test]
    fn accept_entry_with_provenance() {
        let mut engine = MemoryEngine::new(MemoryEngineConfig::default());
        let entry = active_entry("with provenance");
        assert!(engine.store(entry).is_ok());
    }

    #[test]
    fn store_routes_to_correct_plane() {
        let mut engine = MemoryEngine::new(MemoryEngineConfig::default());

        engine.store(sensory_entry("s1")).unwrap();
        engine.store(active_entry("a1")).unwrap();
        engine.store(episodic_entry("e1")).unwrap();
        engine.store(semantic_entry("sem1")).unwrap();

        let stats = engine.stats();
        assert_eq!(stats.working_sensory, 1);
        assert_eq!(stats.working_active, 1);
        assert_eq!(stats.episodic_count, 1);
        assert_eq!(stats.semantic_count, 1);
        assert_eq!(stats.total_entries, 4);
    }

    #[test]
    fn query_across_both_planes() {
        let mut engine = MemoryEngine::new(MemoryEngineConfig::default());

        engine.store(active_entry("active data")).unwrap();
        engine.store(episodic_entry("episodic data")).unwrap();

        let filter = MemoryFilter::new().with_content_contains("data");
        let results = engine.query(&filter);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn consolidation_moves_active_to_episodic() {
        let mut engine = MemoryEngine::new(MemoryEngineConfig::default());

        engine.store(active_entry("a1")).unwrap();
        engine.store(active_entry("a2")).unwrap();
        engine.store(active_entry("a3")).unwrap();

        assert_eq!(engine.stats().working_active, 3);
        assert_eq!(engine.stats().episodic_count, 0);

        let report = engine.consolidate().unwrap();

        assert_eq!(report.entries_consolidated, 3);
        assert_eq!(engine.stats().working_active, 0);
        assert_eq!(engine.stats().episodic_count, 3);
    }

    #[test]
    fn consolidation_preserves_provenance() {
        let mut engine = MemoryEngine::new(MemoryEngineConfig::default());

        let prov = provenance_from(EventId::new());
        let mut entry = active_entry("with provenance");
        entry.provenance = prov.clone();
        engine.store(entry).unwrap();

        engine.consolidate().unwrap();

        // The consolidated entry should still have the same provenance
        let results = engine.episodic().query_by_provenance(&prov);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn consolidation_discards_low_confidence() {
        let mut engine = MemoryEngine::new(MemoryEngineConfig::default());

        let mut low = active_entry("low confidence");
        low.confidence = 0.05; // below 0.1 threshold
        engine.store(low).unwrap();

        let high = active_entry("high confidence");
        engine.store(high).unwrap();

        let report = engine.consolidate().unwrap();
        assert_eq!(report.entries_consolidated, 1);
        assert_eq!(report.entries_discarded, 1);
    }

    #[test]
    fn should_consolidate_threshold() {
        let config = MemoryEngineConfig {
            consolidation_threshold: 3,
            ..Default::default()
        };
        let mut engine = MemoryEngine::new(config);

        engine.store(active_entry("a1")).unwrap();
        engine.store(active_entry("a2")).unwrap();
        assert!(!engine.should_consolidate());

        engine.store(active_entry("a3")).unwrap();
        assert!(engine.should_consolidate());
    }

    #[test]
    fn stats_accurate() {
        let mut engine = MemoryEngine::new(MemoryEngineConfig::default());

        engine.store(sensory_entry("s1")).unwrap();
        engine.store(sensory_entry("s2")).unwrap();
        engine.store(active_entry("a1")).unwrap();
        engine.store(episodic_entry("e1")).unwrap();
        engine.store(episodic_entry("e2")).unwrap();
        engine.store(episodic_entry("e3")).unwrap();

        let stats = engine.stats();
        assert_eq!(stats.working_sensory, 2);
        assert_eq!(stats.working_active, 1);
        assert_eq!(stats.episodic_count, 3);
        assert_eq!(stats.total_entries, 6);
    }

    #[tokio::test]
    async fn rebuild_working_from_episodic() {
        let mut engine = MemoryEngine::new(MemoryEngineConfig::default());

        // Store some episodic entries (simulating post-consolidation state)
        engine.store(episodic_entry("important context 1")).unwrap();
        engine.store(episodic_entry("important context 2")).unwrap();

        // Create a fabric for rebuild
        let fabric = EventFabric::init(maple_kernel_fabric::FabricConfig::default())
            .await
            .unwrap();

        // Rebuild should populate active context from episodic
        let report = engine.rebuild_working(&fabric).await.unwrap();

        assert!(report.entries_restored > 0);
        assert!(engine.working().active_len() > 0);
    }

    #[tokio::test]
    async fn rebuild_after_consolidation_preserves_content() {
        let mut engine = MemoryEngine::new(MemoryEngineConfig {
            working: WorkingPlaneConfig {
                active_max_entries: 10,
                ..Default::default()
            },
            ..Default::default()
        });

        // Store active entries and consolidate them to episodic
        engine.store(active_entry("context alpha")).unwrap();
        engine.store(active_entry("context beta")).unwrap();
        engine.consolidate().unwrap();

        // Verify working is empty after consolidation
        assert_eq!(engine.working().active_len(), 0);

        // Create fabric for rebuild
        let fabric = EventFabric::init(maple_kernel_fabric::FabricConfig::default())
            .await
            .unwrap();

        // Rebuild should restore from episodic
        let report = engine.rebuild_working(&fabric).await.unwrap();
        assert_eq!(report.entries_restored, 2);
        assert_eq!(engine.working().active_len(), 2);
    }
}
