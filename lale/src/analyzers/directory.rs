//! Directory-level WCET analysis
//!
//! Analyzes all LLVM IR files in a directory and generates WCET estimates.

use crate::analysis::{Cycles, InkwellTimingCalculator};
use crate::ir::{InkwellCFG, InkwellParser};
use crate::platform::PlatformModel;
use crate::scheduling::Task;
use ahash::AHashMap;
use std::path::{Path, PathBuf};

/// Result of analyzing a directory
#[derive(Debug, Clone)]
pub struct DirectoryAnalysisResult {
    /// WCET results per function (function_name -> wcet_cycles)
    pub function_wcets: AHashMap<String, u64>,

    /// Tasks generated from functions
    pub tasks: Vec<Task>,

    /// Files that were successfully analyzed
    pub analyzed_files: Vec<PathBuf>,

    /// Files that failed to analyze
    pub failed_files: Vec<(PathBuf, String)>,
}

/// Analyzer for directories containing LLVM IR files
pub struct DirectoryAnalyzer {
    platform: PlatformModel,
}

impl DirectoryAnalyzer {
    /// Create a new directory analyzer with the given platform
    pub fn new(platform: PlatformModel) -> Self {
        Self { platform }
    }

    /// Analyze all .ll files in a directory recursively
    pub fn analyze_directory(
        &self,
        dir_path: impl AsRef<Path>,
    ) -> Result<DirectoryAnalysisResult, String> {
        let dir = dir_path.as_ref();

        if !dir.exists() {
            return Err(format!("Directory does not exist: {}", dir.display()));
        }

        if !dir.is_dir() {
            return Err(format!("Path is not a directory: {}", dir.display()));
        }

        // Find all .ll files
        let ll_files = self.find_ll_files(dir)?;

        if ll_files.is_empty() {
            return Err(format!(
                "No .ll files found in directory: {}",
                dir.display()
            ));
        }

        let mut function_wcets = AHashMap::new();
        let mut analyzed_files = Vec::new();
        let mut failed_files = Vec::new();

        // Analyze each file
        for ll_file in ll_files {
            match self.analyze_file(&ll_file) {
                Ok(wcets) => {
                    function_wcets.extend(wcets);
                    analyzed_files.push(ll_file);
                }
                Err(e) => {
                    failed_files.push((ll_file, e));
                }
            }
        }

        if function_wcets.is_empty() {
            return Err("No functions were successfully analyzed".to_string());
        }

        // Generate tasks from analyzed functions
        let tasks = self.generate_tasks(&function_wcets);

        Ok(DirectoryAnalysisResult {
            function_wcets,
            tasks,
            analyzed_files,
            failed_files,
        })
    }

    /// Analyze a single LLVM IR file
    fn analyze_file(&self, path: &Path) -> Result<AHashMap<String, u64>, String> {
        let (_context, module) = InkwellParser::parse_file(path)?;

        let mut results = AHashMap::new();

        // Analyze each function in the module
        let mut func_iter = module.get_first_function();
        while let Some(function) = func_iter {
            let func_name = function.get_name().to_str().unwrap_or("").to_string();

            // Skip intrinsics and declarations
            if func_name.starts_with("llvm.") || function.count_basic_blocks() == 0 {
                func_iter = function.get_next_function();
                continue;
            }

            // Build CFG and calculate timing
            let cfg = InkwellCFG::from_function(&function);
            let timings =
                InkwellTimingCalculator::calculate_block_timings(&function, &cfg, &self.platform);

            // Sum all block timings as a simple WCET estimate
            let wcet: u64 = timings.values().sum();

            results.insert(func_name, wcet);

            func_iter = function.get_next_function();
        }

        Ok(results)
    }

    /// Find all .ll files in directory recursively
    fn find_ll_files(&self, dir: &Path) -> Result<Vec<PathBuf>, String> {
        let mut ll_files = Vec::new();

        let entries =
            std::fs::read_dir(dir).map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "ll" {
                        ll_files.push(path);
                    }
                }
            } else if path.is_dir() {
                ll_files.extend(self.find_ll_files(&path)?);
            }
        }

        Ok(ll_files)
    }

    /// Generate tasks from function WCET results
    fn generate_tasks(&self, function_wcets: &AHashMap<String, u64>) -> Vec<Task> {
        function_wcets
            .iter()
            .map(|(func_name, &wcet_cycles)| {
                let wcet_us = wcet_cycles as f64 / self.platform.cpu_frequency_mhz as f64;

                Task {
                    name: func_name.clone(),
                    function: func_name.clone(),
                    wcet_cycles,
                    wcet_us,
                    period_us: None,
                    deadline_us: None,
                    priority: None,
                    preemptible: true,
                    dependencies: vec![],
                }
            })
            .collect()
    }

    /// Analyze directory and generate tasks with specified period
    pub fn analyze_with_period(
        &self,
        dir_path: impl AsRef<Path>,
        period_us: f64,
    ) -> Result<DirectoryAnalysisResult, String> {
        let mut result = self.analyze_directory(dir_path)?;

        // Update tasks with period
        for task in &mut result.tasks {
            task.period_us = Some(period_us);
            task.deadline_us = Some(period_us);
        }

        Ok(result)
    }
}
