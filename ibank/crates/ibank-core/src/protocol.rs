use crate::error::IBankError;
use crate::ledger::AppendOnlyLedger;
use crate::types::{
    AccountableWireMessage, AuditWitness, CommitmentReference, OriginProof, TransferPayload,
};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

/// Authority used to sign and verify origin proofs.
///
/// This uses deterministic keyed hashing for reproducible tests and stable audits.
/// In production deployments this should be backed by asymmetric signatures and HSM keys.
#[derive(Debug, Clone, Default)]
pub struct OriginAuthority {
    keys: HashMap<String, String>,
}

impl OriginAuthority {
    pub fn new() -> Self {
        Self {
            keys: HashMap::new(),
        }
    }

    pub fn register_key(&mut self, key_id: impl Into<String>, secret: impl Into<String>) {
        self.keys.insert(key_id.into(), secret.into());
    }

    pub fn has_key(&self, key_id: &str) -> bool {
        self.keys.contains_key(key_id)
    }

    pub fn sign_message(
        &self,
        key_id: &str,
        origin_actor: &str,
        trace_id: &str,
        payload: &TransferPayload,
        witness: &AuditWitness,
        commitment_ref: Option<&CommitmentReference>,
    ) -> Result<OriginProof, IBankError> {
        let secret = self
            .keys
            .get(key_id)
            .ok_or_else(|| IBankError::Accountability(format!("unknown key_id '{}'", key_id)))?;

        let signed_at = Utc::now();
        let nonce = Uuid::new_v4().to_string();
        let digest = signing_digest(
            secret,
            origin_actor,
            trace_id,
            payload,
            witness,
            commitment_ref,
            &nonce,
            signed_at,
        )?;

        Ok(OriginProof {
            key_id: key_id.to_string(),
            nonce,
            signed_at,
            signature: digest,
        })
    }

    pub fn verify_message(&self, message: &AccountableWireMessage) -> Result<(), IBankError> {
        let secret = self.keys.get(&message.origin_proof.key_id).ok_or_else(|| {
            IBankError::Accountability(format!("unknown key_id '{}'", message.origin_proof.key_id))
        })?;

        let expected = signing_digest(
            secret,
            &message.origin_actor,
            &message.trace_id,
            &message.payload,
            &message.audit_witness,
            message.commitment_ref.as_ref(),
            &message.origin_proof.nonce,
            message.origin_proof.signed_at,
        )?;

        if expected != message.origin_proof.signature {
            return Err(IBankError::Accountability(
                "origin signature mismatch".to_string(),
            ));
        }

        Ok(())
    }
}

/// Build a canonical accountable wire message.
pub fn build_accountable_wire_message(
    trace_id: &str,
    origin_actor: &str,
    payload: TransferPayload,
    witness: AuditWitness,
    commitment_ref: Option<CommitmentReference>,
    authority: &OriginAuthority,
    key_id: &str,
) -> Result<AccountableWireMessage, IBankError> {
    let proof = authority.sign_message(
        key_id,
        origin_actor,
        trace_id,
        &payload,
        &witness,
        commitment_ref.as_ref(),
    )?;

    Ok(AccountableWireMessage {
        message_id: AccountableWireMessage::message_id(),
        trace_id: trace_id.to_string(),
        origin_actor: origin_actor.to_string(),
        payload,
        origin_proof: proof,
        audit_witness: witness,
        commitment_ref,
    })
}

/// Verify accountable message structure and referenced audit witness.
///
/// Non-negotiable enforcement:
/// - origin proof must be valid
/// - audit witness must resolve to a real ledger entry
/// - witness hash must match ledger hash
pub fn verify_accountable_wire_message(
    message: &AccountableWireMessage,
    authority: &OriginAuthority,
    ledger: &AppendOnlyLedger,
) -> Result<(), IBankError> {
    authority.verify_message(message)?;

    let entry = ledger
        .find_entry(&message.audit_witness.entry_id)
        .ok_or_else(|| {
            IBankError::Accountability(format!(
                "audit entry '{}' not found",
                message.audit_witness.entry_id
            ))
        })?;

    if entry.entry_hash != message.audit_witness.entry_hash {
        return Err(IBankError::Accountability(
            "audit witness hash mismatch".to_string(),
        ));
    }

    Ok(())
}

fn signing_digest(
    secret: &str,
    origin_actor: &str,
    trace_id: &str,
    payload: &TransferPayload,
    witness: &AuditWitness,
    commitment_ref: Option<&CommitmentReference>,
    nonce: &str,
    signed_at: chrono::DateTime<Utc>,
) -> Result<String, IBankError> {
    let material = serde_json::json!({
        "secret": secret,
        "origin_actor": origin_actor,
        "trace_id": trace_id,
        "payload": payload,
        "witness": witness,
        "commitment_ref": commitment_ref,
        "nonce": nonce,
        "signed_at": signed_at,
    });

    let bytes = serde_json::to_vec(&material).map_err(|e| {
        IBankError::Serialization(format!("failed to encode signing material: {e}"))
    })?;
    Ok(blake3::hash(&bytes).to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{AuditEvent, LedgerEntryKind};

    #[test]
    fn verifies_end_to_end_accountable_message() {
        let mut ledger = AppendOnlyLedger::new();
        let audit = ledger
            .append_audit("trace-z", None, AuditEvent::new("prepared", "ok"))
            .unwrap();
        assert_eq!(audit.kind, LedgerEntryKind::Audit);

        let payload = TransferPayload {
            from: "agent-a".to_string(),
            to: "agent-b".to_string(),
            amount_minor: 1_000,
            currency: "USD".to_string(),
            destination: "acct-1".to_string(),
            purpose: "settle".to_string(),
        };

        let mut authority = OriginAuthority::new();
        authority.register_key("ibank-node", "local-secret");

        let witness = AuditWitness {
            entry_id: audit.entry_id.clone(),
            entry_hash: audit.entry_hash.clone(),
            observed_at: Utc::now(),
        };

        let message = build_accountable_wire_message(
            "trace-z",
            "agent-a",
            payload,
            witness,
            None,
            &authority,
            "ibank-node",
        )
        .unwrap();

        assert!(verify_accountable_wire_message(&message, &authority, &ledger).is_ok());
    }
}
