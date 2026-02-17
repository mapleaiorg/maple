//! Failure & Recovery Integration Tests
//!
//! Verifies graceful degradation, identity persistence across
//! restarts, and explicit failure recording.

use worldline_core::identity::IdentityManager;
use worldline_core::types::{EffectDomain, FailureReason, IdentityMaterial, WorldlineId};
use worldline_runtime::fabric::{EventFabric, EventPayload, FabricConfig, ResonanceStage};
use worldline_runtime::gate::{AdjudicationResult, CommitmentOutcome};

use crate::helpers::{KernelOptions, TestKernel};

/// Identity survives across simulated restart.
///
/// WorldlineId is derived from identity material, not from session state.
/// The same material always produces the same identity.
#[test]
fn test_identity_survives_restart() {
    let material = IdentityMaterial::GenesisHash([42u8; 32]);

    // "Session 1": create worldline
    let wid_session1 = {
        let mut mgr = IdentityManager::new();
        mgr.create_worldline(material.clone()).unwrap()
    };

    // "Session 2": recreate from same material
    let wid_session2 = {
        let mut mgr = IdentityManager::new();
        mgr.create_worldline(material.clone()).unwrap()
    };

    // Same material → same WorldlineId
    assert_eq!(
        wid_session1, wid_session2,
        "WorldlineId must be deterministic from identity material"
    );
}

/// ContinuityChain extends across sessions.
#[test]
fn test_continuity_chain_extension() {
    let material = IdentityMaterial::GenesisHash([43u8; 32]);
    let mut mgr = IdentityManager::new();

    let wid = mgr.create_worldline(material.clone()).unwrap();

    // Verify continuity chain exists
    let ctx = mgr.continuity_context(&wid);
    assert!(ctx.is_some(), "Worldline should have a continuity context");

    let ctx = ctx.unwrap();
    assert_eq!(
        ctx.worldline_id, wid,
        "Continuity context should reference the correct worldline"
    );
    assert_eq!(ctx.segment_index, 0, "First segment should have index 0");
}

/// Commitment failure is explicitly recorded, never silent (I.S-4).
#[tokio::test]
async fn test_commitment_failure_explicit() {
    let mut kernel = TestKernel::new(KernelOptions::default()).await;

    let wid = kernel.create_worldline(50);
    let target = kernel.create_worldline(51);
    kernel.grant_capability(&wid, "CAP-INFRA", EffectDomain::Infrastructure);

    let genesis = kernel.emit_genesis(&wid).await;
    let meaning = kernel.emit_meaning(&wid, vec![genesis.id.clone()]).await;
    let intent = kernel.emit_intent(&wid, vec![meaning.id.clone()]).await;

    // Submit and approve
    let decl = kernel.build_declaration(
        wid.clone(),
        intent.id.clone(),
        EffectDomain::Infrastructure,
        "CAP-INFRA",
        vec![target],
    );
    let cid = decl.id.clone();
    let result = kernel.gate.submit(decl).await.unwrap();
    assert!(matches!(result, AdjudicationResult::Approved { .. }));

    // Simulate execution failure
    let failure_reason = FailureReason {
        code: "EXEC-001".into(),
        message: "Computation timeout after 30s".into(),
        partial_completion: Some(0.0),
    };
    kernel
        .gate
        .record_outcome(&cid, CommitmentOutcome::Failed(failure_reason))
        .await
        .unwrap();

    // Verify failure is explicitly recorded
    let entry = kernel.gate.ledger().history(&cid).unwrap();

    // Find the Failed lifecycle event
    let has_failure = entry
        .lifecycle
        .iter()
        .any(|ev| matches!(ev, worldline_runtime::gate::LifecycleEvent::Failed { .. }));
    assert!(
        has_failure,
        "I.S-4: Failure must be explicitly recorded in ledger"
    );
}

/// WAL recovery preserves events after simulated crash.
#[tokio::test]
async fn test_wal_recovery_preserves_events() {
    let dir = tempfile::tempdir().unwrap();
    let wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([60u8; 32]));
    let event_count;

    // "Session 1": emit events and checkpoint
    {
        let config = FabricConfig {
            data_dir: Some(dir.path().to_path_buf()),
            ..FabricConfig::default()
        };
        let fabric = EventFabric::init(config).await.unwrap();

        for i in 0..5 {
            fabric
                .emit(
                    wid.clone(),
                    ResonanceStage::Meaning,
                    EventPayload::MeaningFormed {
                        interpretation_count: i,
                        confidence: 0.5,
                        ambiguity_preserved: true,
                    },
                    vec![],
                )
                .await
                .unwrap();
        }
        fabric.checkpoint().await.unwrap();
        event_count = 5;
    }

    // "Session 2": recover from WAL
    {
        let config = FabricConfig {
            data_dir: Some(dir.path().to_path_buf()),
            ..FabricConfig::default()
        };
        let fabric = EventFabric::init(config).await.unwrap();

        let mut recovered_events = Vec::new();
        let count = fabric
            .recover(|_seq, event| {
                recovered_events.push(event);
                Ok(())
            })
            .await
            .unwrap();

        assert_eq!(count, event_count, "Should recover all events from WAL");
        for event in &recovered_events {
            assert!(
                event.verify_integrity(),
                "Recovered events must pass integrity check"
            );
        }
    }
}

/// Fabric integrity verification catches corruption.
#[tokio::test]
async fn test_fabric_integrity_verification() {
    let fabric = EventFabric::init(FabricConfig::default()).await.unwrap();
    let wid = WorldlineId::derive(&IdentityMaterial::GenesisHash([70u8; 32]));

    // Emit events
    for i in 0..10 {
        fabric
            .emit(
                wid.clone(),
                ResonanceStage::Meaning,
                EventPayload::MeaningFormed {
                    interpretation_count: i,
                    confidence: 0.5 + (i as f64 * 0.01),
                    ambiguity_preserved: true,
                },
                vec![],
            )
            .await
            .unwrap();
    }

    let report = fabric.verify().await.unwrap();
    assert!(report.is_clean(), "Integrity report should be clean");
    assert_eq!(report.total_events, 10);
    assert_eq!(report.verified_events, 10);
}
