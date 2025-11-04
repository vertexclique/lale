pub mod aeg;
pub mod analysis;
pub mod analyzers;
pub mod async_analysis;
pub mod config;
pub mod ir;
pub mod microarch;
pub mod multicore;
pub mod output;
pub mod platform;
pub mod scheduling;
pub mod wcet;

// Re-export commonly used types
pub use analysis::{Cycles, IPETSolver, LoopAnalyzer};
pub use analyzers::{
    ActorAnalyzer, DirectoryAnalysisResult, DirectoryAnalyzer, FunctionAnalysisResult,
    FunctionAnalyzer, ModuleAnalysisResult, ModuleAnalyzer,
};
pub use async_analysis::{
    Actor, ActorConfig, ActorConfigEntry, ActorConfigLoader, ActorSystem, ActorSystemConfig,
    AsyncFunctionInfo, InkwellAsyncDetector, InkwellSegmentExtractor, InkwellSegmentWCETAnalyzer,
    SchedulingPolicy, VeecleActor, VeecleMetadata, VeecleModel, VeecleService,
};
pub use ir::{InkwellCFG, InkwellParser};
pub use multicore::{
    CoreSchedulabilityResult, DeadlineViolation, MultiCoreResult, MultiCoreScheduler,
};
pub use output::{AnalysisReport, GanttOutput, GraphvizOutput, JSONOutput};
pub use platform::{
    CortexA53Model, CortexA7Model, CortexM0Model, CortexM33Model, CortexM3Model, CortexM4Model,
    CortexM7Model, CortexR4Model, CortexR5Model, PlatformModel, RV32GCModel, RV32IMACModel,
    RV32IModel, RV64GCModel,
};
pub use scheduling::{
    EDFScheduler, RMAScheduler, SchedulabilityResult, StaticScheduleGenerator, Task, TaskExtractor,
};

/// LALE version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}
