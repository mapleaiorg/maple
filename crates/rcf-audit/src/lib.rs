#![deny(unsafe_code)]
use rcf_types::IdentityRef;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditTrail {
    pub trail_id: String,
    pub identity: IdentityRef,
    pub events: Vec<AuditEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuditEvent {
    pub event_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub description: String,
}
