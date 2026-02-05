//! MapleVerse configuration types
//!
//! Configuration for the MapleVerse world with **CRITICAL** enforcement
//! of the no-human-profiles invariant.

use crate::errors::{MapleVerseError, MapleVerseResult};
use serde::{Deserialize, Serialize};

/// Configuration for a MapleVerse world instance
///
/// # CRITICAL: Human Profile Rejection
///
/// The `human_profiles_allowed` field is ALWAYS `false` at runtime.
/// This is not configurable - it's a fundamental invariant of MapleVerse.
/// Any attempt to create a human profile will result in a runtime error.
///
/// The `unsafe-human-profiles` feature flag exists ONLY for testing
/// infrastructure and must NEVER be enabled in production.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapleVerseConfig {
    /// Name of this MapleVerse instance
    pub world_name: String,

    /// **CRITICAL**: Human profiles are NOT allowed
    ///
    /// This field is ALWAYS false at runtime. It exists to make the
    /// invariant explicit and auditable. Any attempt to set this to
    /// true without the `unsafe-human-profiles` feature will fail.
    #[serde(default)]
    human_profiles_allowed: bool,

    /// World simulation parameters
    pub world_parameters: WorldParameters,

    /// Economy configuration
    pub economy_config: EconomyConfig,

    /// Attention system configuration
    pub attention_config: AttentionConfig,

    /// Reputation system configuration
    pub reputation_config: ReputationConfig,

    /// Region configuration
    pub region_config: RegionConfig,
}

impl Default for MapleVerseConfig {
    fn default() -> Self {
        Self {
            world_name: "MapleVerse".to_string(),
            human_profiles_allowed: false, // ALWAYS false
            world_parameters: WorldParameters::default(),
            economy_config: EconomyConfig::default(),
            attention_config: AttentionConfig::default(),
            reputation_config: ReputationConfig::default(),
            region_config: RegionConfig::default(),
        }
    }
}

impl MapleVerseConfig {
    /// Create a new MapleVerse configuration
    ///
    /// Human profiles are NEVER allowed. This is enforced at construction.
    pub fn new(world_name: impl Into<String>) -> Self {
        Self {
            world_name: world_name.into(),
            human_profiles_allowed: false, // ALWAYS false
            ..Default::default()
        }
    }

    /// Check if human profiles are allowed
    ///
    /// This ALWAYS returns false unless the `unsafe-human-profiles` feature
    /// is enabled (which should NEVER happen in production).
    #[inline]
    pub fn human_profiles_allowed(&self) -> bool {
        #[cfg(feature = "unsafe-human-profiles")]
        {
            self.human_profiles_allowed
        }
        #[cfg(not(feature = "unsafe-human-profiles"))]
        {
            false // ALWAYS false in production
        }
    }

    /// Attempt to enable human profiles
    ///
    /// This will FAIL unless the `unsafe-human-profiles` feature is enabled.
    /// This feature should NEVER be used in production.
    #[cfg(feature = "unsafe-human-profiles")]
    pub fn enable_human_profiles(&mut self) {
        self.human_profiles_allowed = true;
    }

    /// Attempt to enable human profiles (ALWAYS fails without feature)
    #[cfg(not(feature = "unsafe-human-profiles"))]
    pub fn enable_human_profiles(&mut self) -> MapleVerseResult<()> {
        Err(MapleVerseError::HumanProfilesFeatureDisabled)
    }

    /// Validate the configuration
    pub fn validate(&self) -> MapleVerseResult<()> {
        // CRITICAL: Ensure human profiles are not allowed without feature
        #[cfg(not(feature = "unsafe-human-profiles"))]
        if self.human_profiles_allowed {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "human_profiles_allowed cannot be true without unsafe-human-profiles feature".to_string(),
            });
        }

        // Validate world parameters
        self.world_parameters.validate()?;

        // Validate economy config
        self.economy_config.validate()?;

        // Validate attention config
        self.attention_config.validate()?;

        // Validate reputation config
        self.reputation_config.validate()?;

        // Validate region config
        self.region_config.validate()?;

        Ok(())
    }

    /// Create configuration for a small test world
    pub fn test_world() -> Self {
        Self {
            world_name: "TestWorld".to_string(),
            human_profiles_allowed: false,
            world_parameters: WorldParameters {
                max_entities: 1000,
                epoch_duration_secs: 60,
                tick_rate_hz: 10.0,
            },
            economy_config: EconomyConfig {
                initial_maple_balance: 100,
                maple_transfer_fee_bps: 0, // No fees in test
                max_maple_supply: 1_000_000,
            },
            attention_config: AttentionConfig {
                base_attention_per_epoch: 100,
                attention_regeneration_rate: 1.0,
                max_attention_carryover: 50,
                attention_tradeable: true,
            },
            reputation_config: ReputationConfig {
                initial_reputation: 0,
                receipt_reputation_weight: 1.0,
                reputation_decay_per_epoch: 0.0,
                min_reputation: -1000,
                max_reputation: 1000,
            },
            region_config: RegionConfig {
                default_region_capacity: 100,
                migration_cooldown_epochs: 1,
                allow_cross_region_interaction: true,
            },
        }
    }

    /// Create configuration for a large production world
    pub fn production_world(name: impl Into<String>) -> Self {
        Self {
            world_name: name.into(),
            human_profiles_allowed: false, // ALWAYS false
            world_parameters: WorldParameters {
                max_entities: 100_000_000, // 100M agents
                epoch_duration_secs: 3600, // 1 hour epochs
                tick_rate_hz: 100.0,
            },
            economy_config: EconomyConfig {
                initial_maple_balance: 1000,
                maple_transfer_fee_bps: 10, // 0.1% transfer fee
                max_maple_supply: 10_000_000_000_000, // 10T max supply
            },
            attention_config: AttentionConfig {
                base_attention_per_epoch: 1000,
                attention_regeneration_rate: 0.5,
                max_attention_carryover: 500,
                attention_tradeable: true,
            },
            reputation_config: ReputationConfig {
                initial_reputation: 0,
                receipt_reputation_weight: 1.0,
                reputation_decay_per_epoch: 0.01, // 1% decay
                min_reputation: -10000,
                max_reputation: 10000,
            },
            region_config: RegionConfig {
                default_region_capacity: 1_000_000,
                migration_cooldown_epochs: 10,
                allow_cross_region_interaction: false,
            },
        }
    }
}

/// World simulation parameters
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldParameters {
    /// Maximum number of entities in the world
    pub max_entities: u64,

    /// Duration of each epoch in seconds
    pub epoch_duration_secs: u64,

    /// Simulation tick rate in Hz
    pub tick_rate_hz: f64,
}

impl Default for WorldParameters {
    fn default() -> Self {
        Self {
            max_entities: 1_000_000,
            epoch_duration_secs: 3600, // 1 hour
            tick_rate_hz: 10.0,
        }
    }
}

impl WorldParameters {
    /// Validate world parameters
    pub fn validate(&self) -> MapleVerseResult<()> {
        if self.max_entities == 0 {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "max_entities must be greater than 0".to_string(),
            });
        }

        if self.epoch_duration_secs == 0 {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "epoch_duration_secs must be greater than 0".to_string(),
            });
        }

        if self.tick_rate_hz <= 0.0 {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "tick_rate_hz must be positive".to_string(),
            });
        }

        Ok(())
    }
}

/// Economy configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EconomyConfig {
    /// Initial MAPLE balance for new entities
    pub initial_maple_balance: u64,

    /// Transfer fee in basis points (1 bps = 0.01%)
    pub maple_transfer_fee_bps: u16,

    /// Maximum total MAPLE supply
    pub max_maple_supply: u64,
}

impl Default for EconomyConfig {
    fn default() -> Self {
        Self {
            initial_maple_balance: 1000,
            maple_transfer_fee_bps: 10,
            max_maple_supply: 10_000_000_000_000,
        }
    }
}

impl EconomyConfig {
    /// Validate economy configuration
    pub fn validate(&self) -> MapleVerseResult<()> {
        if self.maple_transfer_fee_bps > 10000 {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "maple_transfer_fee_bps cannot exceed 10000 (100%)".to_string(),
            });
        }

        if self.initial_maple_balance > self.max_maple_supply {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "initial_maple_balance cannot exceed max_maple_supply".to_string(),
            });
        }

        Ok(())
    }
}

/// Attention system configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttentionConfig {
    /// Base attention units regenerated per epoch
    pub base_attention_per_epoch: u64,

    /// Rate at which attention regenerates (0.0 to 1.0)
    pub attention_regeneration_rate: f64,

    /// Maximum attention that can carry over to next epoch
    pub max_attention_carryover: u64,

    /// Whether attention can be traded between entities
    pub attention_tradeable: bool,
}

impl Default for AttentionConfig {
    fn default() -> Self {
        Self {
            base_attention_per_epoch: 1000,
            attention_regeneration_rate: 0.5,
            max_attention_carryover: 500,
            attention_tradeable: true,
        }
    }
}

impl AttentionConfig {
    /// Validate attention configuration
    pub fn validate(&self) -> MapleVerseResult<()> {
        if self.attention_regeneration_rate < 0.0 || self.attention_regeneration_rate > 1.0 {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "attention_regeneration_rate must be between 0.0 and 1.0".to_string(),
            });
        }

        if self.max_attention_carryover > self.base_attention_per_epoch {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "max_attention_carryover cannot exceed base_attention_per_epoch"
                    .to_string(),
            });
        }

        Ok(())
    }
}

/// Reputation system configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReputationConfig {
    /// Initial reputation score for new entities
    pub initial_reputation: i64,

    /// Weight applied to receipt-based reputation
    pub receipt_reputation_weight: f64,

    /// Reputation decay per epoch (0.0 to 1.0)
    pub reputation_decay_per_epoch: f64,

    /// Minimum reputation score
    pub min_reputation: i64,

    /// Maximum reputation score
    pub max_reputation: i64,
}

impl Default for ReputationConfig {
    fn default() -> Self {
        Self {
            initial_reputation: 0,
            receipt_reputation_weight: 1.0,
            reputation_decay_per_epoch: 0.01,
            min_reputation: -10000,
            max_reputation: 10000,
        }
    }
}

impl ReputationConfig {
    /// Validate reputation configuration
    pub fn validate(&self) -> MapleVerseResult<()> {
        if self.receipt_reputation_weight <= 0.0 {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "receipt_reputation_weight must be positive".to_string(),
            });
        }

        if self.reputation_decay_per_epoch < 0.0 || self.reputation_decay_per_epoch > 1.0 {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "reputation_decay_per_epoch must be between 0.0 and 1.0".to_string(),
            });
        }

        if self.min_reputation >= self.max_reputation {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "min_reputation must be less than max_reputation".to_string(),
            });
        }

        if self.initial_reputation < self.min_reputation
            || self.initial_reputation > self.max_reputation
        {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "initial_reputation must be within min/max bounds".to_string(),
            });
        }

        Ok(())
    }
}

/// Region configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegionConfig {
    /// Default capacity for new regions
    pub default_region_capacity: u64,

    /// Cooldown epochs between migrations
    pub migration_cooldown_epochs: u64,

    /// Whether entities can interact across regions
    pub allow_cross_region_interaction: bool,
}

impl Default for RegionConfig {
    fn default() -> Self {
        Self {
            default_region_capacity: 1_000_000,
            migration_cooldown_epochs: 10,
            allow_cross_region_interaction: false,
        }
    }
}

impl RegionConfig {
    /// Validate region configuration
    pub fn validate(&self) -> MapleVerseResult<()> {
        if self.default_region_capacity == 0 {
            return Err(MapleVerseError::InvalidConfiguration {
                reason: "default_region_capacity must be greater than 0".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_no_humans() {
        let config = MapleVerseConfig::default();
        assert!(!config.human_profiles_allowed());
    }

    #[test]
    fn test_new_config_has_no_humans() {
        let config = MapleVerseConfig::new("TestWorld");
        assert!(!config.human_profiles_allowed());
    }

    #[test]
    fn test_config_validation_passes() {
        let config = MapleVerseConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_test_world_config() {
        let config = MapleVerseConfig::test_world();
        assert!(!config.human_profiles_allowed());
        assert!(config.validate().is_ok());
        assert_eq!(config.world_parameters.max_entities, 1000);
    }

    #[test]
    fn test_production_world_config() {
        let config = MapleVerseConfig::production_world("ProdWorld");
        assert!(!config.human_profiles_allowed());
        assert!(config.validate().is_ok());
        assert_eq!(config.world_parameters.max_entities, 100_000_000);
    }

    #[test]
    fn test_invalid_world_parameters() {
        let mut config = MapleVerseConfig::default();
        config.world_parameters.max_entities = 0;
        assert!(config.validate().is_err());

        config.world_parameters.max_entities = 1000;
        config.world_parameters.epoch_duration_secs = 0;
        assert!(config.validate().is_err());

        config.world_parameters.epoch_duration_secs = 3600;
        config.world_parameters.tick_rate_hz = -1.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_economy_config() {
        let mut config = MapleVerseConfig::default();
        config.economy_config.maple_transfer_fee_bps = 20000; // > 100%
        assert!(config.validate().is_err());

        config.economy_config.maple_transfer_fee_bps = 10;
        config.economy_config.initial_maple_balance = u64::MAX;
        config.economy_config.max_maple_supply = 1000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_attention_config() {
        let mut config = MapleVerseConfig::default();
        config.attention_config.attention_regeneration_rate = 1.5;
        assert!(config.validate().is_err());

        config.attention_config.attention_regeneration_rate = 0.5;
        config.attention_config.max_attention_carryover = 10000;
        config.attention_config.base_attention_per_epoch = 100;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_reputation_config() {
        let mut config = MapleVerseConfig::default();
        config.reputation_config.receipt_reputation_weight = 0.0;
        assert!(config.validate().is_err());

        config.reputation_config.receipt_reputation_weight = 1.0;
        config.reputation_config.reputation_decay_per_epoch = 2.0;
        assert!(config.validate().is_err());

        config.reputation_config.reputation_decay_per_epoch = 0.01;
        config.reputation_config.min_reputation = 100;
        config.reputation_config.max_reputation = 50;
        assert!(config.validate().is_err());

        config.reputation_config.min_reputation = -100;
        config.reputation_config.max_reputation = 100;
        config.reputation_config.initial_reputation = 500;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_invalid_region_config() {
        let mut config = MapleVerseConfig::default();
        config.region_config.default_region_capacity = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    #[cfg(not(feature = "unsafe-human-profiles"))]
    fn test_enable_humans_fails_without_feature() {
        let mut config = MapleVerseConfig::default();
        let result = config.enable_human_profiles();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            MapleVerseError::HumanProfilesFeatureDisabled
        ));
    }

    #[test]
    fn test_serialization() {
        let config = MapleVerseConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: MapleVerseConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.world_name, deserialized.world_name);
        assert!(!deserialized.human_profiles_allowed());
    }
}
