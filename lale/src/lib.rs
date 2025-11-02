pub mod aeg;
pub mod analysis;
pub mod async_analysis;
pub mod config;
pub mod ir;
pub mod microarch;
pub mod output;
pub mod platform;
pub mod scheduling;
pub mod wcet;

// Re-export commonly used types
pub use analysis::{Cycles, IPETSolver, LoopAnalyzer, TimingCalculator};
pub use async_analysis::{
    Actor, ActorConfig, ActorConfigEntry, ActorConfigLoader, ActorSegment, ActorSystem,
    ActorSystemConfig, AsyncDetector, AsyncFunctionInfo, DetectionMethod, InkwellAsyncDetector,
    SchedulingPolicy, SegmentExtractor, SegmentType, SegmentWCET, SegmentWCETAnalyzer, StateBlock,
    VeecleActor, VeecleMetadata, VeecleModel, VeecleService,
};
pub use ir::{CallGraph, IRParser, CFG};
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

/// Complete WCET analysis pipeline
pub struct WCETAnalyzer {
    pub platform: PlatformModel,
}

impl WCETAnalyzer {
    /// Create new analyzer with platform model
    pub fn new(platform: PlatformModel) -> Self {
        Self { platform }
    }

    /// Analyze a single function
    pub fn analyze_function(&self, function: &llvm_ir::Function) -> Result<u64, String> {
        // Build CFG
        let cfg = CFG::from_function(function);

        // Analyze loops
        let loops = LoopAnalyzer::analyze_loops(&cfg);

        // Calculate block timings
        let timings = TimingCalculator::calculate_block_timings(function, &cfg, &self.platform);

        // Solve WCET with IPET
        IPETSolver::solve_wcet(&cfg, &timings, &loops)
    }

    /// Analyze entire module
    pub fn analyze_module(&self, module: &llvm_ir::Module) -> ahash::AHashMap<String, u64> {
        let mut results = ahash::AHashMap::new();

        // Sort functions by name for deterministic order
        let mut functions: Vec<_> = module.functions.iter().collect();
        functions.sort_by_key(|f| &f.name);

        for function in functions {
            if let Ok(wcet) = self.analyze_function(function) {
                results.insert(function.name.to_string(), wcet);
            }
        }

        results
    }

    /// Complete analysis pipeline: WCET + Schedulability + Schedule generation
    /// Returns JSON string with complete analysis report including reusable schedule
    pub fn analyze_and_export_schedule(
        &self,
        module: &llvm_ir::Module,
        tasks: Vec<Task>,
        scheduling_policy: async_analysis::SchedulingPolicy,
    ) -> anyhow::Result<String> {
        // 1. Analyze WCET for all functions
        let wcet_results = self.analyze_module(module);

        // 2. Perform schedulability analysis
        let schedulability = match scheduling_policy {
            async_analysis::SchedulingPolicy::RMA => RMAScheduler::schedulability_test(&tasks),
            async_analysis::SchedulingPolicy::EDF => EDFScheduler::schedulability_test(&tasks),
        };

        // 3. Generate static schedule if schedulable
        let schedule = match schedulability {
            SchedulabilityResult::Schedulable => {
                Some(StaticScheduleGenerator::generate_schedule(&tasks))
            }
            _ => None,
        };

        // 4. Generate JSON report
        let report = JSONOutput::generate_report(
            &wcet_results,
            &tasks,
            &schedulability,
            schedule,
            &self.platform.name,
            self.platform.cpu_frequency_mhz,
        );

        // 5. Export to JSON string
        JSONOutput::to_json(&report).map_err(|e| e.into())
    }

    /// Export analysis report to JSON file
    pub fn export_to_file(
        &self,
        module: &llvm_ir::Module,
        tasks: Vec<Task>,
        scheduling_policy: async_analysis::SchedulingPolicy,
        output_path: &str,
    ) -> anyhow::Result<()> {
        let json = self.analyze_and_export_schedule(module, tasks, scheduling_policy)?;
        std::fs::write(output_path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_wcet_analyzer() {
        let platform = CortexM4Model::new();
        let analyzer = WCETAnalyzer::new(platform);

        let sample_path = "data/armv7e-m/56e3741adeae4068.ll";
        if std::path::Path::new(sample_path).exists() {
            let module = IRParser::parse_file(sample_path).unwrap();
            let results = analyzer.analyze_module(&module);

            assert!(!results.is_empty(), "Should analyze at least one function");
        }
    }
}
