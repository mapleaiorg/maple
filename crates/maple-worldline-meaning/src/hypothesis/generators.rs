//! Built-in hypothesis generators for the meaning formation engine.
//!
//! Six generators propose interpretations from different perspectives:
//! 1. **ComponentIsolation** — Single-component root cause analysis
//! 2. **InteractionPattern** — Cross-component interaction issues
//! 3. **ResourcePressure** — Resource exhaustion / capacity concerns
//! 4. **CodePath** — Hot path / dead code optimization targets
//! 5. **HistoricalPattern** — Recurring issue detection
//! 6. **EnvironmentalChange** — External environment shift detection

use chrono::Utc;

use maple_worldline_observation::{AnomalyCategory, AnomalySeverity, PerformanceAnomaly};

use crate::types::{
    ArchitecturalInsightType, Evidence, EvidenceCategory, GrowthModel, HypothesisId,
    MemoryOptimizationType, OperatorBottleneckType, RedundancyType, RootCauseHypothesis,
    SelfMeaning, SelfMeaningCategory,
};

use super::types::{Hypothesis, HypothesisGenerator, ObservationSummary};

// ── 1. Component Isolation Generator ────────────────────────────────────

/// Generates hypotheses that isolate problems to a single component.
///
/// For each anomaly, this generator proposes that the anomaly's component
/// is the root cause (PerformanceBottleneck or OperatorOptimization).
pub struct ComponentIsolationGenerator;

impl HypothesisGenerator for ComponentIsolationGenerator {
    fn generate(
        &self,
        anomalies: &[PerformanceAnomaly],
        _summary: &ObservationSummary,
        _history: &[SelfMeaning],
    ) -> Vec<Hypothesis> {
        anomalies
            .iter()
            .filter_map(|anomaly| {
                let severity = match anomaly.severity {
                    AnomalySeverity::Critical => 0.9,
                    AnomalySeverity::Warning => 0.6,
                    AnomalySeverity::Info => 0.3,
                };
                let category = match &anomaly.category {
                    AnomalyCategory::LatencyRegression
                    | AnomalyCategory::ThroughputDegradation
                    | AnomalyCategory::ErrorRateSpike => {
                        SelfMeaningCategory::PerformanceBottleneck {
                            component: anomaly.component.0.clone(),
                            severity,
                            root_causes: vec![RootCauseHypothesis {
                                description: format!(
                                    "{} in {}",
                                    anomaly.category, anomaly.component
                                ),
                                confidence: anomaly.score,
                                supporting_evidence: vec![anomaly.description.clone()],
                            }],
                        }
                    }
                    AnomalyCategory::OperatorBottleneck => {
                        SelfMeaningCategory::OperatorOptimization {
                            operator_id: anomaly.component.0.clone(),
                            bottleneck_type: OperatorBottleneckType::CpuBound,
                            improvement_hypothesis: format!(
                                "Operator {} is consistently underperforming",
                                anomaly.component
                            ),
                        }
                    }
                    _ => return None,
                };

                Some(Hypothesis {
                    id: HypothesisId::new(),
                    meaning_category: category,
                    confidence: anomaly.score * 0.7,
                    evidence: vec![Evidence {
                        source: format!("anomaly:{}", anomaly.id),
                        strength: anomaly.score,
                        timestamp: anomaly.detected_at,
                        description: anomaly.description.clone(),
                        category: EvidenceCategory::Anomaly,
                    }],
                    description: format!(
                        "Component {} shows {} (score {:.2})",
                        anomaly.component, anomaly.category, anomaly.score
                    ),
                    generator_name: self.name().into(),
                    created_at: Utc::now(),
                })
            })
            .collect()
    }

    fn name(&self) -> &str {
        "component-isolation"
    }
}

// ── 2. Interaction Pattern Generator ────────────────────────────────────

/// Generates hypotheses about cross-component interaction issues.
///
/// When ≥2 anomalies affect different components within temporal proximity,
/// this generator hypothesizes an architectural interaction problem.
pub struct InteractionPatternGenerator;

impl HypothesisGenerator for InteractionPatternGenerator {
    fn generate(
        &self,
        anomalies: &[PerformanceAnomaly],
        _summary: &ObservationSummary,
        _history: &[SelfMeaning],
    ) -> Vec<Hypothesis> {
        if anomalies.len() < 2 {
            return vec![];
        }

        // Group anomalies by component
        let mut by_component: std::collections::HashMap<String, Vec<&PerformanceAnomaly>> =
            std::collections::HashMap::new();
        for a in anomalies {
            by_component
                .entry(a.component.0.clone())
                .or_default()
                .push(a);
        }

        // If multiple components are affected, hypothesize interaction issue
        if by_component.len() >= 2 {
            let affected: Vec<String> = by_component.keys().cloned().collect();
            let avg_score =
                anomalies.iter().map(|a| a.score).sum::<f64>() / anomalies.len() as f64;

            let evidence: Vec<Evidence> = anomalies
                .iter()
                .take(5)
                .map(|a| Evidence {
                    source: format!("anomaly:{}", a.id),
                    strength: a.score,
                    timestamp: a.detected_at,
                    description: format!("{} in {}", a.category, a.component),
                    category: EvidenceCategory::Correlation,
                })
                .collect();

            vec![Hypothesis {
                id: HypothesisId::new(),
                meaning_category: SelfMeaningCategory::ArchitecturalInsight {
                    insight_type: ArchitecturalInsightType::CouplingTooTight,
                    affected_components: affected.clone(),
                    structural_pressure: format!(
                        "Correlated anomalies across {} components",
                        affected.len()
                    ),
                },
                confidence: avg_score * 0.5,
                evidence,
                description: format!(
                    "Correlated anomalies across components: {}",
                    affected.join(", ")
                ),
                generator_name: self.name().into(),
                created_at: Utc::now(),
            }]
        } else {
            vec![]
        }
    }

    fn name(&self) -> &str {
        "interaction-pattern"
    }
}

// ── 3. Resource Pressure Generator ──────────────────────────────────────

/// Generates hypotheses about resource exhaustion and capacity issues.
///
/// Maps ResourceExhaustion and MemoryLeak anomalies to CapacityForecast
/// or MemoryOptimization meanings.
pub struct ResourcePressureGenerator;

impl HypothesisGenerator for ResourcePressureGenerator {
    fn generate(
        &self,
        anomalies: &[PerformanceAnomaly],
        _summary: &ObservationSummary,
        _history: &[SelfMeaning],
    ) -> Vec<Hypothesis> {
        anomalies
            .iter()
            .filter_map(|anomaly| {
                let category = match &anomaly.category {
                    AnomalyCategory::ResourceExhaustion => {
                        SelfMeaningCategory::CapacityForecast {
                            resource: anomaly.component.0.clone(),
                            current_utilization: anomaly.observed_value
                                / anomaly.baseline_mean.max(1.0),
                            projected_exhaustion_hours: if anomaly.severity
                                == AnomalySeverity::Critical
                            {
                                Some(2.0)
                            } else {
                                Some(24.0)
                            },
                            growth_model: GrowthModel::Linear,
                        }
                    }
                    AnomalyCategory::MemoryLeak => SelfMeaningCategory::MemoryOptimization {
                        component: anomaly.component.0.clone(),
                        optimization_type: MemoryOptimizationType::EvictionTuning,
                        estimated_improvement: 0.3,
                    },
                    _ => return None,
                };

                Some(Hypothesis {
                    id: HypothesisId::new(),
                    meaning_category: category,
                    confidence: anomaly.score * 0.8,
                    evidence: vec![Evidence {
                        source: format!("anomaly:{}", anomaly.id),
                        strength: anomaly.score,
                        timestamp: anomaly.detected_at,
                        description: anomaly.description.clone(),
                        category: EvidenceCategory::Anomaly,
                    }],
                    description: format!(
                        "Resource pressure detected in {}: {}",
                        anomaly.component, anomaly.category
                    ),
                    generator_name: self.name().into(),
                    created_at: Utc::now(),
                })
            })
            .collect()
    }

    fn name(&self) -> &str {
        "resource-pressure"
    }
}

// ── 4. Code Path Generator ──────────────────────────────────────────────

/// Generates hypotheses about code path optimization opportunities.
///
/// Maps HotPath anomalies to OperatorOptimization and ColdCode anomalies
/// to CodeSimplification meanings.
pub struct CodePathGenerator;

impl HypothesisGenerator for CodePathGenerator {
    fn generate(
        &self,
        anomalies: &[PerformanceAnomaly],
        _summary: &ObservationSummary,
        _history: &[SelfMeaning],
    ) -> Vec<Hypothesis> {
        anomalies
            .iter()
            .filter_map(|anomaly| {
                let category = match &anomaly.category {
                    AnomalyCategory::HotPath => SelfMeaningCategory::OperatorOptimization {
                        operator_id: anomaly.component.0.clone(),
                        bottleneck_type: OperatorBottleneckType::AlgorithmicComplexity,
                        improvement_hypothesis: format!(
                            "Hot path in {} executing {:.1}x more than baseline",
                            anomaly.component,
                            anomaly.observed_value / anomaly.baseline_mean.max(1.0)
                        ),
                    },
                    AnomalyCategory::ColdCode => SelfMeaningCategory::CodeSimplification {
                        component: anomaly.component.0.clone(),
                        dead_paths: vec![anomaly.metric_id.to_string()],
                        redundancy_type: RedundancyType::DeadCode,
                    },
                    _ => return None,
                };

                Some(Hypothesis {
                    id: HypothesisId::new(),
                    meaning_category: category,
                    confidence: anomaly.score * 0.6,
                    evidence: vec![Evidence {
                        source: format!("anomaly:{}", anomaly.id),
                        strength: anomaly.score,
                        timestamp: anomaly.detected_at,
                        description: anomaly.description.clone(),
                        category: EvidenceCategory::Observation,
                    }],
                    description: format!(
                        "Code path optimization target in {}: {}",
                        anomaly.component, anomaly.category
                    ),
                    generator_name: self.name().into(),
                    created_at: Utc::now(),
                })
            })
            .collect()
    }

    fn name(&self) -> &str {
        "code-path"
    }
}

// ── 5. Historical Pattern Generator ─────────────────────────────────────

/// Generates hypotheses by comparing current anomalies against past meanings.
///
/// If a similar meaning was formed before, this generator boosts confidence
/// and proposes a SystematicError interpretation.
pub struct HistoricalPatternGenerator;

impl HypothesisGenerator for HistoricalPatternGenerator {
    fn generate(
        &self,
        anomalies: &[PerformanceAnomaly],
        _summary: &ObservationSummary,
        history: &[SelfMeaning],
    ) -> Vec<Hypothesis> {
        if history.is_empty() {
            return vec![];
        }

        anomalies
            .iter()
            .filter_map(|anomaly| {
                // Check if any historical meaning matches this component + category
                let matching_history: Vec<&SelfMeaning> = history
                    .iter()
                    .filter(|m| {
                        m.category.primary_component()
                            == Some(anomaly.component.0.as_str())
                    })
                    .collect();

                if matching_history.is_empty() {
                    return None;
                }

                let recurrence_count = matching_history.len();
                let boosted_confidence =
                    (anomaly.score * 0.8 + 0.1 * recurrence_count as f64).min(0.95);

                Some(Hypothesis {
                    id: HypothesisId::new(),
                    meaning_category: SelfMeaningCategory::SystematicError {
                        error_pattern: format!("{} in {}", anomaly.category, anomaly.component),
                        frequency: anomaly.score,
                        affected_operations: vec![anomaly.component.0.clone()],
                        root_causes: vec![RootCauseHypothesis {
                            description: format!(
                                "Recurring issue: seen {} time(s) before",
                                recurrence_count
                            ),
                            confidence: boosted_confidence,
                            supporting_evidence: vec![anomaly.description.clone()],
                        }],
                    },
                    confidence: boosted_confidence,
                    evidence: vec![Evidence {
                        source: format!("anomaly:{}", anomaly.id),
                        strength: anomaly.score,
                        timestamp: anomaly.detected_at,
                        description: format!(
                            "Matches {} historical pattern(s) in {}",
                            recurrence_count, anomaly.component
                        ),
                        category: EvidenceCategory::Historical,
                    }],
                    description: format!(
                        "Recurring pattern: {} in {} (seen {} times)",
                        anomaly.category, anomaly.component, recurrence_count
                    ),
                    generator_name: self.name().into(),
                    created_at: Utc::now(),
                })
            })
            .collect()
    }

    fn name(&self) -> &str {
        "historical-pattern"
    }
}

// ── 6. Environmental Change Generator ───────────────────────────────────

/// Generates hypotheses about external environment changes.
///
/// When many anomalies (≥3) correlate across components in the same window,
/// this generator hypothesizes an external environmental shift rather than
/// an internal issue.
pub struct EnvironmentalChangeGenerator;

impl EnvironmentalChangeGenerator {
    /// Minimum number of correlated anomalies to trigger environmental hypothesis.
    const MIN_CORRELATED_ANOMALIES: usize = 3;
}

impl HypothesisGenerator for EnvironmentalChangeGenerator {
    fn generate(
        &self,
        anomalies: &[PerformanceAnomaly],
        _summary: &ObservationSummary,
        _history: &[SelfMeaning],
    ) -> Vec<Hypothesis> {
        if anomalies.len() < Self::MIN_CORRELATED_ANOMALIES {
            return vec![];
        }

        // Count distinct components affected
        let distinct_components: std::collections::HashSet<&str> =
            anomalies.iter().map(|a| a.component.0.as_str()).collect();

        if distinct_components.len() < 2 {
            return vec![];
        }

        let affected: Vec<String> = distinct_components.iter().map(|s| s.to_string()).collect();
        let avg_score =
            anomalies.iter().map(|a| a.score).sum::<f64>() / anomalies.len() as f64;

        let evidence: Vec<Evidence> = anomalies
            .iter()
            .take(5)
            .map(|a| Evidence {
                source: format!("anomaly:{}", a.id),
                strength: a.score,
                timestamp: a.detected_at,
                description: format!("{} in {}", a.category, a.component),
                category: EvidenceCategory::Correlation,
            })
            .collect();

        vec![Hypothesis {
            id: HypothesisId::new(),
            meaning_category: SelfMeaningCategory::ArchitecturalInsight {
                insight_type: ArchitecturalInsightType::MissingAbstraction,
                affected_components: affected.clone(),
                structural_pressure: format!(
                    "Broad anomaly pattern across {} components suggests environmental change",
                    affected.len()
                ),
            },
            confidence: avg_score * 0.4,
            evidence,
            description: format!(
                "Possible environmental change: {} anomalies across {} components",
                anomalies.len(),
                affected.len()
            ),
            generator_name: self.name().into(),
            created_at: Utc::now(),
        }]
    }

    fn name(&self) -> &str {
        "environmental-change"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MeaningId;
    use maple_worldline_observation::{AnomalyId, ComponentId, MetricId};

    fn make_anomaly(
        component: &str,
        category: AnomalyCategory,
        severity: AnomalySeverity,
        score: f64,
    ) -> PerformanceAnomaly {
        PerformanceAnomaly {
            id: AnomalyId::new(),
            category,
            severity,
            component: ComponentId(component.into()),
            metric_id: MetricId::new(component, "latency_ns"),
            description: format!("test anomaly in {}", component),
            score,
            detector_agreement: 0.8,
            detected_at: Utc::now(),
            baseline_mean: 5_000_000.0,
            observed_value: 25_000_000.0,
            detectors: vec!["statistical".into()],
        }
    }

    fn empty_summary() -> ObservationSummary {
        ObservationSummary {
            total_events: 0,
            subsystem_summaries: std::collections::HashMap::new(),
            snapshot_timestamp: Utc::now(),
        }
    }

    #[test]
    fn component_isolation_generates_bottleneck() {
        let gen = ComponentIsolationGenerator;
        let anomalies = vec![make_anomaly(
            "event-fabric",
            AnomalyCategory::LatencyRegression,
            AnomalySeverity::Warning,
            0.8,
        )];
        let hyps = gen.generate(&anomalies, &empty_summary(), &[]);
        assert_eq!(hyps.len(), 1);
        assert_eq!(hyps[0].generator_name, "component-isolation");
        assert!(matches!(
            hyps[0].meaning_category,
            SelfMeaningCategory::PerformanceBottleneck { .. }
        ));
    }

    #[test]
    fn component_isolation_skips_irrelevant() {
        let gen = ComponentIsolationGenerator;
        let anomalies = vec![make_anomaly(
            "memory",
            AnomalyCategory::MemoryLeak,
            AnomalySeverity::Warning,
            0.7,
        )];
        let hyps = gen.generate(&anomalies, &empty_summary(), &[]);
        assert!(hyps.is_empty(), "MemoryLeak should not produce bottleneck");
    }

    #[test]
    fn interaction_pattern_requires_multiple_components() {
        let gen = InteractionPatternGenerator;

        // Single component — no hypothesis
        let single = vec![make_anomaly(
            "gate",
            AnomalyCategory::LatencyRegression,
            AnomalySeverity::Warning,
            0.7,
        )];
        assert!(gen.generate(&single, &empty_summary(), &[]).is_empty());

        // Multiple components — hypothesis generated
        let multi = vec![
            make_anomaly(
                "gate",
                AnomalyCategory::LatencyRegression,
                AnomalySeverity::Warning,
                0.7,
            ),
            make_anomaly(
                "fabric",
                AnomalyCategory::ThroughputDegradation,
                AnomalySeverity::Warning,
                0.6,
            ),
        ];
        let hyps = gen.generate(&multi, &empty_summary(), &[]);
        assert_eq!(hyps.len(), 1);
        assert!(matches!(
            hyps[0].meaning_category,
            SelfMeaningCategory::ArchitecturalInsight { .. }
        ));
    }

    #[test]
    fn resource_pressure_maps_resource_exhaustion() {
        let gen = ResourcePressureGenerator;
        let anomalies = vec![make_anomaly(
            "memory-engine",
            AnomalyCategory::ResourceExhaustion,
            AnomalySeverity::Critical,
            0.9,
        )];
        let hyps = gen.generate(&anomalies, &empty_summary(), &[]);
        assert_eq!(hyps.len(), 1);
        assert!(matches!(
            hyps[0].meaning_category,
            SelfMeaningCategory::CapacityForecast { .. }
        ));
    }

    #[test]
    fn resource_pressure_maps_memory_leak() {
        let gen = ResourcePressureGenerator;
        let anomalies = vec![make_anomaly(
            "memory-engine",
            AnomalyCategory::MemoryLeak,
            AnomalySeverity::Warning,
            0.7,
        )];
        let hyps = gen.generate(&anomalies, &empty_summary(), &[]);
        assert_eq!(hyps.len(), 1);
        assert!(matches!(
            hyps[0].meaning_category,
            SelfMeaningCategory::MemoryOptimization { .. }
        ));
    }

    #[test]
    fn code_path_generates_hot_path() {
        let gen = CodePathGenerator;
        let anomalies = vec![make_anomaly(
            "operator-bus",
            AnomalyCategory::HotPath,
            AnomalySeverity::Info,
            0.5,
        )];
        let hyps = gen.generate(&anomalies, &empty_summary(), &[]);
        assert_eq!(hyps.len(), 1);
        assert!(matches!(
            hyps[0].meaning_category,
            SelfMeaningCategory::OperatorOptimization { .. }
        ));
    }

    #[test]
    fn code_path_generates_dead_code() {
        let gen = CodePathGenerator;
        let anomalies = vec![make_anomaly(
            "operator-bus",
            AnomalyCategory::ColdCode,
            AnomalySeverity::Info,
            0.4,
        )];
        let hyps = gen.generate(&anomalies, &empty_summary(), &[]);
        assert_eq!(hyps.len(), 1);
        assert!(matches!(
            hyps[0].meaning_category,
            SelfMeaningCategory::CodeSimplification { .. }
        ));
    }

    #[test]
    fn historical_pattern_requires_history() {
        let gen = HistoricalPatternGenerator;
        let anomalies = vec![make_anomaly(
            "gate",
            AnomalyCategory::LatencyRegression,
            AnomalySeverity::Warning,
            0.7,
        )];

        // No history — no hypothesis
        assert!(gen.generate(&anomalies, &empty_summary(), &[]).is_empty());

        // With matching history
        let past = vec![SelfMeaning {
            id: MeaningId::new(),
            category: SelfMeaningCategory::PerformanceBottleneck {
                component: "gate".into(),
                severity: 0.6,
                root_causes: vec![],
            },
            evidence: vec![],
            confidence: 0.7,
            ambiguity: 0.3,
            formed_at: Utc::now(),
            temporal_stability_secs: 3600.0,
            competing_with: vec![],
            converged: true,
        }];
        let hyps = gen.generate(&anomalies, &empty_summary(), &past);
        assert_eq!(hyps.len(), 1);
        assert!(matches!(
            hyps[0].meaning_category,
            SelfMeaningCategory::SystematicError { .. }
        ));
    }

    #[test]
    fn historical_pattern_boosts_confidence() {
        let gen = HistoricalPatternGenerator;
        let anomalies = vec![make_anomaly(
            "gate",
            AnomalyCategory::LatencyRegression,
            AnomalySeverity::Warning,
            0.5,
        )];
        let past: Vec<SelfMeaning> = (0..3)
            .map(|_| SelfMeaning {
                id: MeaningId::new(),
                category: SelfMeaningCategory::PerformanceBottleneck {
                    component: "gate".into(),
                    severity: 0.5,
                    root_causes: vec![],
                },
                evidence: vec![],
                confidence: 0.6,
                ambiguity: 0.4,
                formed_at: Utc::now(),
                temporal_stability_secs: 7200.0,
                competing_with: vec![],
                converged: true,
            })
            .collect();

        let hyps = gen.generate(&anomalies, &empty_summary(), &past);
        assert_eq!(hyps.len(), 1);
        // Confidence boosted by history: 0.5*0.8 + 0.1*3 = 0.7
        assert!(hyps[0].confidence > 0.5);
    }

    #[test]
    fn environmental_change_requires_threshold() {
        let gen = EnvironmentalChangeGenerator;

        // Below threshold — no hypothesis
        let few = vec![
            make_anomaly("a", AnomalyCategory::LatencyRegression, AnomalySeverity::Warning, 0.6),
            make_anomaly("b", AnomalyCategory::LatencyRegression, AnomalySeverity::Warning, 0.5),
        ];
        assert!(gen.generate(&few, &empty_summary(), &[]).is_empty());

        // Above threshold, multiple components — hypothesis generated
        let many = vec![
            make_anomaly("a", AnomalyCategory::LatencyRegression, AnomalySeverity::Warning, 0.6),
            make_anomaly("b", AnomalyCategory::ThroughputDegradation, AnomalySeverity::Warning, 0.5),
            make_anomaly("c", AnomalyCategory::ErrorRateSpike, AnomalySeverity::Critical, 0.8),
        ];
        let hyps = gen.generate(&many, &empty_summary(), &[]);
        assert_eq!(hyps.len(), 1);
        assert_eq!(hyps[0].generator_name, "environmental-change");
    }

    #[test]
    fn environmental_change_requires_distinct_components() {
        let gen = EnvironmentalChangeGenerator;
        // 3 anomalies but same component — no hypothesis
        let same = vec![
            make_anomaly("a", AnomalyCategory::LatencyRegression, AnomalySeverity::Warning, 0.6),
            make_anomaly("a", AnomalyCategory::ThroughputDegradation, AnomalySeverity::Warning, 0.5),
            make_anomaly("a", AnomalyCategory::ErrorRateSpike, AnomalySeverity::Warning, 0.4),
        ];
        assert!(gen.generate(&same, &empty_summary(), &[]).is_empty());
    }

    #[test]
    fn all_generators_have_distinct_names() {
        let generators: Vec<Box<dyn HypothesisGenerator>> = vec![
            Box::new(ComponentIsolationGenerator),
            Box::new(InteractionPatternGenerator),
            Box::new(ResourcePressureGenerator),
            Box::new(CodePathGenerator),
            Box::new(HistoricalPatternGenerator),
            Box::new(EnvironmentalChangeGenerator),
        ];
        let names: std::collections::HashSet<&str> =
            generators.iter().map(|g| g.name()).collect();
        assert_eq!(names.len(), 6);
    }
}
