use crate::analysis::{IPETSolver, LoopAnalyzer, TimingCalculator};
use crate::ir::{IRParser, CFG};
use crate::platform::PlatformModel;
use std::path::Path;

/// WCET calculator
pub struct WCETCalculator {
    platform: PlatformModel,
}

impl WCETCalculator {
    /// Create new WCET calculator with platform model
    pub fn new(platform: PlatformModel) -> Self {
        Self { platform }
    }

    /// Calculate WCET for a function from LLVM IR file
    pub fn calculate_wcet_from_file<P: AsRef<Path>>(
        &self,
        ir_path: P,
        function_name: &str,
    ) -> Result<u64, String> {
        // Parse LLVM IR
        let module =
            IRParser::parse_file(ir_path).map_err(|e| format!("Failed to parse IR: {}", e))?;

        // Find function
        let function = module
            .functions
            .iter()
            .find(|f| f.name == function_name)
            .ok_or_else(|| format!("Function '{}' not found", function_name))?;

        self.calculate_wcet_for_function(function)
    }

    /// Calculate WCET for a function
    pub fn calculate_wcet_for_function(&self, function: &llvm_ir::Function) -> Result<u64, String> {
        // Build CFG
        let cfg = CFG::from_function(function);

        // Analyze loops
        let loops = LoopAnalyzer::analyze_loops(&cfg);

        // Calculate instruction timing
        let timings = TimingCalculator::calculate_block_timings(function, &cfg, &self.platform);

        // Solve IPET to get WCET
        IPETSolver::solve_wcet(&cfg, &timings, &loops)
    }

    /// Calculate WCET for all functions in a module
    pub fn calculate_wcet_for_module(
        &self,
        module: &llvm_ir::Module,
    ) -> ahash::AHashMap<String, u64> {
        let mut results = ahash::AHashMap::new();

        for function in &module.functions {
            if let Ok(wcet) = self.calculate_wcet_for_function(function) {
                results.insert(function.name.to_string(), wcet);
            }
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::CortexM4Model;

    #[test]
    fn test_wcet_calculator() {
        let platform = CortexM4Model::new();
        let calculator = WCETCalculator::new(platform);

        let sample_path = "data/armv7e-m/56e3741adeae4068.ll";
        if std::path::Path::new(sample_path).exists() {
            let module = IRParser::parse_file(sample_path).unwrap();
            let results = calculator.calculate_wcet_for_module(&module);

            assert!(!results.is_empty(), "Should calculate WCET for functions");
        }
    }
}
