use crate::async_analysis::{
    Actor, ActorConfigLoader, AsyncFunctionInfo, InkwellAsyncDetector, InkwellSegmentExtractor,
    InkwellSegmentWCETAnalyzer, SchedulingPolicy,
};
use crate::ir::InkwellParser;
use crate::multicore::{MultiCoreResult, MultiCoreScheduler};
use crate::platform::PlatformModel;

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

                                // Create actor
                                let mut actor = Actor::new(
                                    actor_name.to_string(),
                                    function_path.to_string(),
                                    10,         // Default priority
                                    100.0,      // Default deadline (ms)
                                    Some(50.0), // Default period (ms)
                                    Some(0),    // Default core
                                );

                                match InkwellParser::parse_file(&path) {
                                    Ok((_context, inkwell_module)) => {
                                        if let Some(inkwell_func) =
                                            inkwell_module.get_function(&async_info.function_name)
                                        {
                                            // Extract segments using inkwell
                                            let segments =
                                                InkwellSegmentExtractor::extract_segments(
                                                    &inkwell_func,
                                                    &async_info,
                                                );

                                            // Analyze WCET using inkwell
                                            let analyzer = InkwellSegmentWCETAnalyzer::new(
                                                self.platform.clone(),
                                            );
                                            let wcets =
                                                analyzer.analyze_segments(&inkwell_func, &segments);

                                            actor.segments = segments;
                                            actor.segment_wcets = wcets
                                                .into_iter()
                                                .map(|(id, w)| (id as u32, w.wcet_cycles))
                                                .collect();
                                            actor.compute_actor_wcet(
                                                self.platform.cpu_frequency_mhz,
                                            );
                                            eprintln!("      ✓ WCET analysis completed");
                                        } else {
                                            eprintln!("      ✗ Function not found in module");
                                            actor.actor_wcet_cycles = 1000;
                                            actor.actor_wcet_us =
                                                1000.0 / (self.platform.cpu_frequency_mhz as f64);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("      ✗ Parser failed: {}", e);
                                        actor.actor_wcet_cycles = 1000;
                                        actor.actor_wcet_us =
                                            1000.0 / (self.platform.cpu_frequency_mhz as f64);
                                    }
                                }

                                return Ok(actor);
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
