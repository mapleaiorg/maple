//! PALM Platform Pack Conformance Test Suite
//!
//! This crate provides a comprehensive test framework to validate that
//! platform pack implementations correctly satisfy the Platform Contract.
//!
//! # Conformance Levels
//!
//! - **Core Conformance**: Required for all packs
//!   - Trait implementation completeness
//!   - Configuration validity
//!   - Capability consistency
//!
//! - **Behavioral Conformance**: Runtime behavior validation
//!   - Lifecycle callbacks
//!   - Agent spec validation
//!
//! - **Platform-Specific Conformance**: Profile-specific requirements
//!   - Mapleverse: Throughput characteristics
//!   - Finalverse: Safety requirements
//!   - iBank: Accountability requirements
//!
//! # Example
//!
//! ```rust,ignore
//! use palm_conformance::{ConformanceRunner, ConformanceConfig};
//! use std::sync::Arc;
//!
//! let config = ConformanceConfig::default();
//! let runner = ConformanceRunner::new(config);
//! let report = runner.run(pack).await;
//! println!("{}", report.to_text());
//! ```

pub mod behavioral_tests;
pub mod core_tests;
pub mod framework;
pub mod harness;
pub mod platform_tests;
pub mod reports;

pub use framework::{ConformanceConfig, ConformanceRunner};
pub use harness::TestHarness;
pub use reports::{ConformanceReport, ReportSummary, TestCategory, TestResult, TestStatus};
