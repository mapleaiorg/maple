//! Resonator Types - Cognitive agent types
//!
//! Resonators think and produce Meaning/Intent/Commitment drafts.
//! They have NO execution authority - all commitments must go through AAS.

#![deny(unsafe_code)]

use rcf_commitment::RcfCommitment;
use rcf_types::{EffectDomain, IdentityRef, ScopeConstraint};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A Resonator - a cognitive entity with NO execution authority
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resonator {
    pub resonator_id: ResonatorId,
    pub identity: IdentityRef,
    pub profile: ResonatorProfile,
    pub state: ResonatorState,
    pub capabilities: Vec<CognitiveCapability>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Unique identifier for a Resonator
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResonatorId(pub String);

impl ResonatorId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for ResonatorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Profile defining a Resonator's cognitive characteristics
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResonatorProfile {
    pub name: String,
    pub description: String,
    pub domains: Vec<EffectDomain>,
    pub risk_tolerance: RiskTolerance,
    pub autonomy_level: AutonomyLevel,
    pub constraints: Vec<ProfileConstraint>,
}

/// Attention budget attached to an agent shell.
///
/// This is the canonical budget shape used when AgentKernel composes
/// Resonator cognition with governance/execution surfaces.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttentionBudget {
    pub total: u64,
    pub allocated: u64,
    pub reserved: u64,
}

impl AttentionBudget {
    pub fn available(&self) -> u64 {
        self.total
            .saturating_sub(self.allocated.saturating_add(self.reserved))
    }
}

impl Default for AttentionBudget {
    fn default() -> Self {
        Self {
            total: 100,
            allocated: 0,
            reserved: 0,
        }
    }
}

/// Coupling edge metadata for one relation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CouplingEdge {
    pub target: ResonatorId,
    pub strength: f64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Lightweight coupling graph view used by runtime composition.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CouplingGraph {
    pub edges: HashMap<ResonatorId, Vec<CouplingEdge>>,
}

impl CouplingGraph {
    pub fn upsert_edge(&mut self, source: ResonatorId, edge: CouplingEdge) {
        let entry = self.edges.entry(source).or_default();
        if let Some(existing) = entry.iter_mut().find(|current| current.target == edge.target) {
            *existing = edge;
            return;
        }
        entry.push(edge);
    }
}

/// Risk tolerance level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RiskTolerance {
    Conservative,
    #[default]
    Balanced,
    Aggressive,
}

/// Autonomy level - how much can the Resonator do without human review
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AutonomyLevel {
    /// All commitments require human approval
    FullHumanOversight,
    /// Low-risk commitments can be auto-approved
    #[default]
    GuidedAutonomy,
    /// Only high-risk commitments need review
    HighAutonomy,
}

/// Profile constraints
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProfileConstraint {
    pub constraint_type: ConstraintType,
    pub description: String,
    pub parameters: HashMap<String, String>,
}

/// Types of constraints
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintType {
    DomainRestriction,
    ScopeLimit,
    RateLimit,
    TimeRestriction,
    Custom(String),
}

/// Resonator state
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResonatorState {
    pub status: ResonatorStatus,
    pub current_context: Option<CognitiveContext>,
    pub pending_commitments: Vec<PendingCommitment>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

impl Default for ResonatorState {
    fn default() -> Self {
        Self {
            status: ResonatorStatus::Idle,
            current_context: None,
            pending_commitments: vec![],
            last_activity: chrono::Utc::now(),
        }
    }
}

/// Resonator status
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ResonatorStatus {
    #[default]
    Idle,
    Processing,
    WaitingForApproval,
    Suspended,
}

/// Cognitive capabilities a Resonator can have
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CognitiveCapability {
    /// Can produce Meaning from input
    MeaningProduction,
    /// Can formulate Intents from Meanings
    IntentFormulation,
    /// Can draft Commitments from Intents
    CommitmentDrafting,
    /// Can analyze consequences
    ConsequenceAnalysis,
    /// Can learn from feedback
    FeedbackLearning,
}

/// Context for cognitive processing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CognitiveContext {
    pub context_id: String,
    pub task_description: String,
    pub inputs: Vec<ContextInput>,
    pub produced_meanings: Vec<MeaningDraft>,
    pub produced_intents: Vec<IntentDraft>,
    pub produced_commitments: Vec<CommitmentDraft>,
    pub started_at: chrono::DateTime<chrono::Utc>,
}

/// Input to cognitive processing
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextInput {
    pub input_type: InputType,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

/// Types of inputs
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputType {
    Text,
    UserRequest,
    Observation,
    Feedback,
}

/// A meaning draft produced by a Resonator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeaningDraft {
    pub draft_id: String,
    pub content: String,
    pub interpretation: String,
    pub confidence: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// An intent draft produced by a Resonator
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentDraft {
    pub draft_id: String,
    pub goal: String,
    pub rationale: String,
    pub source_meanings: Vec<String>,
    pub confidence: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// A commitment draft produced by a Resonator (NOT YET APPROVED)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentDraft {
    pub draft_id: String,
    pub source_intent: String,
    pub proposed_domain: EffectDomain,
    pub proposed_scope: ScopeConstraint,
    pub description: String,
    pub rationale: String,
    pub estimated_risk: EstimatedRisk,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Estimated risk of a commitment draft
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EstimatedRisk {
    pub level: RiskLevel,
    pub factors: Vec<String>,
    pub mitigations: Vec<String>,
}

/// Risk level
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum RiskLevel {
    #[default]
    Low,
    Medium,
    High,
    Critical,
}

/// A commitment pending AAS approval
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingCommitment {
    pub draft_id: String,
    pub commitment: RcfCommitment,
    pub submitted_at: chrono::DateTime<chrono::Utc>,
    pub status: PendingStatus,
}

/// Status of a pending commitment
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PendingStatus {
    AwaitingSubmission,
    Submitted,
    UnderReview,
    Approved,
    Denied,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resonator_id() {
        let id = ResonatorId::generate();
        assert!(!id.0.is_empty());
    }
}
