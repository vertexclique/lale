use anyhow::{Context, Result};
use lale::{
    AnalysisReport, CortexA53Model, CortexA7Model, CortexM0Model, CortexM33Model, CortexM3Model,
    CortexM4Model, CortexM7Model, CortexR4Model, CortexR5Model, InkwellParser, PlatformModel,
    RV32GCModel, RV32IMACModel, RV32IModel, RV64GCModel, SchedulingPolicy, Task,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub dir_path: String,
    pub platform: String,
    pub policy: String,
    pub tasks: Vec<TaskConfig>,
    pub auto_tasks: bool,
    pub auto_period_us: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskConfig {
    pub name: String,
    pub function: String,
    pub period_us: f64,
    pub deadline_us: Option<f64>,
    pub priority: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub id: String,
    pub name: String,
    pub frequency_mhz: u32,
    pub category: String,
}

/// Get list of all available platforms
pub fn list_platforms() -> Vec<PlatformInfo> {
    vec![
        // ARM Cortex-M
        PlatformInfo {
            id: "cortex-m0".to_string(),
            name: "ARM Cortex-M0".to_string(),
            frequency_mhz: 48,
            category: "ARM Cortex-M".to_string(),
        },
        PlatformInfo {
            id: "cortex-m3".to_string(),
            name: "ARM Cortex-M3".to_string(),
            frequency_mhz: 72,
            category: "ARM Cortex-M".to_string(),
        },
        PlatformInfo {
            id: "cortex-m4".to_string(),
            name: "ARM Cortex-M4".to_string(),
            frequency_mhz: 168,
            category: "ARM Cortex-M".to_string(),
        },
        PlatformInfo {
            id: "cortex-m7".to_string(),
            name: "ARM Cortex-M7".to_string(),
            frequency_mhz: 400,
            category: "ARM Cortex-M".to_string(),
        },
        PlatformInfo {
            id: "cortex-m33".to_string(),
            name: "ARM Cortex-M33".to_string(),
            frequency_mhz: 120,
            category: "ARM Cortex-M".to_string(),
        },
        // ARM Cortex-R
        PlatformInfo {
            id: "cortex-r4".to_string(),
            name: "ARM Cortex-R4".to_string(),
            frequency_mhz: 600,
            category: "ARM Cortex-R".to_string(),
        },
        PlatformInfo {
            id: "cortex-r5".to_string(),
            name: "ARM Cortex-R5".to_string(),
            frequency_mhz: 800,
            category: "ARM Cortex-R".to_string(),
        },
        // ARM Cortex-A
        PlatformInfo {
            id: "cortex-a7".to_string(),
            name: "ARM Cortex-A7".to_string(),
            frequency_mhz: 1200,
            category: "ARM Cortex-A".to_string(),
        },
        PlatformInfo {
            id: "cortex-a53".to_string(),
            name: "ARM Cortex-A53".to_string(),
            frequency_mhz: 1400,
            category: "ARM Cortex-A".to_string(),
        },
        // RISC-V
        PlatformInfo {
            id: "rv32i".to_string(),
            name: "RISC-V RV32I".to_string(),
            frequency_mhz: 100,
            category: "RISC-V".to_string(),
        },
        PlatformInfo {
            id: "rv32imac".to_string(),
            name: "RISC-V RV32IMAC".to_string(),
            frequency_mhz: 320,
            category: "RISC-V".to_string(),
        },
        PlatformInfo {
            id: "rv32gc".to_string(),
            name: "RISC-V RV32GC".to_string(),
            frequency_mhz: 1000,
            category: "RISC-V".to_string(),
        },
        PlatformInfo {
            id: "rv64gc".to_string(),
            name: "RISC-V RV64GC".to_string(),
            frequency_mhz: 1500,
            category: "RISC-V".to_string(),
        },
    ]
}

/// Select platform model by ID or configuration path
fn select_platform(platform_id: &str) -> Result<PlatformModel> {
    // Check if it's a TOML configuration path (e.g., "platforms/nucleo-h743zi")
    if platform_id.contains('/') || platform_id.contains("platforms") {
        use lale::analysis::timing::AccessType;
        use lale::analysis::{Cycles, InstructionClass};
        use lale::config::ConfigManager;
        use std::path::PathBuf;

        let config_dir = PathBuf::from("config");
        let mut manager = ConfigManager::new(config_dir);

        let config = manager.load_platform(platform_id).map_err(|e| {
            anyhow::anyhow!(
                "Failed to load platform configuration '{}': {}",
                platform_id,
                e
            )
        })?;

        // Build instruction timings from ISA config
        let mut instruction_timings = ahash::AHashMap::new();

        // Arithmetic operations
        instruction_timings.insert(
            InstructionClass::Add,
            Cycles::new(config.isa.instruction_timings.alu),
        );
        instruction_timings.insert(
            InstructionClass::Sub,
            Cycles::new(config.isa.instruction_timings.alu),
        );
        instruction_timings.insert(
            InstructionClass::And,
            Cycles::new(config.isa.instruction_timings.alu),
        );
        instruction_timings.insert(
            InstructionClass::Or,
            Cycles::new(config.isa.instruction_timings.alu),
        );
        instruction_timings.insert(
            InstructionClass::Xor,
            Cycles::new(config.isa.instruction_timings.alu),
        );
        instruction_timings.insert(
            InstructionClass::Shl,
            Cycles::new(config.isa.instruction_timings.alu),
        );
        instruction_timings.insert(
            InstructionClass::Shr,
            Cycles::new(config.isa.instruction_timings.alu),
        );

        // Memory operations
        instruction_timings.insert(
            InstructionClass::Load(AccessType::Ram),
            Cycles::new(config.isa.instruction_timings.load),
        );
        instruction_timings.insert(
            InstructionClass::Store(AccessType::Ram),
            Cycles::new(config.isa.instruction_timings.store),
        );

        // Control flow
        instruction_timings.insert(
            InstructionClass::Branch,
            Cycles::new(config.isa.instruction_timings.branch),
        );
        instruction_timings.insert(
            InstructionClass::Call,
            Cycles::new(config.isa.instruction_timings.branch),
        );
        instruction_timings.insert(
            InstructionClass::Ret,
            Cycles::new(config.isa.instruction_timings.branch),
        );

        // Multiply/Divide
        instruction_timings.insert(
            InstructionClass::Mul,
            Cycles::new(config.isa.instruction_timings.multiply),
        );
        instruction_timings.insert(
            InstructionClass::Div,
            Cycles::new(config.isa.instruction_timings.divide),
        );
        instruction_timings.insert(
            InstructionClass::Rem,
            Cycles::new(config.isa.instruction_timings.divide),
        );

        // Get CPU frequency from SoC or use default
        let cpu_frequency_mhz = config
            .soc
            .as_ref()
            .map(|s| s.cpu_frequency_mhz)
            .unwrap_or(100); // Default 100 MHz if no SoC specified

        return Ok(PlatformModel {
            name: platform_id.to_string(),
            cpu_frequency_mhz,
            instruction_timings,
        });
    }

    // Fallback to hardcoded platform models for backward compatibility
    let model = match platform_id.to_lowercase().as_str() {
        "cortex-m0" | "m0" => CortexM0Model::new(),
        "cortex-m3" | "m3" => CortexM3Model::new(),
        "cortex-m4" | "m4" => CortexM4Model::new(),
        "cortex-m7" | "m7" => CortexM7Model::new(),
        "cortex-m33" | "m33" => CortexM33Model::new(),
        "cortex-r4" | "r4" => CortexR4Model::new(),
        "cortex-r5" | "r5" => CortexR5Model::new(),
        "cortex-a7" | "a7" => CortexA7Model::new(),
        "cortex-a53" | "a53" => CortexA53Model::new(),
        "rv32i" => RV32IModel::new(),
        "rv32imac" => RV32IMACModel::new(),
        "rv32gc" => RV32GCModel::new(),
        "rv64gc" => RV64GCModel::new(),
        _ => anyhow::bail!("Unknown platform: {}", platform_id),
    };
    Ok(model)
}

/// Find all .ll files in directory recursively
fn find_ll_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut ll_files = Vec::new();

    if !dir.exists() {
        anyhow::bail!("Directory does not exist: {}", dir.display());
    }

    if !dir.is_dir() {
        anyhow::bail!("Path is not a directory: {}", dir.display());
    }

    // Collect and sort entries for deterministic order
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "ll" {
                    ll_files.push(path);
                }
            }
        } else if path.is_dir() {
            ll_files.extend(find_ll_files(&path)?);
        }
    }

    Ok(ll_files)
}

/// Perform complete WCET analysis using the new DirectoryAnalyzer
pub fn analyze_directory(config: AnalysisConfig) -> Result<AnalysisReport> {
    use lale::DirectoryAnalyzer;

    // Select platform
    let platform = select_platform(&config.platform)?;

    // Create analyzer
    let analyzer = DirectoryAnalyzer::new(platform.clone());

    // Analyze directory
    let result = if config.auto_tasks {
        analyzer
            .analyze_with_period(&config.dir_path, config.auto_period_us)
            .map_err(|e| anyhow::anyhow!("Analysis failed: {}", e))?
    } else {
        analyzer
            .analyze_directory(&config.dir_path)
            .map_err(|e| anyhow::anyhow!("Analysis failed: {}", e))?
    };

    // Use configured tasks if provided, otherwise use auto-generated tasks
    let mut tasks = if config.auto_tasks {
        result.tasks
    } else {
        // Filter to only configured tasks
        config
            .tasks
            .iter()
            .filter_map(|tc| {
                let wcet_cycles = result.function_wcets.get(&tc.function).copied()?;
                let wcet_us = wcet_cycles as f64 / platform.cpu_frequency_mhz as f64;

                Some(Task {
                    name: tc.name.clone(),
                    function: tc.function.clone(),
                    wcet_cycles,
                    wcet_us,
                    period_us: Some(tc.period_us),
                    deadline_us: tc.deadline_us.or(Some(tc.period_us)),
                    priority: tc.priority,
                    preemptible: true,
                    dependencies: vec![],
                })
            })
            .collect()
    };

    if tasks.is_empty() {
        anyhow::bail!("No valid tasks configured or found");
    }

    // Parse scheduling policy
    let policy = match config.policy.to_lowercase().as_str() {
        "rma" => SchedulingPolicy::RMA,
        "edf" => SchedulingPolicy::EDF,
        _ => SchedulingPolicy::RMA,
    };

    // Generate schedule using RMA or EDF
    let schedulability = match policy {
        SchedulingPolicy::RMA => lale::scheduling::RMAScheduler::schedulability_test(&tasks),
        SchedulingPolicy::EDF => lale::scheduling::EDFScheduler::schedulability_test(&tasks),
    };

    // Create analysis report with proper structure
    use chrono::Utc;
    use lale::output::json::{
        AnalysisInfo, FunctionWCET, SchedulabilityAnalysis, TaskModel, WCETAnalysis,
    };

    let analysis_info = AnalysisInfo {
        tool: "lale".to_string(),
        version: lale::VERSION.to_string(),
        timestamp: Utc::now().to_rfc3339(),
        platform: platform.name.clone(),
    };

    let wcet_analysis = WCETAnalysis {
        functions: result
            .function_wcets
            .iter()
            .map(|(name, &wcet)| FunctionWCET {
                name: name.clone(),
                llvm_name: name.clone(),
                wcet_cycles: wcet,
                wcet_us: wcet as f64 / platform.cpu_frequency_mhz as f64,
                bcet_cycles: wcet, // Conservative estimate
                bcet_us: wcet as f64 / platform.cpu_frequency_mhz as f64,
                loop_count: 0,
            })
            .collect(),
    };

    let task_model = TaskModel {
        tasks: tasks.clone(),
    };

    // Calculate utilization
    let utilization = tasks
        .iter()
        .filter(|t| t.period_us.is_some())
        .map(|t| t.wcet_us / t.period_us.unwrap())
        .sum();

    let schedulability_analysis = SchedulabilityAnalysis {
        method: format!("{:?}", policy),
        result: match schedulability {
            lale::scheduling::SchedulabilityResult::Schedulable => "Schedulable".to_string(),
            lale::scheduling::SchedulabilityResult::Unschedulable { .. } => {
                "Not Schedulable".to_string()
            }
        },
        utilization,
        utilization_bound: Some(1.0),
        response_times: ahash::AHashMap::new(),
    };

    let report = AnalysisReport {
        analysis_info,
        wcet_analysis,
        task_model,
        schedulability: schedulability_analysis,
        schedule: None,
    };

    Ok(report)
}
