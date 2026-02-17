use serde::{Deserialize, Serialize};

/// Strong typed IDs used throughout MWL.

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub uuid::Uuid);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitmentId(pub uuid::Uuid);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CapabilityId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PolicyId(pub String);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProvenanceRef(pub EventId);

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u16);

impl EventId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for EventId {
    fn default() -> Self {
        Self::new()
    }
}

impl CommitmentId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for CommitmentId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EventId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "evt:{}", self.0)
    }
}

impl std::fmt::Display for CommitmentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cmt:{}", self.0)
    }
}

impl std::fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cap:{}", self.0)
    }
}

impl std::fmt::Display for PolicyId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "pol:{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_id_uniqueness() {
        let a = EventId::new();
        let b = EventId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn commitment_id_uniqueness() {
        let a = CommitmentId::new();
        let b = CommitmentId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn event_id_serialization() {
        let id = EventId::new();
        let json = serde_json::to_string(&id).unwrap();
        let restored: EventId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, restored);
    }

    #[test]
    fn display_formats() {
        let eid = EventId::new();
        assert!(format!("{}", eid).starts_with("evt:"));

        let cid = CommitmentId::new();
        assert!(format!("{}", cid).starts_with("cmt:"));

        let cap = CapabilityId("CAP-001".into());
        assert_eq!(format!("{}", cap), "cap:CAP-001");

        let pol = PolicyId("POL-001".into());
        assert_eq!(format!("{}", pol), "pol:POL-001");
    }
}
