use maple_kernel_fabric::ResonanceStage;
use maple_mwl_types::{EventId, WorldlineId};
use serde::{Deserialize, Serialize};

/// Impact analysis report for a given event.
///
/// Shows how an event's consequences rippled through the system.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ImpactReport {
    /// The event being analyzed
    pub event_id: EventId,
    /// Total number of descendant events
    pub total_descendants: usize,
    /// WorldLines affected by this event's causal chain
    pub affected_worldlines: Vec<WorldlineId>,
    /// Breakdown by resonance stage
    pub stage_breakdown: Vec<(String, usize)>,
    /// Depth of the deepest causal chain from this event
    pub max_depth: u32,
}

/// Risk contagion report for a worldline.
///
/// Shows how a worldline's events are causally connected to other worldlines.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContagionReport {
    /// The worldline being analyzed
    pub worldline: WorldlineId,
    /// WorldLines that are causally downstream of this worldline
    pub downstream_worldlines: Vec<WorldlineId>,
    /// WorldLines that are causally upstream of this worldline
    pub upstream_worldlines: Vec<WorldlineId>,
    /// Total causal connections to other worldlines
    pub total_connections: usize,
    /// Highest-risk resonance stage in the causal chain
    pub highest_stage: Option<ResonanceStage>,
}
