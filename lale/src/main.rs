use anyhow::{Context, Result};
use lale::{
    CortexA53Model, CortexA7Model, CortexM0Model, CortexM33Model, CortexM3Model, CortexM4Model,
    CortexM7Model, CortexR4Model, CortexR5Model, IRParser, PlatformModel, RV32GCModel,
    RV32IMACModel, RV32IModel, RV64GCModel, SchedulingPolicy, Task, WCETAnalyzer,
};
use std::path::PathBuf;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "analyze" => {
            if args.len() < 3 {
                eprintln!("Error: Missing directory path");
                print_usage();
                std::process::exit(1);
            }
            let dir = PathBuf::from(&args[2]);
            let config = parse_config(&args[3..])?;
            analyze_directory(dir, config)?;
        }
        "list-boards" => {
            list_boards()?;
        }
        "validate-board" => {
            if args.len() < 3 {
                eprintln!("Error: Missing board name");
                eprintln!("Usage: lale validate-board <board-name>");
                std::process::exit(1);
            }
            validate_board(&args[2])?;
        }
        "export-board" => {
            if args.len() < 3 {
                eprintln!("Error: Missing board name");
                eprintln!("Usage: lale export-board <board-name>");
                std::process::exit(1);
            }
            export_board(&args[2])?;
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        "version" | "--version" | "-v" => {
            println!("LALE v{}", lale::VERSION);
        }
        _ => {
            eprintln!("Error: Unknown command '{}'", command);
            print_usage();
            std::process::exit(1);
        }
    }

    Ok(())
}

#[derive(Debug)]
struct Config {
    platform: Option<String>, // Legacy platform name
    board: Option<String>,    // New board config path
    policy: SchedulingPolicy,
    output: PathBuf,
    tasks: Vec<TaskConfig>,
    auto_tasks: bool,
    auto_period_us: f64,
}

#[derive(Debug, Clone)]
struct TaskConfig {
    name: String,
    function: String,
    period_us: f64,
    deadline_us: Option<f64>,
    priority: Option<u8>,
}

fn parse_config(args: &[String]) -> Result<Config> {
    let mut platform: Option<String> = None;
    let mut board: Option<String> = None;
    let mut policy = SchedulingPolicy::RMA;
    let mut output = PathBuf::from("schedule.json");
    let mut tasks = Vec::new();
    let mut auto_tasks = false;
    let mut auto_period_us = 10000.0;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--platform" | "-p" => {
                i += 1;
                if i < args.len() {
                    platform = Some(args[i].clone());
                }
            }
            "--board" | "-b" => {
                i += 1;
                if i < args.len() {
                    board = Some(args[i].clone());
                }
            }
            "--policy" => {
                i += 1;
                if i < args.len() {
                    policy = match args[i].as_str() {
                        "rma" | "RMA" => SchedulingPolicy::RMA,
                        "edf" | "EDF" => SchedulingPolicy::EDF,
                        _ => {
                            eprintln!("Warning: Unknown policy '{}', using RMA", args[i]);
                            SchedulingPolicy::RMA
                        }
                    };
                }
            }
            "--output" | "-o" => {
                i += 1;
                if i < args.len() {
                    output = PathBuf::from(&args[i]);
                }
            }
            "--task" | "-t" => {
                // Format: --task name:function:period_us[:deadline_us[:priority]]
                i += 1;
                if i < args.len() {
                    if let Some(task) = parse_task(&args[i]) {
                        tasks.push(task);
                    }
                }
            }
            "--auto-tasks" => {
                auto_tasks = true;
            }
            "--auto-period" => {
                i += 1;
                if i < args.len() {
                    if let Ok(period) = args[i].parse::<f64>() {
                        auto_period_us = period;
                    } else {
                        eprintln!(
                            "Warning: Invalid period '{}', using default 10000us",
                            args[i]
                        );
                    }
                }
            }
            _ => {
                eprintln!("Warning: Unknown option '{}'", args[i]);
            }
        }
        i += 1;
    }

    // Default to cortex-m4 if neither platform nor board specified
    let final_platform = platform.or(Some("cortex-m4".to_string()));

    Ok(Config {
        platform: final_platform,
        board,
        policy,
        output,
        tasks,
        auto_tasks,
        auto_period_us,
    })
}

fn parse_task(spec: &str) -> Option<TaskConfig> {
    let parts: Vec<&str> = spec.split(':').collect();
    if parts.len() < 3 {
        eprintln!("Warning: Invalid task spec '{}', expected name:function:period_us[:deadline_us[:priority]]", spec);
        return None;
    }

    let name = parts[0].to_string();
    let function = parts[1].to_string();
    let period_us = parts[2].parse::<f64>().ok()?;
    let deadline_us = parts.get(3).and_then(|s| s.parse::<f64>().ok());
    let priority = parts.get(4).and_then(|s| s.parse::<u8>().ok());

    Some(TaskConfig {
        name,
        function,
        period_us,
        deadline_us,
        priority,
    })
}

fn select_platform(name: &str) -> Result<PlatformModel> {
    let model = match name.to_lowercase().as_str() {
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
        _ => {
            anyhow::bail!(
                "Unknown platform '{}'. Use --help to see available platforms.",
                name
            );
        }
    };
    Ok(model)
}

fn analyze_directory(dir: PathBuf, config: Config) -> Result<()> {
    println!("LALE - LLVM-based WCET Analysis and Static Scheduling");
    println!("======================================================");
    println!();
    println!("Configuration:");
    println!("  Directory: {}", dir.display());

    if let Some(ref board) = config.board {
        println!("  Board: {}", board);
    } else if let Some(ref platform) = config.platform {
        println!("  Platform: {}", platform);
    }

    println!("  Policy: {:?}", config.policy);
    println!("  Output: {}", config.output.display());
    println!("  Tasks: {} configured", config.tasks.len());
    println!();

    // Find all .ll files in directory
    let ll_files = find_ll_files(&dir)?;
    if ll_files.is_empty() {
        anyhow::bail!("No .ll files found in directory: {}", dir.display());
    }

    println!("Found {} LLVM IR file(s)", ll_files.len());
    println!();

    // Select platform - use platform if specified, otherwise default
    let platform_name = config
        .platform
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No platform specified"))?;
    let platform = select_platform(platform_name)?;
    let analyzer = WCETAnalyzer::new(platform);

    // Parse all modules
    let mut all_wcet_results = ahash::AHashMap::new();
    for ll_file in &ll_files {
        println!("Analyzing: {}", ll_file.display());
        match IRParser::parse_file(ll_file.to_str().unwrap()) {
            Ok(module) => {
                let results = analyzer.analyze_module(&module);
                println!("  Found {} functions", results.len());
                all_wcet_results.extend(results);
            }
            Err(e) => {
                eprintln!("  Warning: Failed to parse {}: {}", ll_file.display(), e);
            }
        }
    }

    println!();
    println!("Total functions analyzed: {}", all_wcet_results.len());
    println!();

    // Build tasks from config or auto-generate
    let tasks: Vec<Task> = if config.auto_tasks {
        println!("Auto-generating tasks from all analyzed functions...");
        println!("  Default period: {:.2}us", config.auto_period_us);
        println!();

        all_wcet_results
            .iter()
            .enumerate()
            .map(|(idx, (func_name, &wcet_cycles))| {
                let wcet_us = wcet_cycles as f64 / analyzer.platform.cpu_frequency_mhz as f64;
                let period_us = config.auto_period_us;

                Task {
                    name: format!("task_{}", idx),
                    function: func_name.clone(),
                    wcet_cycles,
                    wcet_us,
                    period_us: Some(period_us),
                    deadline_us: Some(period_us),
                    priority: Some(idx as u8),
                    preemptible: true,
                    dependencies: vec![],
                }
            })
            .collect()
    } else {
        config
            .tasks
            .iter()
            .filter_map(|tc| {
                let wcet_cycles = all_wcet_results.get(&tc.function).copied();
                if wcet_cycles.is_none() {
                    eprintln!(
                        "Warning: Function '{}' not found in WCET results, skipping task '{}'",
                        tc.function, tc.name
                    );
                    return None;
                }

                let wcet_cycles = wcet_cycles.unwrap();
                let wcet_us = wcet_cycles as f64 / analyzer.platform.cpu_frequency_mhz as f64;

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
        anyhow::bail!("No valid tasks configured. Use --task to specify tasks or --auto-tasks.");
    }

    println!("Tasks:");
    for task in &tasks {
        println!(
            "  {} ({}): WCET={:.2}us, Period={:.2}us",
            task.name,
            task.function,
            task.wcet_us,
            task.period_us.unwrap_or(0.0)
        );
    }
    println!();

    // Perform analysis and export
    println!("Performing schedulability analysis...");

    // We need to parse at least one module for the export
    let first_module = IRParser::parse_file(ll_files[0].to_str().unwrap())
        .context("Failed to parse first module")?;

    let json = analyzer
        .analyze_and_export_schedule(&first_module, tasks, config.policy)
        .context("Failed to generate schedule")?;

    // Write to file
    std::fs::write(&config.output, &json)
        .with_context(|| format!("Failed to write to {}", config.output.display()))?;

    println!("✓ Analysis complete!");
    println!("✓ Schedule exported to: {}", config.output.display());

    Ok(())
}

fn find_ll_files(dir: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut ll_files = Vec::new();

    if !dir.exists() {
        anyhow::bail!("Directory does not exist: {}", dir.display());
    }

    if !dir.is_dir() {
        anyhow::bail!("Path is not a directory: {}", dir.display());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "ll" {
                    ll_files.push(path);
                }
            }
        } else if path.is_dir() {
            // Recursively search subdirectories
            ll_files.extend(find_ll_files(&path)?);
        }
    }

    Ok(ll_files)
}

fn list_boards() -> Result<()> {
    use lale::config::ConfigManager;

    let config_dir = PathBuf::from("config");
    let manager = ConfigManager::new(config_dir);

    println!("Available Board Configurations:");
    println!("================================");
    println!();

    match manager.list_platforms() {
        Ok(platforms) => {
            if platforms.is_empty() {
                println!("No board configurations found in config/ directory");
                return Ok(());
            }

            // Group by category
            let mut cores = Vec::new();
            let mut platforms_list = Vec::new();

            for platform in platforms {
                if platform.starts_with("cores/") {
                    cores.push(platform);
                } else if platform.starts_with("platforms/") {
                    platforms_list.push(platform);
                }
            }

            if !cores.is_empty() {
                println!("Core Configurations:");
                for core in &cores {
                    println!("  {}", core);
                }
                println!();
            }

            if !platforms_list.is_empty() {
                println!("Platform Configurations:");
                for platform in &platforms_list {
                    println!("  {}", platform);
                }
                println!();
            }

            println!(
                "Total: {} configurations",
                cores.len() + platforms_list.len()
            );
        }
        Err(e) => {
            eprintln!("Error listing boards: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn validate_board(board_name: &str) -> Result<()> {
    use lale::config::ConfigManager;

    let config_dir = PathBuf::from("config");
    let mut manager = ConfigManager::new(config_dir);

    println!("Validating board configuration: {}", board_name);
    println!();

    match manager.load_platform(board_name) {
        Ok(config) => {
            println!("✓ Configuration loaded successfully");
            println!();
            println!("Configuration Details:");
            println!("  ISA: {}", config.isa.name);
            println!("  Core: {}", config.core.name);
            println!("  Pipeline stages: {}", config.core.pipeline.stages);

            if let Some(ref icache) = config.core.cache.instruction_cache {
                println!(
                    "  I-Cache: {} KB, {}-way",
                    icache.size_kb, icache.associativity
                );
            }

            if let Some(ref dcache) = config.core.cache.data_cache {
                println!(
                    "  D-Cache: {} KB, {}-way",
                    dcache.size_kb, dcache.associativity
                );
            }

            if let Some(ref soc) = config.soc {
                println!("  SoC: {} @ {} MHz", soc.name, soc.cpu_frequency_mhz);
                println!("  Memory regions: {}", soc.memory_regions.len());
            }

            if let Some(ref board) = config.board {
                println!("  Board: {}", board.name);
            }

            println!();
            println!("✓ Validation passed");
        }
        Err(e) => {
            eprintln!("✗ Validation failed:");
            eprintln!("{}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn export_board(board_name: &str) -> Result<()> {
    use lale::config::ConfigManager;

    let config_dir = PathBuf::from("config");
    let mut manager = ConfigManager::new(config_dir);

    match manager.load_platform(board_name) {
        Ok(config) => match manager.export_platform(&config) {
            Ok(toml_string) => {
                println!("{}", toml_string);
            }
            Err(e) => {
                eprintln!("Error exporting configuration: {}", e);
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}

fn print_usage() {
    println!("LALE - LLVM-based WCET Analysis and Static Scheduling");
    println!();
    println!("USAGE:");
    println!("    lale analyze <directory> [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --platform, -p <platform>    Target platform (default: cortex-m4)");
    println!("    --policy <policy>            Scheduling policy: rma, edf (default: rma)");
    println!("    --output, -o <file>          Output file (default: schedule.json)");
    println!("    --task, -t <spec>            Task specification (can be repeated)");
    println!("    --auto-tasks                 Auto-generate tasks from all functions");
    println!("    --auto-period <us>           Period for auto-generated tasks (default: 10000us)");
    println!();
    println!("TASK SPECIFICATION:");
    println!("    Format: name:function:period_us[:deadline_us[:priority]]");
    println!("    Example: --task sensor:read_sensor:10000:8000:1");
    println!();
    println!("AUTO-TASK MODE:");
    println!("    Use --auto-tasks to automatically create a task for every analyzed function.");
    println!("    All tasks will have the same period (configurable with --auto-period).");
    println!("    Priorities are assigned based on function order (0 = highest).");
    println!();
    println!("AVAILABLE PLATFORMS:");
    println!("    ARM Cortex-M:");
    println!("      cortex-m0, m0      - Cortex-M0/M0+/M1 @ 48MHz");
    println!("      cortex-m3, m3      - Cortex-M3 @ 72MHz");
    println!("      cortex-m4, m4      - Cortex-M4 @ 168MHz (default)");
    println!("      cortex-m7, m7      - Cortex-M7 @ 400MHz");
    println!("      cortex-m33, m33    - Cortex-M33 @ 120MHz");
    println!();
    println!("    ARM Cortex-R:");
    println!("      cortex-r4, r4      - Cortex-R4 @ 600MHz");
    println!("      cortex-r5, r5      - Cortex-R5 @ 800MHz");
    println!();
    println!("    ARM Cortex-A:");
    println!("      cortex-a7, a7      - Cortex-A7 @ 1200MHz");
    println!("      cortex-a53, a53    - Cortex-A53 @ 1400MHz");
    println!();
    println!("    RISC-V:");
    println!("      rv32i              - RV32I @ 100MHz");
    println!("      rv32imac           - RV32IMAC @ 320MHz");
    println!("      rv32gc             - RV32GC @ 1000MHz");
    println!("      rv64gc             - RV64GC @ 1500MHz");
    println!();
    println!("EXAMPLES:");
    println!("    # Manual task specification:");
    println!("    lale analyze ./data/armv7e-m \\");
    println!("        --platform cortex-m4 \\");
    println!("        --policy rma \\");
    println!("        --output schedule.json \\");
    println!("        --task sensor:read_sensor:10000:8000:1 \\");
    println!("        --task control:control_loop:5000:4500:0");
    println!();
    println!("    # Auto-generate tasks from all functions:");
    println!("    lale analyze ./data/armv7e-m \\");
    println!("        --platform cortex-m4 \\");
    println!("        --auto-tasks \\");
    println!("        --auto-period 10000 \\");
    println!("        --output schedule.json");
    println!();
    println!("BOARD CONFIGURATION COMMANDS:");
    println!("    lale list-boards                List available board configurations");
    println!("    lale validate-board <name>      Validate a board configuration");
    println!("    lale export-board <name>        Export resolved board configuration");
    println!();
    println!("    Examples:");
    println!("      lale list-boards");
    println!("      lale validate-board platforms/stm32f746-discovery");
    println!("      lale export-board platforms/stm32f746-discovery > my-board.toml");
    println!();
    println!("OTHER COMMANDS:");
    println!("    lale help              Show this help message");
    println!("    lale version           Show version information");
}
