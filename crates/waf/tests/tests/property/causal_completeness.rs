//! Property tests: Any random sequence of context graph operations maintains causal completeness.
//!
//! Causal completeness means every node is reachable from its declared parents,
//! and all parent references resolve to existing nodes in the graph.

use maple_waf_context_graph::{
    ContentHash, ContextGraphManager, GovernanceTier, InMemoryContextGraphManager, IntentNode,
    NodeContent,
};
use proptest::prelude::*;
use worldline_types::{EventId, TemporalAnchor, WorldlineId};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a random governance tier.
fn arb_governance_tier() -> impl Strategy<Value = GovernanceTier> {
    prop_oneof![
        Just(GovernanceTier::Tier0),
        Just(GovernanceTier::Tier1),
        Just(GovernanceTier::Tier2),
        Just(GovernanceTier::Tier3),
        Just(GovernanceTier::Tier4),
        Just(GovernanceTier::Tier5),
    ]
}

/// Generate a random intent description.
fn arb_description() -> impl Strategy<Value = String> {
    "[a-z]{3,20}".prop_map(|s| format!("intent: {}", s))
}

// ---------------------------------------------------------------------------
// Property Tests
// ---------------------------------------------------------------------------

proptest! {
    /// Adding N root nodes (no parents) maintains graph integrity: all have valid content hashes.
    #[test]
    fn root_nodes_always_have_valid_content_hashes(
        count in 1usize..20,
        tier in arb_governance_tier(),
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mgr = InMemoryContextGraphManager::new();
            let wl = WorldlineId::ephemeral();

            let mut ids = Vec::new();
            for i in 0..count {
                let intent = IntentNode::new(
                    EventId::new(),
                    format!("intent-{}", i),
                    tier,
                );
                let id = mgr
                    .append(
                        wl.clone(),
                        NodeContent::Intent(intent),
                        vec![],
                        TemporalAnchor::new((i as u64 + 1) * 100, 0, 0),
                        tier,
                    )
                    .await
                    .unwrap();
                ids.push(id);
            }

            // Every node should be retrievable and have a valid hash.
            for id in &ids {
                let node = mgr.get_node(id).await.unwrap().unwrap();
                prop_assert!(node.verify_content_hash().is_ok());
            }

            prop_assert_eq!(mgr.node_count().await.unwrap(), count);
            Ok(())
        })?;
    }

    /// A chain of parent-child nodes always maintains causal completeness.
    #[test]
    fn chain_maintains_causal_completeness(
        chain_len in 2usize..10,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mgr = InMemoryContextGraphManager::new();
            let wl = WorldlineId::ephemeral();

            let mut prev_id: Option<ContentHash> = None;
            let mut all_ids = Vec::new();

            for i in 0..chain_len {
                let parents = match &prev_id {
                    Some(pid) => vec![pid.clone()],
                    None => vec![],
                };

                let intent = IntentNode::new(
                    EventId::new(),
                    format!("chain-node-{}", i),
                    GovernanceTier::Tier0,
                );
                let id = mgr
                    .append(
                        wl.clone(),
                        NodeContent::Intent(intent),
                        parents,
                        TemporalAnchor::new((i as u64 + 1) * 100, 0, 0),
                        GovernanceTier::Tier0,
                    )
                    .await
                    .unwrap();

                all_ids.push(id.clone());
                prev_id = Some(id);
            }

            // Validate the chain from the leaf node.
            let leaf_id = all_ids.last().unwrap();
            let validation = mgr.validate_chain(leaf_id).await.unwrap();
            prop_assert!(validation.valid);
            prop_assert!(validation.checks_performed >= chain_len);

            // Every node should be retrievable.
            for id in &all_ids {
                let node = mgr.get_node(id).await.unwrap();
                prop_assert!(node.is_some());
            }

            Ok(())
        })?;
    }

    /// Parent references in retrieved nodes always match what was provided at append time.
    #[test]
    fn parent_references_preserved(
        branching in 1usize..5,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mgr = InMemoryContextGraphManager::new();
            let wl = WorldlineId::ephemeral();

            // Create root nodes.
            let mut root_ids = Vec::new();
            for i in 0..branching {
                let intent = IntentNode::new(
                    EventId::new(),
                    format!("root-{}", i),
                    GovernanceTier::Tier0,
                );
                let id = mgr
                    .append(
                        wl.clone(),
                        NodeContent::Intent(intent),
                        vec![],
                        TemporalAnchor::new((i as u64 + 1) * 100, 0, 0),
                        GovernanceTier::Tier0,
                    )
                    .await
                    .unwrap();
                root_ids.push(id);
            }

            // Create a child with all roots as parents (merge node).
            let child_intent = IntentNode::new(
                EventId::new(),
                "merge child",
                GovernanceTier::Tier0,
            );
            let child_id = mgr
                .append(
                    wl.clone(),
                    NodeContent::Intent(child_intent),
                    root_ids.clone(),
                    TemporalAnchor::new(10000, 0, 0),
                    GovernanceTier::Tier0,
                )
                .await
                .unwrap();

            // Retrieve and verify parent references.
            let child_node = mgr.get_node(&child_id).await.unwrap().unwrap();
            prop_assert_eq!(child_node.parent_ids.len(), branching);
            for root_id in &root_ids {
                prop_assert!(child_node.parent_ids.contains(root_id));
            }

            // Validate chain from child should pass.
            let validation = mgr.validate_chain(&child_id).await.unwrap();
            prop_assert!(validation.valid);

            Ok(())
        })?;
    }

    /// Worldline isolation: operations on one worldline do not affect another.
    #[test]
    fn worldlines_are_isolated(
        count_wl1 in 1usize..5,
        count_wl2 in 1usize..5,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mgr = InMemoryContextGraphManager::new();
            let wl1 = WorldlineId::ephemeral();
            let wl2 = WorldlineId::ephemeral();

            for i in 0..count_wl1 {
                let intent = IntentNode::new(
                    EventId::new(),
                    format!("wl1-{}", i),
                    GovernanceTier::Tier0,
                );
                mgr.append(
                    wl1.clone(),
                    NodeContent::Intent(intent),
                    vec![],
                    TemporalAnchor::new((i as u64 + 1) * 100, 0, 0),
                    GovernanceTier::Tier0,
                )
                .await
                .unwrap();
            }

            for i in 0..count_wl2 {
                let intent = IntentNode::new(
                    EventId::new(),
                    format!("wl2-{}", i),
                    GovernanceTier::Tier0,
                );
                mgr.append(
                    wl2.clone(),
                    NodeContent::Intent(intent),
                    vec![],
                    TemporalAnchor::new((i as u64 + 1) * 100, 0, 0),
                    GovernanceTier::Tier0,
                )
                .await
                .unwrap();
            }

            let chains1 = mgr.get_evolution_chains(&wl1).await.unwrap();
            let chains2 = mgr.get_evolution_chains(&wl2).await.unwrap();
            prop_assert_eq!(chains1.len(), count_wl1);
            prop_assert_eq!(chains2.len(), count_wl2);

            // Total should be the sum.
            prop_assert_eq!(
                mgr.node_count().await.unwrap(),
                count_wl1 + count_wl2
            );

            Ok(())
        })?;
    }
}
