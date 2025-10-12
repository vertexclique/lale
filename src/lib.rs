pub mod analysis;
pub mod ir;
pub mod output;
pub mod platform;
pub mod scheduling;
pub mod wcet;

// Re-export commonly used types
pub use analysis::{Cycles, IPETSolver, LoopAnalyzer, TimingCalculator};
pub use ir::{CallGraph, IRParser, CFG};
pub use output::{AnalysisReport, GanttOutput, GraphvizOutput, JSONOutput};
pub use platform::{CortexM4Model, PlatformModel};
pub use scheduling::{
    EDFScheduler, RMAScheduler, SchedulabilityResult, StaticScheduleGenerator, Task, TaskExtractor,
};

/// LALE version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Complete WCET analysis pipeline
pub struct WCETAnalyzer {
    platform: PlatformModel,
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
    pub fn analyze_module(
        &self,
        module: &llvm_ir::Module,
    ) -> ahash::AHashMap<String, u64> {
        let mut results = ahash::AHashMap::new();

        for function in &module.functions {
            if let Ok(wcet) = self.analyze_function(function) {
                results.insert(function.name.to_string(), wcet);
            }
        }

        results
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
