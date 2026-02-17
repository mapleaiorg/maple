use serde::{Deserialize, Serialize};

/// Seed configuration for genesis boot.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SeedConfig {
    /// Minimum resonance threshold.
    pub resonance_min: f64,
    /// Maximum evolution steps before halting.
    pub max_evolution_steps: u64,
    /// Cooldown between evolution cycles (ms).
    pub evolution_cooldown_ms: u64,
    /// Auto-approve governance tiers up to this level.
    pub auto_approve_max_tier: u8,
    /// Maximum snapshot history.
    pub max_snapshots: usize,
    /// Enable demo mode (relaxed constraints).
    pub demo_mode: bool,
}

impl Default for SeedConfig {
    fn default() -> Self {
        Self {
            resonance_min: 0.6,
            max_evolution_steps: 1000,
            evolution_cooldown_ms: 5000,
            auto_approve_max_tier: 2,
            max_snapshots: 10,
            demo_mode: false,
        }
    }
}

impl SeedConfig {
    /// Demo configuration with relaxed constraints.
    pub fn demo() -> Self {
        Self {
            resonance_min: 0.3,
            max_evolution_steps: 100,
            evolution_cooldown_ms: 1000,
            auto_approve_max_tier: 3,
            max_snapshots: 5,
            demo_mode: true,
        }
    }

    /// Strict production configuration.
    pub fn production() -> Self {
        Self {
            resonance_min: 0.7,
            max_evolution_steps: 10000,
            evolution_cooldown_ms: 10000,
            auto_approve_max_tier: 1,
            max_snapshots: 50,
            demo_mode: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let c = SeedConfig::default();
        assert_eq!(c.resonance_min, 0.6);
        assert!(!c.demo_mode);
    }

    #[test]
    fn demo_config() {
        let c = SeedConfig::demo();
        assert!(c.demo_mode);
        assert!(c.resonance_min < SeedConfig::default().resonance_min);
    }

    #[test]
    fn production_config() {
        let c = SeedConfig::production();
        assert!(c.resonance_min > SeedConfig::default().resonance_min);
        assert!(!c.demo_mode);
    }

    #[test]
    fn config_serde() {
        let c = SeedConfig::default();
        let json = serde_json::to_string(&c).unwrap();
        let restored: SeedConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.resonance_min, c.resonance_min);
    }
}
