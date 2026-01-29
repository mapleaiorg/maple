//! Temporal Types
use crate::identity::CausalRef;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemporalAnchor {
    pub local_time: chrono::DateTime<chrono::Utc>,
    pub causal_refs: Vec<CausalRef>,
    pub sequence: u64,
}

impl TemporalAnchor {
    pub fn now() -> Self {
        Self {
            local_time: chrono::Utc::now(),
            causal_refs: Vec::new(),
            sequence: 0,
        }
    }
    
    pub fn at(time: chrono::DateTime<chrono::Utc>) -> Self {
        Self {
            local_time: time,
            causal_refs: Vec::new(),
            sequence: 0,
        }
    }
}

impl Default for TemporalAnchor {
    fn default() -> Self { Self::now() }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TemporalValidity {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid_until: Option<chrono::DateTime<chrono::Utc>>,
}

impl TemporalValidity {
    pub fn new(valid_from: chrono::DateTime<chrono::Utc>, valid_until: chrono::DateTime<chrono::Utc>) -> Self {
        Self { valid_from: Some(valid_from), valid_until: Some(valid_until) }
    }

    pub fn unbounded() -> Self {
        Self { valid_from: None, valid_until: None }
    }

    pub fn from_now_secs(seconds: i64) -> Self {
        let now = chrono::Utc::now();
        Self {
            valid_from: Some(now),
            valid_until: Some(now + chrono::Duration::seconds(seconds)),
        }
    }

    pub fn is_valid_at(&self, time: chrono::DateTime<chrono::Utc>) -> bool {
        if let Some(from) = self.valid_from {
            if time < from {
                return false;
            }
        }
        if let Some(until) = self.valid_until {
            if time > until {
                return false;
            }
        }
        true
    }

    pub fn is_valid_now(&self) -> bool {
        self.is_valid_at(chrono::Utc::now())
    }
}

impl Default for TemporalValidity {
    fn default() -> Self {
        Self::unbounded()
    }
}
