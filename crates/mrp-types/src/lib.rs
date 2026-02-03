#![deny(unsafe_code)]
use rcf_commitment::RcfCommitment;
use rcf_types::{
    hex_bytes_32, hex_bytes_64, CapabilityRef, ContinuityRef, IdentityRef, ResonanceType,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MrpEnvelope {
    pub header: EnvelopeHeader,
    pub body: EnvelopeBody,
    pub integrity: EnvelopeIntegrity,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvelopeHeader {
    pub envelope_id: EnvelopeId,
    pub resonance_type: ResonanceType,
    pub schema_version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub ttl: u32,
    pub origin_identity: IdentityRef,
    pub continuity_context: ContinuityRef,
    pub trace_context: TraceContext,
    pub routing_constraints: RoutingConstraints,
    #[serde(default)]
    pub capability_refs: Vec<CapabilityRef>,
    #[serde(default)]
    pub policy_tags: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EnvelopeId(pub String);
impl EnvelopeId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct TraceContext {
    pub trace_id: String,
    pub span_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_span_id: Option<String>,
    #[serde(default)]
    pub baggage: HashMap<String, String>,
}
impl TraceContext {
    pub fn new() -> Self {
        Self {
            trace_id: uuid::Uuid::new_v4().to_string(),
            span_id: uuid::Uuid::new_v4().to_string(),
            parent_span_id: None,
            baggage: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct RoutingConstraints {
    pub required_destinations: Vec<Destination>,
    #[serde(default)]
    pub forbidden_destinations: Vec<Destination>,
    pub priority: RoutingPriority,
    pub delivery: DeliveryRequirements,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Destination {
    pub destination_type: DestinationType,
    pub identifier: String,
}
impl Destination {
    pub fn service(name: impl Into<String>) -> Self {
        Self {
            destination_type: DestinationType::Service,
            identifier: name.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DestinationType {
    #[default]
    Service,
    Identity,
    Broadcast,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum RoutingPriority {
    Low,
    #[default]
    Normal,
    High,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveryRequirements {
    pub guaranteed: bool,
    pub ordered: bool,
    pub max_retries: u32,
}
impl Default for DeliveryRequirements {
    fn default() -> Self {
        Self {
            guaranteed: true,
            ordered: true,
            max_retries: 3,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EnvelopeBody {
    Meaning(MeaningPayload),
    Intent(IntentPayload),
    Commitment(CommitmentPayload),
    Consequence(ConsequencePayload),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MeaningPayload {
    pub meaning_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntentPayload {
    pub intent_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentPayload {
    pub commitment: RcfCommitment,
    pub submission: CommitmentSubmission,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsequencePayload {
    pub commitment_ref: rcf_commitment::CommitmentId,
    pub outcome: Outcome,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CommitmentSubmission {
    pub submission_id: String,
    pub adjudication_type: AdjudicationType,
    pub urgency: Urgency,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AdjudicationType {
    #[default]
    Standard,
    Expedited,
    PreApproved,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Urgency {
    #[default]
    Normal,
    High,
    Critical,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Outcome {
    pub status: OutcomeStatus,
    pub description: String,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomeStatus {
    Success,
    PartialSuccess,
    Failure,
    Aborted,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct EnvelopeIntegrity {
    #[serde(with = "hex_bytes_32")]
    pub hash: [u8; 32],
    pub signatures: Vec<EnvelopeSignature>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EnvelopeSignature {
    pub signer: IdentityRef,
    #[serde(with = "hex_bytes_64")]
    pub signature: [u8; 64],
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
