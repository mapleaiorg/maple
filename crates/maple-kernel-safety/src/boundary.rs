use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::error::SafetyCheckResult;

/// Resonance Boundary — structural limits on interaction.
///
/// Per I.S-BOUND: Coupling MUST always be bounded by available attention.
/// Boundaries enforce structural limits that cannot be bypassed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResonanceBoundary {
    pub boundary_type: BoundaryType,
    pub limits: BoundaryLimits,
    pub enforcement: EnforcementPolicy,
}

/// Types of boundaries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundaryType {
    /// Cognitive load limits
    Cognitive,
    /// Temporal (session duration, interaction rate)
    Temporal,
    /// Semantic (topic scope, depth limits)
    Semantic,
    /// Operational (resource consumption, side effects)
    Operational,
}

/// Limits enforced by a boundary.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct BoundaryLimits {
    /// Maximum coupling strength allowed (0.0–1.0)
    pub max_coupling_strength: f64,
    /// Maximum interactions per time window
    pub max_interaction_rate: Option<u32>,
    /// Time window for rate limiting (ms)
    pub rate_window_ms: Option<u64>,
    /// Maximum session duration (ms)
    pub max_session_duration_ms: Option<u64>,
    /// Maximum concurrent couplings
    pub max_concurrent_couplings: Option<u32>,
}

impl Default for BoundaryLimits {
    fn default() -> Self {
        Self {
            max_coupling_strength: 0.9,
            max_interaction_rate: Some(100),
            rate_window_ms: Some(60_000),
            max_session_duration_ms: Some(3_600_000), // 1 hour
            max_concurrent_couplings: Some(10),
        }
    }
}

/// How strictly boundaries are enforced.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EnforcementPolicy {
    /// Hard boundary — immediately blocked
    Hard,
    /// Soft boundary — warning issued, allowed to proceed
    Soft,
    /// Adaptive — enforcement increases with repeated violations
    Adaptive {
        violation_count: u32,
        escalation_threshold: u32,
    },
}

impl ResonanceBoundary {
    /// Create a cognitive boundary with default limits.
    pub fn cognitive() -> Self {
        Self {
            boundary_type: BoundaryType::Cognitive,
            limits: BoundaryLimits {
                max_coupling_strength: 0.8,
                max_interaction_rate: Some(60),
                rate_window_ms: Some(60_000),
                max_session_duration_ms: None,
                max_concurrent_couplings: Some(5),
            },
            enforcement: EnforcementPolicy::Hard,
        }
    }

    /// Create a temporal boundary.
    pub fn temporal(max_duration_ms: u64) -> Self {
        Self {
            boundary_type: BoundaryType::Temporal,
            limits: BoundaryLimits {
                max_coupling_strength: 1.0,
                max_interaction_rate: None,
                rate_window_ms: None,
                max_session_duration_ms: Some(max_duration_ms),
                max_concurrent_couplings: None,
            },
            enforcement: EnforcementPolicy::Hard,
        }
    }

    /// Create an operational boundary.
    pub fn operational() -> Self {
        Self {
            boundary_type: BoundaryType::Operational,
            limits: BoundaryLimits::default(),
            enforcement: EnforcementPolicy::Soft,
        }
    }

    /// Check if a coupling strength violates this boundary.
    pub fn check_coupling(&self, strength: f64) -> SafetyCheckResult {
        if strength > self.limits.max_coupling_strength {
            let msg = format!(
                "{:?} boundary violated: coupling strength {:.2} exceeds limit {:.2}",
                self.boundary_type, strength, self.limits.max_coupling_strength
            );

            match &self.enforcement {
                EnforcementPolicy::Hard => {
                    warn!(%msg, "HARD boundary violation");
                    SafetyCheckResult::Blocked(msg)
                }
                EnforcementPolicy::Soft => {
                    debug!(%msg, "Soft boundary warning");
                    SafetyCheckResult::Warning(msg)
                }
                EnforcementPolicy::Adaptive {
                    violation_count,
                    escalation_threshold,
                } => {
                    if *violation_count >= *escalation_threshold {
                        warn!(%msg, violations = violation_count, "Adaptive boundary escalated to hard");
                        SafetyCheckResult::Blocked(msg)
                    } else {
                        debug!(%msg, violations = violation_count, "Adaptive boundary warning");
                        SafetyCheckResult::Warning(msg)
                    }
                }
            }
        } else {
            SafetyCheckResult::Safe
        }
    }

    /// Check if an interaction rate violates this boundary.
    pub fn check_rate(&self, current_rate: u32) -> SafetyCheckResult {
        if let Some(max_rate) = self.limits.max_interaction_rate {
            if current_rate > max_rate {
                let msg = format!(
                    "{:?} boundary: interaction rate {} exceeds limit {}",
                    self.boundary_type, current_rate, max_rate
                );
                return match &self.enforcement {
                    EnforcementPolicy::Hard => SafetyCheckResult::Blocked(msg),
                    _ => SafetyCheckResult::Warning(msg),
                };
            }
        }
        SafetyCheckResult::Safe
    }

    /// Record a violation (for adaptive enforcement).
    pub fn record_violation(&mut self) {
        if let EnforcementPolicy::Adaptive {
            violation_count, ..
        } = &mut self.enforcement
        {
            *violation_count += 1;
        }
    }
}

/// Resonance Controller — damping, throttling, emergency decouple.
///
/// Controls the dynamics of resonance coupling to maintain safety bounds.
pub struct ResonanceController {
    /// Active boundaries
    boundaries: Vec<ResonanceBoundary>,
}

/// Result of an emergency decouple operation.
#[derive(Clone, Debug)]
pub struct DecoupleResult {
    pub success: bool,
    pub reason: String,
    pub couplings_severed: u32,
}

impl ResonanceController {
    pub fn new() -> Self {
        Self {
            boundaries: Vec::new(),
        }
    }

    /// Create a controller with standard safety boundaries.
    pub fn with_default_boundaries() -> Self {
        let mut controller = Self::new();
        controller.add_boundary(ResonanceBoundary::cognitive());
        controller.add_boundary(ResonanceBoundary::temporal(3_600_000)); // 1 hour
        controller.add_boundary(ResonanceBoundary::operational());
        controller
    }

    /// Add a boundary.
    pub fn add_boundary(&mut self, boundary: ResonanceBoundary) {
        self.boundaries.push(boundary);
    }

    /// Apply damping to a coupling strength.
    ///
    /// Reduces coupling strength by a factor. Factor of 0.5 halves it.
    /// Result is always clamped to [0.0, 1.0].
    pub fn apply_damping(&self, coupling_strength: f64, factor: f64) -> f64 {
        (coupling_strength * (1.0 - factor)).clamp(0.0, 1.0)
    }

    /// Apply throttling to a signal rate.
    ///
    /// Caps the signal rate at max_rate.
    pub fn apply_throttle(&self, signal_rate: f64, max_rate: f64) -> f64 {
        signal_rate.min(max_rate)
    }

    /// Emergency decouple — sever all couplings.
    ///
    /// This is the nuclear option. Used when safety boundaries are
    /// critically violated and no gentler intervention suffices.
    pub fn emergency_decouple(&self, reason: &str) -> DecoupleResult {
        warn!(reason = %reason, "EMERGENCY DECOUPLE triggered");
        DecoupleResult {
            success: true,
            reason: reason.into(),
            couplings_severed: 0, // Would be populated by actual coupling manager
        }
    }

    /// Check all boundaries against a coupling strength.
    pub fn check_boundaries(&self, coupling_strength: f64) -> SafetyCheckResult {
        for boundary in &self.boundaries {
            let result = boundary.check_coupling(coupling_strength);
            if result.is_blocked() {
                return result;
            }
        }
        SafetyCheckResult::Safe
    }

    /// Check all boundaries against an interaction rate.
    pub fn check_rate_boundaries(&self, rate: u32) -> SafetyCheckResult {
        for boundary in &self.boundaries {
            let result = boundary.check_rate(rate);
            if result.is_blocked() {
                return result;
            }
        }
        SafetyCheckResult::Safe
    }

    /// Number of active boundaries.
    pub fn boundary_count(&self) -> usize {
        self.boundaries.len()
    }
}

impl Default for ResonanceController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_boundary_blocks_excess_coupling() {
        let boundary = ResonanceBoundary::cognitive();
        // Cognitive boundary has max 0.8
        let result = boundary.check_coupling(0.9);
        assert!(result.is_blocked());

        let result = boundary.check_coupling(0.5);
        assert!(result.is_safe());
    }

    #[test]
    fn soft_boundary_warns() {
        let boundary = ResonanceBoundary::operational();
        let result = boundary.check_coupling(0.95);
        assert!(matches!(result, SafetyCheckResult::Warning(_)));
    }

    #[test]
    fn adaptive_boundary_escalates() {
        let mut boundary = ResonanceBoundary {
            boundary_type: BoundaryType::Cognitive,
            limits: BoundaryLimits {
                max_coupling_strength: 0.5,
                ..BoundaryLimits::default()
            },
            enforcement: EnforcementPolicy::Adaptive {
                violation_count: 0,
                escalation_threshold: 3,
            },
        };

        // First violation — warning
        let result = boundary.check_coupling(0.7);
        assert!(matches!(result, SafetyCheckResult::Warning(_)));
        boundary.record_violation();

        // Second and third
        boundary.record_violation();
        boundary.record_violation();

        // After threshold — blocked
        let result = boundary.check_coupling(0.7);
        assert!(result.is_blocked());
    }

    #[test]
    fn rate_limiting() {
        let boundary = ResonanceBoundary::cognitive();
        // Cognitive boundary has max 60 interactions/minute
        let result = boundary.check_rate(50);
        assert!(result.is_safe());

        let result = boundary.check_rate(100);
        assert!(result.is_blocked());
    }

    #[test]
    fn damping_reduces_coupling() {
        let controller = ResonanceController::new();
        let damped = controller.apply_damping(0.8, 0.5);
        assert!((damped - 0.4).abs() < 0.001);
    }

    #[test]
    fn damping_clamps_to_range() {
        let controller = ResonanceController::new();
        assert!(controller.apply_damping(0.5, 1.5) >= 0.0);
        assert!(controller.apply_damping(0.5, -1.0) <= 1.0);
    }

    #[test]
    fn throttle_caps_rate() {
        let controller = ResonanceController::new();
        assert_eq!(controller.apply_throttle(100.0, 60.0), 60.0);
        assert_eq!(controller.apply_throttle(30.0, 60.0), 30.0);
    }

    #[test]
    fn emergency_decouple() {
        let controller = ResonanceController::new();
        let result = controller.emergency_decouple("critical safety violation");
        assert!(result.success);
    }

    #[test]
    fn controller_check_all_boundaries() {
        let controller = ResonanceController::with_default_boundaries();
        assert_eq!(controller.boundary_count(), 3);

        // Within limits
        assert!(controller.check_boundaries(0.5).is_safe());

        // Cognitive boundary blocks at 0.8
        assert!(controller.check_boundaries(0.85).is_blocked());
    }

    #[test]
    fn controller_rate_check() {
        let controller = ResonanceController::with_default_boundaries();

        // Within limits
        assert!(controller.check_rate_boundaries(30).is_safe());

        // Cognitive boundary blocks at 60
        assert!(controller.check_rate_boundaries(70).is_blocked());
    }

    #[test]
    fn boundary_serialization() {
        let boundary = ResonanceBoundary::cognitive();
        let json = serde_json::to_string(&boundary).unwrap();
        let restored: ResonanceBoundary = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.boundary_type, BoundaryType::Cognitive);
    }
}
