//! Async actor analysis for Veecle OS
//!
//! This module provides detection and analysis of Rust async functions
//! compiled to LLVM IR, enabling WCET analysis for actor-based systems.

pub mod actor;
pub mod config;
pub mod detector;
pub mod inkwell_detector;
pub mod segment;
pub mod wcet;

pub use actor::{Actor, ActorConfig, ActorSystem};
pub use config::{
    ActorConfigEntry, ActorConfigLoader, ActorSystemConfig, SchedulingPolicy, VeecleActor,
    VeecleMetadata, VeecleModel, VeecleService,
};
pub use detector::{AsyncDetector, AsyncFunctionInfo, DetectionMethod, StateBlock};
pub use inkwell_detector::InkwellAsyncDetector;
pub use segment::{ActorSegment, SegmentExtractor, SegmentType};
pub use wcet::{SegmentWCET, SegmentWCETAnalyzer};
