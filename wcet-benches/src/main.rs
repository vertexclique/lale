use clap::{Parser, Subcommand};
use std::path::PathBuf;
use wcet_benches::{BenchmarkRunner, BenchmarkResult};

#[derive(Parser)]
#[command(name = "bench-runner")]
#[command(about = "WCET Benchmark Runner", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run WCET analysis on benchmarks
    Run {
        /// Platform configuration file
        #[arg(short, long, default_value = "../config/platforms/stm32f746-discovery.toml")]
        platform: String,
        
        /// LLVM IR directory
        #[arg(short, long, default_value = "data/llvm-ir")]
        ir_dir: PathBuf,
        
        /// Specific benchmark name (optional)
        #[arg(short, long)]
        benchmark: Option<String>,
    },
    
    /// List available benchmarks
    List {
        /// LLVM IR directory
        #[arg(short, long, default_value = "data/llvm-ir")]
        ir_dir: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Run { platform, ir_dir, benchmark } => {
            run_benchmarks(platform, ir_dir, benchmark);
        }
        Commands::List { ir_dir } => {
            list_benchmarks(ir_dir);
        }
    }
}

fn run_benchmarks(platform: String, ir_dir: PathBuf, specific_benchmark: Option<String>) {
    println!("=== WCET Benchmark Analysis ===");
    println!("Platform: {}", platform);
    println!("IR Directory: {}", ir_dir.display());
    println!();
    
    let runner = BenchmarkRunner::new(platform);
    let mut results = Vec::new();
    
    // Find all .ll files
    let mut ir_files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&ir_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Recursively search subdirectories
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.extension().and_then(|s| s.to_str()) == Some("ll") {
                            ir_files.push(sub_path);
                        }
                    }
                }
            } else if path.extension().and_then(|s| s.to_str()) == Some("ll") {
                ir_files.push(path);
            }
        }
    }
    
    // Filter out input/data files (they contain no functions)
    ir_files.retain(|p| {
        if let Some(name) = p.file_stem().and_then(|s| s.to_str()) {
            !name.contains("input") && !name.contains("_data")
        } else {
            false
        }
    });
    
    // Filter by specific benchmark if provided
    if let Some(ref name) = specific_benchmark {
        ir_files.retain(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s == name)
                .unwrap_or(false)
        });
    }
    
    println!("Found {} benchmarks\n", ir_files.len());
    
    // Run analysis on each benchmark
    for ir_file in &ir_files {
        let name = ir_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        print!("Analyzing: {} ... ", name);
        std::io::Write::flush(&mut std::io::stdout()).ok();
        
        let result = runner.run_benchmark(&name, ir_file);
        
        if result.success {
            print!("✓ WCET: {} cycles", result.wcet_cycles);
            
            // Show reference comparison if available
            if let Some(ref_wcet) = result.reference_wcet {
                print!(" (ref: {} cycles", ref_wcet);
                if let Some(accuracy) = result.details.accuracy {
                    print!(", {:.1}%", accuracy);
                }
                print!(")");
            }
            
            println!(" ({} ms)", result.analysis_time_ms);
            print!("  Blocks: {}, Edges: {}", 
                result.details.basic_blocks,
                result.details.cfg_edges
            );
            
            // Show flow facts if found
            if result.details.loop_bounds_found > 0 {
                print!(", Loops: {}", result.details.loop_bounds_found);
            }
            if let Some(ref entry) = result.details.entry_point {
                print!(", Entry: {}", entry);
            }
            println!();
        } else {
            println!("✗ Failed");
            if let Some(ref error) = result.error {
                println!("  Error: {}", error);
            }
        }
        
        results.push(result);
    }
    
    // Print summary
    println!("\n=== Summary ===");
    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.len() - successful;
    
    println!("Total: {}", results.len());
    println!("Successful: {} ({:.1}%)", 
        successful, 
        (successful as f64 / results.len() as f64) * 100.0
    );
    println!("Failed: {}", failed);
    
    if successful > 0 {
        let total_time: u64 = results.iter()
            .filter(|r| r.success)
            .map(|r| r.analysis_time_ms)
            .sum();
        let avg_time = total_time / successful as u64;
        println!("Average analysis time: {} ms", avg_time);
    }
    
    // Save results
    let output_path = PathBuf::from("results/benchmark_results.json");
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    
    if let Ok(json) = serde_json::to_string_pretty(&results) {
        if std::fs::write(&output_path, json).is_ok() {
            println!("\nResults saved to: {}", output_path.display());
        }
    }
}

fn list_benchmarks(ir_dir: PathBuf) {
    println!("=== Available Benchmarks ===\n");
    
    if let Ok(entries) = std::fs::read_dir(&ir_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                
                println!("{}:", dir_name);
                
                if let Ok(sub_entries) = std::fs::read_dir(&path) {
                    let mut count = 0;
                    for sub_entry in sub_entries.flatten() {
                        let sub_path = sub_entry.path();
                        if sub_path.extension().and_then(|s| s.to_str()) == Some("ll") {
                            let name = sub_path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown");
                            println!("  - {}", name);
                            count += 1;
                        }
                    }
                    println!("  Total: {}\n", count);
                }
            }
        }
    }
}
