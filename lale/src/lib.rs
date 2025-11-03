pub mod aeg;
pub mod analysis;
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
pub use analysis::{Cycles, IPETSolver, LoopAnalyzer, TimingCalculator};
pub use async_analysis::{
    Actor, ActorConfig, ActorConfigEntry, ActorConfigLoader, ActorSegment, ActorSystem,
    ActorSystemConfig, AsyncDetector, AsyncFunctionInfo, DetectionMethod, InkwellAsyncDetector,
    SchedulingPolicy, SegmentExtractor, SegmentType, SegmentWCET, SegmentWCETAnalyzer, StateBlock,
    VeecleActor, VeecleMetadata, VeecleModel, VeecleService,
};
pub use ir::{CallGraph, IRParser, CFG};
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

/// High-level API for Veecle OS actor analysis
pub struct ActorAnalyzer {
    config_loader: ActorConfigLoader,
    platform: PlatformModel,
}

impl ActorAnalyzer {
    /// Create new analyzer with config directory and platform
    pub fn new(config_dir: &str, platform_name: &str) -> Result<Self, String> {
        let mut config_loader = ActorConfigLoader::new(config_dir);
        let platform = config_loader.load_platform_model(platform_name)?;

        Ok(Self {
            config_loader,
            platform,
        })
    }

    /// Analyze Veecle OS project
    ///
    /// Returns (actors_with_wcet, schedulability_result)
    pub fn analyze_veecle_project(
        &mut self,
        project_dir: &str,
        ir_dir: &str,
        num_cores: usize,
        policy: SchedulingPolicy,
    ) -> Result<(Vec<Actor>, MultiCoreResult), String> {
        // Load Veecle Model.toml (platform already loaded in constructor)
        let model_path = std::path::Path::new(project_dir).join("Model.toml");
        eprintln!("Loading Model.toml from: {}", model_path.display());
        let model = self.config_loader.load_veecle_model(&model_path)?;
        let actor_paths = self.config_loader.extract_actor_paths(&model);

        eprintln!("Found {} actors in Model.toml:", actor_paths.len());
        for (name, path) in &actor_paths {
            eprintln!("  - {} -> {}", name, path);
        }

        let mut actors = Vec::new();

        // Analyze each actor
        for (name, path) in actor_paths {
            eprintln!("Analyzing actor: {} (path: {})", name, path);
            // Try to find matching LLVM IR file
            match self.analyze_actor_from_ir(ir_dir, &name, &path) {
                Ok(actor) => {
                    eprintln!("  ✓ Successfully analyzed actor: {}", name);
                    actors.push(actor);
                }
                Err(e) => {
                    eprintln!("  ✗ Failed to analyze actor {}: {}", name, e);
                }
            }
        }

        eprintln!("Total actors analyzed: {}", actors.len());

        // Perform multi-core schedulability analysis
        let scheduler = MultiCoreScheduler::new(num_cores, policy);
        let schedulability = scheduler.analyze(&actors);

        Ok((actors, schedulability))
    }

    /// Analyze single actor from LLVM IR
    fn analyze_actor_from_ir(
        &self,
        ir_dir: &str,
        actor_name: &str,
        function_path: &str,
    ) -> Result<Actor, String> {
        eprintln!("  Searching for actor in IR directory: {}", ir_dir);
        eprintln!("  Looking for function path: {}", function_path);

        // Find IR files in directory
        let ir_files =
            std::fs::read_dir(ir_dir).map_err(|e| format!("Failed to read IR directory: {}", e))?;

        let mut ir_file_count = 0;
        let mut async_func_count = 0;

        for entry in ir_files.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("ll") {
                ir_file_count += 1;
                eprintln!("  Checking IR file: {}", path.display());

                // Try to detect async functions in this file
                match InkwellAsyncDetector::detect_from_file(&path) {
                    Ok(async_funcs) => {
                        if !async_funcs.is_empty() {
                            eprintln!("    Found {} async functions", async_funcs.len());
                        }
                        for async_info in async_funcs {
                            async_func_count += 1;
                            eprintln!("      - {}", async_info.function_name);

                            // Check if function matches actor path
                            if async_info.function_name.contains(function_path)
                                || function_path.contains(&async_info.function_name)
                            {
                                eprintln!("      ✓ MATCH! This function matches the actor path");
                                // Parse module to get function
                                if let Ok(module) = IRParser::parse_file(&path) {
                                    if let Some(function) = module
                                        .functions
                                        .iter()
                                        .find(|f| f.name.to_string() == async_info.function_name)
                                    {
                                        // Extract segments
                                        let segments = SegmentExtractor::extract_segments(
                                            function,
                                            &async_info,
                                        );

                                        // Analyze WCET per segment
                                        let analyzer =
                                            SegmentWCETAnalyzer::new(self.platform.clone());
                                        let wcets = analyzer.analyze_segments(function, &segments);

                                        // Create actor
                                        let mut actor = Actor::new(
                                            actor_name.to_string(),
                                            function_path.to_string(),
                                            10,         // Default priority
                                            100.0,      // Default deadline (ms)
                                            Some(50.0), // Default period (ms)
                                            Some(0),    // Default core
                                        );

                                        actor.segments = segments;
                                        actor.segment_wcets = wcets
                                            .into_iter()
                                            .map(|(id, w)| (id, w.wcet_cycles))
                                            .collect();
                                        actor.compute_actor_wcet(self.platform.cpu_frequency_mhz);

                                        return Ok(actor);
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        // Silently skip files with parse errors (likely debug intrinsics)
                        if e.contains("dbg_value") || e.contains("dbg_declare") {
                            eprintln!("    Skipping (debug intrinsics)");
                        } else {
                            eprintln!("    Parse error: {}", e);
                        }
                    }
                }
            }
        }

        eprintln!(
            "  Scanned {} IR files, found {} async functions total",
            ir_file_count, async_func_count
        );
        Err(format!("Could not find LLVM IR for actor: {}", actor_name))
    }
}

/// Analyze async functions in LLVM IR file
pub fn analyze_async_functions(ir_file_path: &str) -> Result<Vec<AsyncFunctionInfo>, String> {
    InkwellAsyncDetector::detect_from_file(ir_file_path).map_err(|e| e.to_string())
}

/// Analyze WCET for specific function in LLVM IR
pub fn analyze_function_wcet(
    ir_file_path: &str,
    function_name: &str,
    platform: PlatformModel,
) -> Result<u64, String> {
    let module = IRParser::parse_file(ir_file_path).map_err(|e| e.to_string())?;

    let function = module
        .functions
        .iter()
        .find(|f| f.name.to_string() == function_name)
        .ok_or_else(|| format!("Function {} not found", function_name))?;

    let analyzer = WCETAnalyzer::new(platform);
    analyzer.analyze_function(function)
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
