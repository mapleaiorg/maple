//! Python SDK bindings via PyO3.
//!
//! Exposes MWL kernel types and operations to Python:
//!
//! ```python
//! from maple import MapleSdk, WorldlineBuilder, CommitmentBuilder
//!
//! sdk = MapleSdk.connect("http://localhost:8080")
//! wl = sdk.create_worldline(profile="agent", label="my-agent")
//! commitment = CommitmentBuilder(wl.id) \
//!     .scope(effect_domain="communication", targets=["other-wl-id"]) \
//!     .capability("cap-send-message") \
//!     .build()
//! result = sdk.submit_commitment(commitment)
//! trail = sdk.audit_trail(result.commitment_id)
//! ```
//!
//! Requires the `python` feature flag and PyO3 runtime.

#[cfg(feature = "python")]
use pyo3::prelude::*;

/// Stub Python module — compiled only with `--features python`.
///
/// When PyO3 is not available, this module provides type definitions
/// that document the intended Python API surface.

// ──────────────────────────────────────────────
// Type definitions (always available for docs)
// ──────────────────────────────────────────────

/// Python-facing WorldLine result.
#[derive(Clone, Debug)]
pub struct PyWorldline {
    pub id: String,
    pub profile: String,
    pub label: Option<String>,
    pub status: String,
}

/// Python-facing commitment result.
#[derive(Clone, Debug)]
pub struct PyCommitmentResult {
    pub commitment_id: String,
    pub status: String,
    pub decision: String,
    pub risk_class: String,
}

/// Python-facing audit trail event.
#[derive(Clone, Debug)]
pub struct PyAuditEvent {
    pub event_id: String,
    pub stage: String,
    pub result: String,
    pub timestamp: String,
}

/// Python-facing balance projection.
#[derive(Clone, Debug)]
pub struct PyBalanceProjection {
    pub worldline_id: String,
    pub asset: String,
    pub balance_minor: i64,
    pub trajectory_length: usize,
}

/// Python-facing commitment builder.
#[derive(Clone, Debug)]
pub struct PyCommitmentBuilder {
    pub declaring_identity: String,
    pub effect_domain: Option<String>,
    pub targets: Vec<String>,
    pub capabilities: Vec<String>,
    pub evidence: Vec<String>,
}

impl PyCommitmentBuilder {
    pub fn new(declaring_identity: String) -> Self {
        Self {
            declaring_identity,
            effect_domain: None,
            targets: vec![],
            capabilities: vec![],
            evidence: vec![],
        }
    }

    pub fn scope(mut self, effect_domain: String, targets: Vec<String>) -> Self {
        self.effect_domain = Some(effect_domain);
        self.targets = targets;
        self
    }

    pub fn capability(mut self, cap: String) -> Self {
        self.capabilities.push(cap);
        self
    }

    pub fn evidence_item(mut self, ev: String) -> Self {
        self.evidence.push(ev);
        self
    }

    pub fn build(self) -> PyCommitmentDeclaration {
        PyCommitmentDeclaration {
            declaring_identity: self.declaring_identity,
            effect_domain: self.effect_domain.unwrap_or_else(|| "communication".into()),
            targets: self.targets,
            capabilities: self.capabilities,
            evidence: self.evidence,
        }
    }
}

/// Python-facing commitment declaration (ready to submit).
#[derive(Clone, Debug)]
pub struct PyCommitmentDeclaration {
    pub declaring_identity: String,
    pub effect_domain: String,
    pub targets: Vec<String>,
    pub capabilities: Vec<String>,
    pub evidence: Vec<String>,
}

/// Python SDK client — connects to a running PALM daemon.
#[derive(Clone, Debug)]
pub struct PyMapleSdk {
    pub endpoint: String,
}

impl PyMapleSdk {
    pub fn connect(endpoint: String) -> Self {
        Self { endpoint }
    }

    /// API base URL.
    pub fn api_base(&self) -> String {
        format!("{}/api/v1", self.endpoint.trim_end_matches('/'))
    }
}

// ──────────────────────────────────────────────
// PyO3 module (only with python feature)
// ──────────────────────────────────────────────

#[cfg(feature = "python")]
#[pymethods]
impl PyMapleSdk {
    #[staticmethod]
    fn py_connect(endpoint: String) -> Self {
        Self::connect(endpoint)
    }
}

#[cfg(feature = "python")]
#[pymodule]
fn maple(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyMapleSdk>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdk_connect() {
        let sdk = PyMapleSdk::connect("http://localhost:8080".into());
        assert_eq!(sdk.endpoint, "http://localhost:8080");
        assert_eq!(sdk.api_base(), "http://localhost:8080/api/v1");
    }

    #[test]
    fn sdk_connect_strips_trailing_slash() {
        let sdk = PyMapleSdk::connect("http://localhost:8080/".into());
        assert_eq!(sdk.api_base(), "http://localhost:8080/api/v1");
    }

    #[test]
    fn commitment_builder_basic() {
        let builder = PyCommitmentBuilder::new("wl-123".into());
        let decl = builder
            .scope("communication".into(), vec!["wl-456".into()])
            .capability("cap-send".into())
            .evidence_item("signed-intent".into())
            .build();

        assert_eq!(decl.declaring_identity, "wl-123");
        assert_eq!(decl.effect_domain, "communication");
        assert_eq!(decl.targets, vec!["wl-456"]);
        assert_eq!(decl.capabilities, vec!["cap-send"]);
        assert_eq!(decl.evidence, vec!["signed-intent"]);
    }

    #[test]
    fn commitment_builder_default_domain() {
        let decl = PyCommitmentBuilder::new("wl-123".into()).build();
        assert_eq!(decl.effect_domain, "communication");
    }

    #[test]
    fn commitment_builder_multiple_capabilities() {
        let decl = PyCommitmentBuilder::new("wl-123".into())
            .capability("cap-a".into())
            .capability("cap-b".into())
            .capability("cap-c".into())
            .build();
        assert_eq!(decl.capabilities.len(), 3);
    }

    #[test]
    fn worldline_type_fields() {
        let wl = PyWorldline {
            id: "wl-abc".into(),
            profile: "agent".into(),
            label: Some("my-agent".into()),
            status: "active".into(),
        };
        assert_eq!(wl.id, "wl-abc");
        assert_eq!(wl.profile, "agent");
    }

    #[test]
    fn commitment_result_fields() {
        let result = PyCommitmentResult {
            commitment_id: "cm-123".into(),
            status: "approved".into(),
            decision: "approve".into(),
            risk_class: "low".into(),
        };
        assert_eq!(result.commitment_id, "cm-123");
        assert_eq!(result.decision, "approve");
    }

    #[test]
    fn balance_projection_fields() {
        let balance = PyBalanceProjection {
            worldline_id: "wl-123".into(),
            asset: "USD".into(),
            balance_minor: 100_000,
            trajectory_length: 5,
        };
        assert_eq!(balance.balance_minor, 100_000);
    }
}
