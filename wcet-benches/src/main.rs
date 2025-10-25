use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use wcet_benches::{discover_benchmarks, Benchmark, BenchmarkRunner, BenchmarkSuite};

#[derive(Parser)]
#[command(name = "bench-runner")]
#[command(about = "WCET Benchmark Runner", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run benchmarks
    Run {
        /// Benchmark suite to run
        #[arg(short, long, value_enum)]
        suite: Option<Suite>,
        
        /// Specific benchmark name
        #[arg(short, long)]
        benchmark: Option<String>,
        
        /// Platform configuration file
        #[arg(short, long, default_value = "../config/platforms/stm32f746-discovery.toml")]
        platform: PathBuf,
        
        /// LLVM IR directory
        #[arg(short, long, default_value = "llvm-ir")]
        ir_dir: PathBuf,
        
        /// Results directory
        #[arg(short, long, default_value = "results")]
        results_dir: PathBuf,
    },
    
    /// List available benchmarks
    List {
        /// LLVM IR directory
        #[arg(short, long, default_value = "llvm-ir")]
        ir_dir: PathBuf,
    },
    
    /// Compare results with reference
    Compare {
        /// Lale results file
        #[arg(short, long)]
        lale: PathBuf,
        
        /// Reference results file
        #[arg(short, long)]
        reference: PathBuf,
    },
}

#[derive(clap::ValueEnum, Clone, Copy)]
enum Suite {
    TACLeBench,
    Malardalen,
    MRTC,
    All,
}

impl Suite {
    fn to_benchmark_suite(self) -> Option<BenchmarkSuite> {
        match self {
            Self::TACLeBench => Some(BenchmarkSuite::TACLeBench),
            Self::Malardalen => Some(BenchmarkSuite::Malardalen),
            Self::MRTC => Some(BenchmarkSuite::MRTC),
            Self::All => None,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Run {
            suite,
            benchmark,
            platform,
            ir_dir,
            results_dir,
        } => {
            run_benchmarks(suite, benchmark, platform, ir_dir, results_dir)?;
        }
        Commands::List { ir_dir } => {
            list_benchmarks(ir_dir)?;
        }
        Commands::Compare { lale, reference } => {
            compare_results(lale, reference)?;
        }
    }
    
    Ok(())
}

fn run_benchmarks(
    suite: Option<Suite>,
    benchmark_name: Option<String>,
    platform: PathBuf,
    ir_dir: PathBuf,
    results_dir: PathBuf,
) -> Result<()> {
    let runner = BenchmarkRunner::new(platform, results_dir);
    
    // Discover benchmarks
    let suites = match suite {
        Some(Suite::All) | None => vec![
            BenchmarkSuite::TACLeBench,
            BenchmarkSuite::Malardalen,
            BenchmarkSuite::MRTC,
        ],
        Some(s) => vec![s.to_benchmark_suite().unwrap()],
    };
    
    let mut all_benchmarks = Vec::new();
    for suite in suites {
        let benchmarks = discover_benchmarks(suite, &ir_dir)?;
        all_benchmarks.extend(benchmarks);
    }
    
    // Filter by name if specified
    let benchmarks: Vec<Benchmark> = if let Some(name) = benchmark_name {
        all_benchmarks
            .into_iter()
            .filter(|b| b.name == name)
            .collect()
    } else {
        all_benchmarks
    };
    
    if benchmarks.is_empty() {
        println!("No benchmarks found");
        return Ok(());
    }
    
    println!("Running {} benchmarks...\n", benchmarks.len());
    
    // Run benchmarks
    let results = runner.run_suite(&benchmarks)?;
    
    // Print summary
    println!("\n=== Summary ===");
    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;
    println!("Total: {}", results.len());
    println!("Successful: {}", successful);
    println!("Failed: {}", failed);
    
    if successful > 0 {
        let total_time: u64 = results.iter().map(|r| r.analysis_time_ms).sum();
        let avg_time = total_time / successful as u64;
        println!("Average analysis time: {} ms", avg_time);
    }
    
    // Save results
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
    let filename = format!("results_{}.json", timestamp);
    runner.save_results(&results, &filename)?;
    println!("\nResults saved to: results/{}", filename);
    
    Ok(())
}

fn list_benchmarks(ir_dir: PathBuf) -> Result<()> {
    let suites = vec![
        BenchmarkSuite::TACLeBench,
        BenchmarkSuite::Malardalen,
        BenchmarkSuite::MRTC,
    ];
    
    for suite in suites {
        let benchmarks = discover_benchmarks(suite, &ir_dir)?;
        
        if !benchmarks.is_empty() {
            println!("\n{} ({} benchmarks):", suite.name(), benchmarks.len());
            for bench in benchmarks {
                println!("  - {}", bench.name);
            }
        }
    }
    
    Ok(())
}

fn compare_results(lale_path: PathBuf, reference_path: PathBuf) -> Result<()> {
    use std::fs;
    
    let lale_json = fs::read_to_string(lale_path)?;
    let reference_json = fs::read_to_string(reference_path)?;
    
    let lale_results: Vec<wcet_benches::BenchmarkResult> = serde_json::from_str(&lale_json)?;
    let reference_results: Vec<wcet_benches::BenchmarkResult> = serde_json::from_str(&reference_json)?;
    
    println!("=== Comparison ===\n");
    println!("{:<30} {:>12} {:>12} {:>10}", "Benchmark", "Lale", "Reference", "Ratio");
    println!("{}", "-".repeat(70));
    
    for lale_result in &lale_results {
        if let Some(ref_result) = reference_results.iter().find(|r| r.name == lale_result.name) {
            if lale_result.success && ref_result.success {
                let ratio = lale_result.wcet_cycles as f64 / ref_result.wcet_cycles as f64;
                println!(
                    "{:<30} {:>12} {:>12} {:>9.2}x",
                    lale_result.name,
                    lale_result.wcet_cycles,
                    ref_result.wcet_cycles,
                    ratio
                );
            }
        }
    }
    
    Ok(())
}
