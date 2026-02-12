use tracing::debug;

use crate::types::{
    ChannelLiquidity, LiquidityField, SettlementChannel, SettlementNetwork, StressLevel,
};

/// ERX — Liquidity Field Operator.
///
/// Models liquidity in the settlement network using a reaction-diffusion approach.
/// Each settlement channel has a liquidity ratio (current / capacity), and the
/// network's overall health is computed from channel utilization patterns.
///
/// When liquidity drops below thresholds, stress levels escalate and circuit
/// breakers may trigger.
pub struct LiquidityFieldOperator {
    /// Stress thresholds
    config: LiquidityConfig,
}

/// Configuration for the liquidity field operator.
#[derive(Clone, Debug)]
pub struct LiquidityConfig {
    /// Below this network score → Elevated stress
    pub elevated_threshold: f64,
    /// Below this network score → High stress
    pub high_threshold: f64,
    /// Below this network score → Critical stress (circuit breaker)
    pub critical_threshold: f64,
    /// Minimum channel ratio before it's considered stressed
    pub channel_stress_threshold: f64,
}

impl Default for LiquidityConfig {
    fn default() -> Self {
        Self {
            elevated_threshold: 0.6,
            high_threshold: 0.3,
            critical_threshold: 0.15,
            channel_stress_threshold: 0.2,
        }
    }
}

impl LiquidityFieldOperator {
    /// Create a new liquidity field operator with default config.
    pub fn new() -> Self {
        Self {
            config: LiquidityConfig::default(),
        }
    }

    /// Create with custom configuration.
    pub fn with_config(config: LiquidityConfig) -> Self {
        Self { config }
    }

    /// Compute the liquidity field for a settlement network.
    ///
    /// Analyzes each channel's liquidity ratio and computes an overall
    /// network health score. Uses a reaction-diffusion inspired model
    /// where channel stress propagates to neighboring channels.
    pub fn compute_field(&self, network: &SettlementNetwork) -> LiquidityField {
        if network.channels.is_empty() {
            return LiquidityField {
                channel_scores: vec![],
                network_score: 1.0,
                circuit_breaker_triggered: false,
                stress_level: StressLevel::Normal,
            };
        }

        // Compute per-channel liquidity ratios
        let channel_scores: Vec<ChannelLiquidity> = network
            .channels
            .iter()
            .map(|ch| self.compute_channel_liquidity(ch))
            .collect();

        // Network score: weighted average of channel ratios,
        // penalized by stress concentration (reaction-diffusion effect)
        let network_score = self.compute_network_score(&channel_scores);

        // Determine stress level from network score
        let stress_level = self.classify_stress(network_score);

        // Circuit breaker triggers at Critical stress
        let circuit_breaker_triggered = stress_level == StressLevel::Critical;

        debug!(
            channels = channel_scores.len(),
            network_score = format!("{:.4}", network_score).as_str(),
            stress = ?stress_level,
            circuit_breaker = circuit_breaker_triggered,
            "Liquidity field computed"
        );

        LiquidityField {
            channel_scores,
            network_score,
            circuit_breaker_triggered,
            stress_level,
        }
    }

    /// Compute liquidity for a single channel.
    fn compute_channel_liquidity(&self, channel: &SettlementChannel) -> ChannelLiquidity {
        let ratio = if channel.capacity_minor > 0 {
            channel.liquidity_minor as f64 / channel.capacity_minor as f64
        } else {
            0.0
        };

        ChannelLiquidity {
            from: channel.from.clone(),
            to: channel.to.clone(),
            asset: channel.asset.clone(),
            ratio: ratio.clamp(0.0, 1.0),
        }
    }

    /// Compute the network-level liquidity score.
    ///
    /// Uses a reaction-diffusion inspired approach:
    /// - Base: mean of channel ratios
    /// - Penalty: variance of channel ratios (stress concentration)
    /// - High variance means some channels are stressed while others are fine,
    ///   indicating liquidity fragmentation
    fn compute_network_score(&self, channels: &[ChannelLiquidity]) -> f64 {
        if channels.is_empty() {
            return 1.0;
        }

        let n = channels.len() as f64;
        let mean: f64 = channels.iter().map(|c| c.ratio).sum::<f64>() / n;

        // Variance (stress concentration penalty)
        let variance: f64 = channels
            .iter()
            .map(|c| {
                let diff = c.ratio - mean;
                diff * diff
            })
            .sum::<f64>()
            / n;

        // Count stressed channels
        let stressed_count = channels
            .iter()
            .filter(|c| c.ratio < self.config.channel_stress_threshold)
            .count() as f64;

        // Network score: mean - variance penalty - stressed channel penalty
        let stress_penalty = (stressed_count / n) * 0.3;
        let score = mean - variance.sqrt() * 0.5 - stress_penalty;

        score.clamp(0.0, 1.0)
    }

    /// Classify stress level from network score.
    fn classify_stress(&self, network_score: f64) -> StressLevel {
        if network_score < self.config.critical_threshold {
            StressLevel::Critical
        } else if network_score < self.config.high_threshold {
            StressLevel::High
        } else if network_score < self.config.elevated_threshold {
            StressLevel::Elevated
        } else {
            StressLevel::Normal
        }
    }
}

impl Default for LiquidityFieldOperator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AssetId;
    use maple_mwl_types::{IdentityMaterial, WorldlineId};

    fn wid_a() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([1u8; 32]))
    }

    fn wid_b() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([2u8; 32]))
    }

    fn wid_c() -> WorldlineId {
        WorldlineId::derive(&IdentityMaterial::GenesisHash([3u8; 32]))
    }

    fn usd() -> AssetId {
        AssetId::new("USD")
    }

    #[test]
    fn empty_network_is_healthy() {
        let erx = LiquidityFieldOperator::new();
        let network = SettlementNetwork {
            participants: vec![],
            channels: vec![],
        };

        let field = erx.compute_field(&network);
        assert!((field.network_score - 1.0).abs() < f64::EPSILON);
        assert_eq!(field.stress_level, StressLevel::Normal);
        assert!(!field.circuit_breaker_triggered);
    }

    #[test]
    fn healthy_network_normal_stress() {
        let erx = LiquidityFieldOperator::new();
        let network = SettlementNetwork {
            participants: vec![wid_a(), wid_b()],
            channels: vec![SettlementChannel {
                from: wid_a(),
                to: wid_b(),
                asset: usd(),
                liquidity_minor: 800_000,
                capacity_minor: 1_000_000,
            }],
        };

        let field = erx.compute_field(&network);
        assert!(field.network_score > 0.6);
        assert_eq!(field.stress_level, StressLevel::Normal);
        assert!(!field.circuit_breaker_triggered);
    }

    #[test]
    fn depleted_network_triggers_circuit_breaker() {
        let erx = LiquidityFieldOperator::new();
        let network = SettlementNetwork {
            participants: vec![wid_a(), wid_b()],
            channels: vec![SettlementChannel {
                from: wid_a(),
                to: wid_b(),
                asset: usd(),
                liquidity_minor: 50_000,
                capacity_minor: 1_000_000, // 5% liquidity
            }],
        };

        let field = erx.compute_field(&network);
        assert!(field.network_score < 0.15);
        assert_eq!(field.stress_level, StressLevel::Critical);
        assert!(field.circuit_breaker_triggered);
    }

    #[test]
    fn mixed_liquidity_elevated_stress() {
        let erx = LiquidityFieldOperator::new();
        let network = SettlementNetwork {
            participants: vec![wid_a(), wid_b(), wid_c()],
            channels: vec![
                SettlementChannel {
                    from: wid_a(),
                    to: wid_b(),
                    asset: usd(),
                    liquidity_minor: 900_000,
                    capacity_minor: 1_000_000, // Healthy
                },
                SettlementChannel {
                    from: wid_b(),
                    to: wid_c(),
                    asset: usd(),
                    liquidity_minor: 100_000,
                    capacity_minor: 1_000_000, // Stressed
                },
            ],
        };

        let field = erx.compute_field(&network);
        // Mix of healthy and stressed → elevated or higher
        assert!(field.stress_level >= StressLevel::Elevated);
    }

    #[test]
    fn channel_liquidity_ratio_computed() {
        let erx = LiquidityFieldOperator::new();
        let network = SettlementNetwork {
            participants: vec![wid_a(), wid_b()],
            channels: vec![SettlementChannel {
                from: wid_a(),
                to: wid_b(),
                asset: usd(),
                liquidity_minor: 500_000,
                capacity_minor: 1_000_000,
            }],
        };

        let field = erx.compute_field(&network);
        assert_eq!(field.channel_scores.len(), 1);
        assert!((field.channel_scores[0].ratio - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn zero_capacity_channel_ratio_is_zero() {
        let erx = LiquidityFieldOperator::new();
        let network = SettlementNetwork {
            participants: vec![wid_a(), wid_b()],
            channels: vec![SettlementChannel {
                from: wid_a(),
                to: wid_b(),
                asset: usd(),
                liquidity_minor: 0,
                capacity_minor: 0,
            }],
        };

        let field = erx.compute_field(&network);
        assert!((field.channel_scores[0].ratio).abs() < f64::EPSILON);
    }

    #[test]
    fn custom_config_thresholds() {
        let config = LiquidityConfig {
            elevated_threshold: 0.8,
            high_threshold: 0.5,
            critical_threshold: 0.2,
            channel_stress_threshold: 0.3,
        };
        let erx = LiquidityFieldOperator::with_config(config);

        let network = SettlementNetwork {
            participants: vec![wid_a(), wid_b()],
            channels: vec![SettlementChannel {
                from: wid_a(),
                to: wid_b(),
                asset: usd(),
                liquidity_minor: 600_000,
                capacity_minor: 1_000_000,
            }],
        };

        let field = erx.compute_field(&network);
        // 0.6 ratio → below 0.8 elevated threshold
        assert!(field.stress_level >= StressLevel::Elevated);
    }

    #[test]
    fn stress_level_ordering() {
        assert!(StressLevel::Normal < StressLevel::Elevated);
        assert!(StressLevel::Elevated < StressLevel::High);
        assert!(StressLevel::High < StressLevel::Critical);
    }
}
