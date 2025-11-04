//! Module-level WCET analysis
//!
//! Analyzes all functions in an LLVM module.

use crate::analysis::InkwellTimingCalculator;
use crate::ir::{InkwellCFG, InkwellParser};
use crate::platform::PlatformModel;
use ahash::AHashMap;
use inkwell::module::Module;
use std::path::Path;

/// Result of analyzing a module
#[derive(Debug, Clone)]
pub struct ModuleAnalysisResult {
    /// WCET results per function (function_name -> wcet_cycles)
    pub function_wcets: AHashMap<String, u64>,

    /// Number of functions analyzed
    pub functions_analyzed: usize,

    /// Number of functions skipped (intrinsics, declarations)
    pub functions_skipped: usize,
}

/// Analyzer for LLVM modules
pub struct ModuleAnalyzer {
    platform: PlatformModel,
}

impl ModuleAnalyzer {
    /// Create a new module analyzer with the given platform
    pub fn new(platform: PlatformModel) -> Self {
        Self { platform }
    }

    /// Analyze all functions in a module from file
    pub fn analyze_file(&self, path: impl AsRef<Path>) -> Result<ModuleAnalysisResult, String> {
        let (_context, module) = InkwellParser::parse_file(path)?;
        self.analyze_module(&module)
    }

    /// Analyze all functions in a module
    pub fn analyze_module(&self, module: &Module) -> Result<ModuleAnalysisResult, String> {
        let mut function_wcets = AHashMap::new();
        let mut functions_analyzed = 0;
        let mut functions_skipped = 0;

        // Iterate through all functions
        let mut func_iter = module.get_first_function();
        while let Some(function) = func_iter {
            let func_name = function.get_name().to_str().unwrap_or("").to_string();

            // Skip intrinsics and declarations
            if func_name.starts_with("llvm.") || function.count_basic_blocks() == 0 {
                functions_skipped += 1;
                func_iter = function.get_next_function();
                continue;
            }

            // Analyze function
            match self.analyze_function_internal(&function) {
                Ok(wcet) => {
                    function_wcets.insert(func_name, wcet);
                    functions_analyzed += 1;
                }
                Err(_) => {
                    functions_skipped += 1;
                }
            }

            func_iter = function.get_next_function();
        }

        if function_wcets.is_empty() {
            return Err("No functions were successfully analyzed".to_string());
        }

        Ok(ModuleAnalysisResult {
            function_wcets,
            functions_analyzed,
            functions_skipped,
        })
    }

    /// Analyze a specific function by name
    pub fn analyze_function(&self, module: &Module, function_name: &str) -> Result<u64, String> {
        let function = module
            .get_function(function_name)
            .ok_or_else(|| format!("Function '{}' not found in module", function_name))?;

        self.analyze_function_internal(&function)
    }

    /// Internal function analysis
    fn analyze_function_internal(
        &self,
        function: &inkwell::values::FunctionValue,
    ) -> Result<u64, String> {
        // Build CFG
        let cfg = InkwellCFG::from_function(function);

        // Calculate block timings
        let timings =
            InkwellTimingCalculator::calculate_block_timings(function, &cfg, &self.platform);

        // Sum all block timings as a simple WCET estimate
        let wcet: u64 = timings.values().sum();

        Ok(wcet)
    }

    /// Get detailed timing information for a function
    pub fn analyze_function_detailed(
        &self,
        module: &Module,
        function_name: &str,
    ) -> Result<FunctionTimingDetails, String> {
        let function = module
            .get_function(function_name)
            .ok_or_else(|| format!("Function '{}' not found in module", function_name))?;

        // Build CFG
        let cfg = InkwellCFG::from_function(&function);

        // Calculate block timings
        let block_timings =
            InkwellTimingCalculator::calculate_block_timings(&function, &cfg, &self.platform);

        let total_wcet: u64 = block_timings.values().sum();
        let block_count = cfg.blocks.len();
        let edge_count: usize = cfg.blocks.iter().map(|b| cfg.successors(b.id).len()).sum();

        Ok(FunctionTimingDetails {
            function_name: function_name.to_string(),
            total_wcet_cycles: total_wcet,
            block_count,
            edge_count,
            block_timings,
        })
    }
}

/// Detailed timing information for a function
#[derive(Debug, Clone)]
pub struct FunctionTimingDetails {
    pub function_name: String,
    pub total_wcet_cycles: u64,
    pub block_count: usize,
    pub edge_count: usize,
    pub block_timings: AHashMap<usize, u64>,
}
