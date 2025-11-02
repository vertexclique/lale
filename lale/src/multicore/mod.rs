//! Multi-core schedulability analysis
//!
//! Provides schedulability analysis for actor systems on multi-core platforms.

pub mod schedulability;

pub use schedulability::{
    CoreSchedulabilityResult, DeadlineViolation, MultiCoreResult, MultiCoreScheduler,
};
