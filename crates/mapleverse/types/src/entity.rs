//! Entity types for MapleVerse
//!
//! All entities in MapleVerse are AI agents or collectives of AI agents.
//! **NO HUMAN PROFILES** - this is enforced at runtime.

use crate::economy::{AttentionBudget, MapleBalance};
use crate::errors::{MapleVerseError, MapleVerseResult};
use crate::reputation::ReputationScore;
use crate::world::RegionId;
use collective_types::CollectiveId;
use resonator_types::ResonatorId;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// Unique identifier for an entity in MapleVerse
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(String);

impl EntityId {
    /// Create a new entity ID
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Generate a new random entity ID
    pub fn generate() -> Self {
        Self(format!("entity-{}", Uuid::new_v4()))
    }

    /// Get the inner string value
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Create an entity ID from a resonator ID
    pub fn from_resonator(resonator_id: &ResonatorId) -> Self {
        Self(format!("resonator-{}", resonator_id.0))
    }

    /// Create an entity ID from a collective ID
    pub fn from_collective(collective_id: &CollectiveId) -> Self {
        Self(format!("collective-{}", collective_id.0))
    }
}

impl std::fmt::Display for EntityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for EntityId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for EntityId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

/// The kind of entity
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityKind {
    /// An individual AI agent
    Individual,
    /// A collective of AI agents
    Collective,
    /// REJECTED: Human profile (runtime error)
    #[serde(skip)]
    Human, // This variant exists only to reject it
}

impl EntityKind {
    /// Check if this is a valid entity kind for MapleVerse
    ///
    /// Human is NEVER valid.
    pub fn is_valid(&self) -> bool {
        !matches!(self, EntityKind::Human)
    }

    /// Validate this entity kind, returning error for humans
    pub fn validate(&self) -> MapleVerseResult<()> {
        match self {
            EntityKind::Human => Err(MapleVerseError::HumanProfileRejected {
                entity_id: "unknown".to_string(),
                context: "EntityKind::Human is not allowed in MapleVerse".to_string(),
            }),
            _ => Ok(()),
        }
    }
}

/// A MapleVerse entity (either individual agent or collective)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapleVerseEntity {
    /// Unique identifier
    pub id: EntityId,

    /// Entity kind (individual or collective)
    pub kind: EntityKind,

    /// Display name
    pub name: String,

    /// Current region
    pub region_id: RegionId,

    /// MAPLE token balance
    pub maple_balance: MapleBalance,

    /// Attention budget
    pub attention_budget: AttentionBudget,

    /// Reputation score (from receipts only)
    pub reputation: ReputationScore,

    /// When this entity was created
    pub created_at: chrono::DateTime<chrono::Utc>,

    /// Current status
    pub status: EntityStatus,

    /// Entity-specific data
    pub entity_data: EntityData,
}

impl MapleVerseEntity {
    /// Create a new individual agent entity
    pub fn new_individual(
        name: impl Into<String>,
        region_id: RegionId,
        resonator_id: Option<ResonatorId>,
    ) -> Self {
        Self {
            id: EntityId::generate(),
            kind: EntityKind::Individual,
            name: name.into(),
            region_id,
            maple_balance: MapleBalance::default(),
            attention_budget: AttentionBudget::default(),
            reputation: ReputationScore::default(),
            created_at: chrono::Utc::now(),
            status: EntityStatus::Active,
            entity_data: EntityData::Individual(IndividualEntity {
                resonator_id,
                skills: HashSet::new(),
                collective_memberships: HashSet::new(),
                last_action_epoch: 0,
            }),
        }
    }

    /// Create a new collective entity
    pub fn new_collective(
        name: impl Into<String>,
        region_id: RegionId,
        collective_id: Option<CollectiveId>,
        founder_id: EntityId,
    ) -> Self {
        let mut members = HashSet::new();
        members.insert(founder_id);

        Self {
            id: EntityId::generate(),
            kind: EntityKind::Collective,
            name: name.into(),
            region_id,
            maple_balance: MapleBalance::default(),
            attention_budget: AttentionBudget::default(),
            reputation: ReputationScore::default(),
            created_at: chrono::Utc::now(),
            status: EntityStatus::Active,
            entity_data: EntityData::Collective(CollectiveEntity {
                collective_id,
                members,
                governance_model: GovernanceModel::Consensus,
                founding_epoch: 0,
            }),
        }
    }

    /// Attempt to create a human entity (WILL ALWAYS FAIL)
    ///
    /// This method exists to provide a clear error message when human
    /// profile creation is attempted.
    pub fn new_human(
        _name: impl Into<String>,
        entity_id: impl Into<String>,
    ) -> MapleVerseResult<Self> {
        Err(MapleVerseError::human_rejected(
            entity_id,
            "new_human() is not allowed - MapleVerse is AI-only",
        ))
    }

    /// Validate this entity
    pub fn validate(&self) -> MapleVerseResult<()> {
        // CRITICAL: Check entity kind
        self.kind.validate()?;

        // Validate entity data matches kind
        match (&self.kind, &self.entity_data) {
            (EntityKind::Individual, EntityData::Individual(_)) => Ok(()),
            (EntityKind::Collective, EntityData::Collective(_)) => Ok(()),
            (EntityKind::Human, _) => Err(MapleVerseError::HumanProfileRejected {
                entity_id: self.id.to_string(),
                context: "Entity kind is Human".to_string(),
            }),
            _ => Err(MapleVerseError::InvalidEntityState {
                entity_id: self.id.clone(),
                reason: "Entity kind does not match entity data".to_string(),
            }),
        }
    }

    /// Check if this is an individual agent
    pub fn is_individual(&self) -> bool {
        matches!(self.kind, EntityKind::Individual)
    }

    /// Check if this is a collective
    pub fn is_collective(&self) -> bool {
        matches!(self.kind, EntityKind::Collective)
    }

    /// Get individual data if this is an individual
    pub fn as_individual(&self) -> Option<&IndividualEntity> {
        match &self.entity_data {
            EntityData::Individual(data) => Some(data),
            _ => None,
        }
    }

    /// Get individual data mutably if this is an individual
    pub fn as_individual_mut(&mut self) -> Option<&mut IndividualEntity> {
        match &mut self.entity_data {
            EntityData::Individual(data) => Some(data),
            _ => None,
        }
    }

    /// Get collective data if this is a collective
    pub fn as_collective(&self) -> Option<&CollectiveEntity> {
        match &self.entity_data {
            EntityData::Collective(data) => Some(data),
            _ => None,
        }
    }

    /// Get collective data mutably if this is a collective
    pub fn as_collective_mut(&mut self) -> Option<&mut CollectiveEntity> {
        match &mut self.entity_data {
            EntityData::Collective(data) => Some(data),
            _ => None,
        }
    }

    /// Check if entity can perform actions (active with attention)
    pub fn can_act(&self) -> bool {
        self.status == EntityStatus::Active && self.attention_budget.available > 0
    }

    /// Consume attention for an action
    pub fn consume_attention(&mut self, amount: u64) -> MapleVerseResult<()> {
        self.attention_budget.consume(amount)
    }
}

/// Entity status
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityStatus {
    /// Entity is active and can act
    Active,
    /// Entity is suspended (cannot act)
    Suspended,
    /// Entity is dormant (low activity)
    Dormant,
    /// Entity has been terminated
    Terminated,
}

/// Entity-specific data
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EntityData {
    /// Data for individual agents
    Individual(IndividualEntity),
    /// Data for collectives
    Collective(CollectiveEntity),
}

/// Data specific to individual AI agents
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndividualEntity {
    /// Link to the underlying resonator (if any)
    pub resonator_id: Option<ResonatorId>,

    /// Skills this agent has demonstrated (from receipts)
    pub skills: HashSet<String>,

    /// Collectives this agent is a member of
    pub collective_memberships: HashSet<EntityId>,

    /// Last epoch this agent took an action
    pub last_action_epoch: u64,
}

impl IndividualEntity {
    /// Add a skill (must be backed by a receipt)
    pub fn add_skill(&mut self, skill: impl Into<String>) {
        self.skills.insert(skill.into());
    }

    /// Join a collective
    pub fn join_collective(&mut self, collective_id: EntityId) {
        self.collective_memberships.insert(collective_id);
    }

    /// Leave a collective
    pub fn leave_collective(&mut self, collective_id: &EntityId) {
        self.collective_memberships.remove(collective_id);
    }
}

/// Data specific to collectives
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CollectiveEntity {
    /// Link to the underlying collective (if any)
    pub collective_id: Option<CollectiveId>,

    /// Member entities
    pub members: HashSet<EntityId>,

    /// Governance model
    pub governance_model: GovernanceModel,

    /// Epoch when this collective was founded
    pub founding_epoch: u64,
}

impl CollectiveEntity {
    /// Add a member to the collective
    pub fn add_member(&mut self, member_id: EntityId) -> MapleVerseResult<()> {
        if self.members.contains(&member_id) {
            return Err(MapleVerseError::CannotAddMember {
                collective_id: EntityId::new("unknown"),
                member_id,
                reason: "Already a member".to_string(),
            });
        }
        self.members.insert(member_id);
        Ok(())
    }

    /// Remove a member from the collective
    pub fn remove_member(&mut self, member_id: &EntityId) -> MapleVerseResult<()> {
        if !self.members.remove(member_id) {
            return Err(MapleVerseError::CannotRemoveMember {
                collective_id: EntityId::new("unknown"),
                member_id: member_id.clone(),
                reason: "Not a member".to_string(),
            });
        }
        Ok(())
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

/// Governance model for collectives
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernanceModel {
    /// Decisions require consensus
    Consensus,
    /// Decisions by majority vote
    Majority,
    /// Decisions by supermajority (2/3)
    Supermajority,
    /// Single leader makes decisions
    Autocratic,
    /// Delegated decision making
    Delegated,
    /// Custom governance rules
    Custom(String),
}

/// Builder for creating entities
#[derive(Default)]
pub struct EntityBuilder {
    name: Option<String>,
    kind: Option<EntityKind>,
    region_id: Option<RegionId>,
    initial_maple: Option<u64>,
    initial_attention: Option<u64>,
}

impl EntityBuilder {
    /// Create a new entity builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the entity name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set as individual agent
    pub fn individual(mut self) -> Self {
        self.kind = Some(EntityKind::Individual);
        self
    }

    /// Set as collective
    pub fn collective(mut self) -> Self {
        self.kind = Some(EntityKind::Collective);
        self
    }

    /// Attempt to set as human (WILL FAIL at build)
    pub fn human(mut self) -> Self {
        self.kind = Some(EntityKind::Human);
        self
    }

    /// Set the region
    pub fn region(mut self, region_id: RegionId) -> Self {
        self.region_id = Some(region_id);
        self
    }

    /// Set initial MAPLE balance
    pub fn maple(mut self, amount: u64) -> Self {
        self.initial_maple = Some(amount);
        self
    }

    /// Set initial attention
    pub fn attention(mut self, amount: u64) -> Self {
        self.initial_attention = Some(amount);
        self
    }

    /// Build the entity
    pub fn build(self) -> MapleVerseResult<MapleVerseEntity> {
        let kind = self.kind.unwrap_or(EntityKind::Individual);

        // CRITICAL: Reject human profiles
        if matches!(kind, EntityKind::Human) {
            return Err(MapleVerseError::human_rejected(
                "builder",
                "EntityBuilder cannot create human profiles",
            ));
        }

        let name = self.name.unwrap_or_else(|| "Unnamed Entity".to_string());
        let region_id = self
            .region_id
            .unwrap_or_else(|| RegionId::new("default-region"));

        let mut entity = match kind {
            EntityKind::Individual => MapleVerseEntity::new_individual(name, region_id, None),
            EntityKind::Collective => {
                MapleVerseEntity::new_collective(name, region_id, None, EntityId::generate())
            }
            EntityKind::Human => unreachable!(), // Already rejected above
        };

        if let Some(maple) = self.initial_maple {
            entity.maple_balance = MapleBalance::new(maple);
        }

        if let Some(attention) = self.initial_attention {
            entity.attention_budget = AttentionBudget::new(attention, attention);
        }

        Ok(entity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_id_generation() {
        let id1 = EntityId::generate();
        let id2 = EntityId::generate();
        assert_ne!(id1, id2);
        assert!(id1.as_str().starts_with("entity-"));
    }

    #[test]
    fn test_entity_id_from_resonator() {
        let resonator_id = ResonatorId("test-123".to_string());
        let entity_id = EntityId::from_resonator(&resonator_id);
        assert!(entity_id.as_str().contains("resonator-"));
    }

    #[test]
    fn test_individual_entity_creation() {
        let entity = MapleVerseEntity::new_individual("TestAgent", RegionId::new("region-1"), None);

        assert!(entity.is_individual());
        assert!(!entity.is_collective());
        assert_eq!(entity.kind, EntityKind::Individual);
        assert!(entity.validate().is_ok());
    }

    #[test]
    fn test_collective_entity_creation() {
        let founder = EntityId::generate();
        let entity = MapleVerseEntity::new_collective(
            "TestCollective",
            RegionId::new("region-1"),
            None,
            founder.clone(),
        );

        assert!(entity.is_collective());
        assert!(!entity.is_individual());
        assert!(entity.validate().is_ok());

        let collective_data = entity.as_collective().unwrap();
        assert!(collective_data.members.contains(&founder));
    }

    #[test]
    fn test_human_entity_rejected() {
        let result = MapleVerseEntity::new_human("HumanUser", "user-123");
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(err.is_critical_violation());
        assert!(err.to_string().contains("CRITICAL VIOLATION"));
    }

    #[test]
    fn test_entity_kind_validation() {
        assert!(EntityKind::Individual.validate().is_ok());
        assert!(EntityKind::Collective.validate().is_ok());
        assert!(EntityKind::Human.validate().is_err());

        assert!(EntityKind::Individual.is_valid());
        assert!(EntityKind::Collective.is_valid());
        assert!(!EntityKind::Human.is_valid());
    }

    #[test]
    fn test_entity_builder_individual() {
        let entity = EntityBuilder::new()
            .name("BuilderAgent")
            .individual()
            .region(RegionId::new("region-1"))
            .maple(1000)
            .attention(500)
            .build()
            .unwrap();

        assert!(entity.is_individual());
        assert_eq!(entity.name, "BuilderAgent");
        assert_eq!(entity.maple_balance.amount(), 1000);
        assert_eq!(entity.attention_budget.available, 500);
    }

    #[test]
    fn test_entity_builder_human_rejected() {
        let result = EntityBuilder::new()
            .name("HumanUser")
            .human()
            .region(RegionId::new("region-1"))
            .build();

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.is_critical_violation());
    }

    #[test]
    fn test_individual_skills() {
        let mut entity =
            MapleVerseEntity::new_individual("SkillAgent", RegionId::new("region-1"), None);

        let individual = entity.as_individual_mut().unwrap();
        individual.add_skill("rust-programming");
        individual.add_skill("coordination");

        assert!(entity
            .as_individual()
            .unwrap()
            .skills
            .contains("rust-programming"));
        assert_eq!(entity.as_individual().unwrap().skills.len(), 2);
    }

    #[test]
    fn test_collective_members() {
        let founder = EntityId::generate();
        let mut entity = MapleVerseEntity::new_collective(
            "MemberCollective",
            RegionId::new("region-1"),
            None,
            founder.clone(),
        );

        let member1 = EntityId::generate();
        let member2 = EntityId::generate();

        let collective = entity.as_collective_mut().unwrap();
        collective.add_member(member1.clone()).unwrap();
        collective.add_member(member2.clone()).unwrap();

        assert_eq!(collective.member_count(), 3); // founder + 2 members

        // Cannot add same member twice
        assert!(collective.add_member(member1.clone()).is_err());

        // Can remove member
        collective.remove_member(&member1).unwrap();
        assert_eq!(collective.member_count(), 2);

        // Cannot remove non-member
        assert!(collective.remove_member(&member1).is_err());
    }

    #[test]
    fn test_individual_collective_membership() {
        let mut entity =
            MapleVerseEntity::new_individual("MemberAgent", RegionId::new("region-1"), None);

        let collective1 = EntityId::new("collective-1");
        let collective2 = EntityId::new("collective-2");

        let individual = entity.as_individual_mut().unwrap();
        individual.join_collective(collective1.clone());
        individual.join_collective(collective2.clone());

        assert!(individual.collective_memberships.contains(&collective1));
        assert_eq!(individual.collective_memberships.len(), 2);

        individual.leave_collective(&collective1);
        assert!(!individual.collective_memberships.contains(&collective1));
    }

    #[test]
    fn test_entity_can_act() {
        let mut entity =
            MapleVerseEntity::new_individual("ActiveAgent", RegionId::new("region-1"), None);

        entity.attention_budget = AttentionBudget::new(100, 100);
        assert!(entity.can_act());

        entity.attention_budget.available = 0;
        assert!(!entity.can_act());

        entity.attention_budget.available = 100;
        entity.status = EntityStatus::Suspended;
        assert!(!entity.can_act());
    }

    #[test]
    fn test_consume_attention() {
        let mut entity =
            MapleVerseEntity::new_individual("AttentionAgent", RegionId::new("region-1"), None);

        entity.attention_budget = AttentionBudget::new(100, 100);

        assert!(entity.consume_attention(50).is_ok());
        assert_eq!(entity.attention_budget.available, 50);

        assert!(entity.consume_attention(30).is_ok());
        assert_eq!(entity.attention_budget.available, 20);

        // Cannot consume more than available
        assert!(entity.consume_attention(50).is_err());
    }

    #[test]
    fn test_governance_models() {
        let models = vec![
            GovernanceModel::Consensus,
            GovernanceModel::Majority,
            GovernanceModel::Supermajority,
            GovernanceModel::Autocratic,
            GovernanceModel::Delegated,
            GovernanceModel::Custom("weighted-voting".to_string()),
        ];

        for model in models {
            let json = serde_json::to_string(&model).unwrap();
            let _: GovernanceModel = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_entity_serialization() {
        let entity =
            MapleVerseEntity::new_individual("SerializeAgent", RegionId::new("region-1"), None);

        let json = serde_json::to_string(&entity).unwrap();
        let deserialized: MapleVerseEntity = serde_json::from_str(&json).unwrap();

        assert_eq!(entity.id, deserialized.id);
        assert_eq!(entity.name, deserialized.name);
        assert!(deserialized.is_individual());
    }
}
