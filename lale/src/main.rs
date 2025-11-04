use anyhow::{Context, Result};
use lale::analysis::InkwellTimingCalculator;
use lale::{
    CortexA53Model, CortexA7Model, CortexM0Model, CortexM33Model, CortexM3Model, CortexM4Model,
    CortexM7Model, CortexR4Model, CortexR5Model, InkwellParser, PlatformModel, RV32GCModel,
    RV32IMACModel, RV32IModel, RV64GCModel, SchedulingPolicy,
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
    platform: Option<String>,
    board: Option<String>,
    output: PathBuf,
}

fn parse_config(args: &[String]) -> Result<Config> {
    let mut platform: Option<String> = None;
    let mut board: Option<String> = None;
    let mut output = PathBuf::from("wcet_results.json");

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
            "--output" | "-o" => {
                i += 1;
                if i < args.len() {
                    output = PathBuf::from(&args[i]);
                }
            }
            _ => {
                eprintln!("Warning: Unknown option '{}'", args[i]);
            }
        }
        i += 1;
    }

    let final_platform = platform.or(Some("cortex-m4".to_string()));

    Ok(Config {
        platform: final_platform,
        board,
        output,
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
    println!("LALE - LLVM-based WCET Analysis (Inkwell)");
    println!("==========================================");
    println!();
    println!("Configuration:");
    println!("  Directory: {}", dir.display());

    if let Some(ref board) = config.board {
        println!("  Board: {}", board);
    } else if let Some(ref platform) = config.platform {
        println!("  Platform: {}", platform);
    }

    println!("  Output: {}", config.output.display());
    println!();

    // Find all .ll files in directory
    let ll_files = find_ll_files(&dir)?;
    if ll_files.is_empty() {
        anyhow::bail!("No .ll files found in directory: {}", dir.display());
    }

    println!("Found {} LLVM IR file(s)", ll_files.len());
    println!();

    // Select platform
    let platform_name = config
        .platform
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No platform specified"))?;
    let platform = select_platform(platform_name)?;

    // Parse all modules and analyze
    let mut all_results = Vec::new();

    for ll_file in &ll_files {
        println!("Analyzing: {}", ll_file.display());
        match InkwellParser::parse_file(ll_file) {
            Ok((_context, module)) => {
                let mut file_results = Vec::new();

                // Iterate through all functions
                for function in module.get_functions() {
                    let func_name = function
                        .get_name()
                        .to_str()
                        .unwrap_or("unknown")
                        .to_string();

                    // Skip intrinsics and declarations
                    if func_name.starts_with("llvm.") || function.count_basic_blocks() == 0 {
                        continue;
                    }

                    // Build CFG and calculate timing
                    let cfg = lale::InkwellCFG::from_function(&function);
                    let timings = InkwellTimingCalculator::calculate_block_timings(
                        &function, &cfg, &platform,
                    );

                    // Sum up all block timings for a simple WCET estimate
                    let total_cycles: u64 = timings.values().sum();
                    let wcet_us = total_cycles as f64 / platform.cpu_frequency_mhz as f64;

                    file_results.push((func_name.clone(), total_cycles, wcet_us));
                    println!(
                        "  {} : {} cycles ({:.2} us)",
                        func_name, total_cycles, wcet_us
                    );
                }

                all_results.extend(file_results);
            }
            Err(e) => {
                eprintln!("  Warning: Failed to parse {}: {}", ll_file.display(), e);
            }
        }
        println!();
    }

    println!("Total functions analyzed: {}", all_results.len());
    println!();

    // Export results to JSON
    let json_output = serde_json::json!({
        "platform": platform_name,
        "cpu_frequency_mhz": platform.cpu_frequency_mhz,
        "functions": all_results.iter().map(|(name, cycles, us)| {
            serde_json::json!({
                "name": name,
                "wcet_cycles": cycles,
                "wcet_us": us
            })
        }).collect::<Vec<_>>()
    });

    let json_str = serde_json::to_string_pretty(&json_output)?;
    std::fs::write(&config.output, &json_str)
        .with_context(|| format!("Failed to write to {}", config.output.display()))?;

    println!("✓ Analysis complete!");
    println!("✓ Results exported to: {}", config.output.display());

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
    println!("LALE - LLVM-based WCET Analysis (Inkwell)");
    println!();
    println!("USAGE:");
    println!("    lale analyze <directory> [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --platform, -p <platform>    Target platform (default: cortex-m4)");
    println!("    --output, -o <file>          Output file (default: wcet_results.json)");
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
    println!("    lale analyze ./data/armv7e-m --platform cortex-m4");
    println!("    lale analyze ./ir_files --platform cortex-m7 --output results.json");
    println!();
    println!("BOARD CONFIGURATION COMMANDS:");
    println!("    lale list-boards                List available board configurations");
    println!("    lale validate-board <name>      Validate a board configuration");
    println!("    lale export-board <name>        Export resolved board configuration");
    println!();
    println!("OTHER COMMANDS:");
    println!("    lale help              Show this help message");
    println!("    lale version           Show version information");
}
