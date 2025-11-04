//! Async actor analysis for Veecle OS
//!
//! This module provides detection and analysis of Rust async functions
//! compiled to LLVM IR, enabling WCET analysis for actor-based systems.

pub mod actor;
pub mod config;
pub mod inkwell_detector;
pub mod inkwell_segment;
pub mod inkwell_wcet;

pub use actor::{Actor, ActorConfig, ActorSystem};
pub use config::{
    ActorConfigEntry, ActorConfigLoader, ActorSystemConfig, SchedulingPolicy, VeecleActor,
    VeecleMetadata, VeecleModel, VeecleService,
};
pub use inkwell_detector::{AsyncFunctionInfo, DetectionMethod, InkwellAsyncDetector, StateBlock};
pub use inkwell_segment::{ActorSegment, InkwellSegmentExtractor, SegmentType};
pub use inkwell_wcet::{InkwellSegmentWCETAnalyzer, SegmentWCET};
