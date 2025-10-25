use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;

/// Benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    
    /// Computed WCET (cycles)
    pub wcet_cycles: u64,
    
    /// Analysis time (milliseconds)
    pub analysis_time_ms: u64,
    
    /// Platform used
    pub platform: String,
    
    /// Success status
    pub success: bool,
    
    /// Error message if failed
    pub error: Option<String>,
}

/// Benchmark suite
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkSuite {
    TACLeBench,
    Malardalen,
    MRTC,
}

impl BenchmarkSuite {
    pub fn name(&self) -> &str {
        match self {
            Self::TACLeBench => "tacle-bench",
            Self::Malardalen => "malardalen",
            Self::MRTC => "mrtc",
        }
    }
    
    pub fn directory(&self) -> &str {
        match self {
            Self::TACLeBench => "benchmarks/tacle-bench",
            Self::Malardalen => "benchmarks/malardalen",
            Self::MRTC => "benchmarks/mrtc",
        }
    }
}

/// Benchmark descriptor
#[derive(Debug, Clone)]
pub struct Benchmark {
    /// Name
    pub name: String,
    
    /// Suite
    pub suite: BenchmarkSuite,
    
    /// LLVM IR file path
    pub ir_path: PathBuf,
    
    /// Expected WCET (if known)
    pub expected_wcet: Option<u64>,
}

impl Benchmark {
    pub fn new(name: String, suite: BenchmarkSuite, ir_path: PathBuf) -> Self {
        Self {
            name,
            suite,
            ir_path,
            expected_wcet: None,
        }
    }
    
    pub fn with_expected_wcet(mut self, wcet: u64) -> Self {
        self.expected_wcet = Some(wcet);
        self
    }
}

/// Benchmark runner
pub struct BenchmarkRunner {
    /// Platform configuration path
    platform_config: PathBuf,
    
    /// Results directory
    results_dir: PathBuf,
}

impl BenchmarkRunner {
    pub fn new(platform_config: PathBuf, results_dir: PathBuf) -> Self {
        Self {
            platform_config,
            results_dir,
        }
    }
    
    /// Run single benchmark
    pub fn run_benchmark(&self, benchmark: &Benchmark) -> Result<BenchmarkResult> {
        let start = Instant::now();
        
        // Load platform configuration
        let config = lale::config::ConfigLoader::load_from_file(&self.platform_config)?;
        
        // Parse LLVM IR
        let module = lale::ir::parser::IRParser::parse_file(&benchmark.ir_path)?;
        
        // Build CFG
        let cfg = lale::ir::cfg::CFG::from_module(&module)?;
        
        // Run WCET analysis
        let result = match self.analyze(&cfg, &config) {
            Ok(wcet) => {
                let elapsed = start.elapsed();
                BenchmarkResult {
                    name: benchmark.name.clone(),
                    wcet_cycles: wcet,
                    analysis_time_ms: elapsed.as_millis() as u64,
                    platform: self.platform_config.display().to_string(),
                    success: true,
                    error: None,
                }
            }
            Err(e) => {
                let elapsed = start.elapsed();
                BenchmarkResult {
                    name: benchmark.name.clone(),
                    wcet_cycles: 0,
                    analysis_time_ms: elapsed.as_millis() as u64,
                    platform: self.platform_config.display().to_string(),
                    success: false,
                    error: Some(e.to_string()),
                }
            }
        };
        
        Ok(result)
    }
    
    /// Analyze CFG and compute WCET
    fn analyze(&self, cfg: &lale::ir::cfg::CFG, config: &lale::config::types::PlatformConfiguration) -> Result<u64> {
        // Convert config to platform config
        let platform_config = lale::config::ConfigLoader::to_platform_config(config);
        
        // Build AEG
        let mut builder = lale::aeg::builder::AEGBuilder::new(
            cfg.clone(),
            platform_config,
            1000, // state limit
        );
        
        let aeg = builder.build()?;
        
        // Run IPET analysis
        let analyzer = lale::analysis::ipet_aeg::IPETAnalyzer::new(aeg);
        let wcet = analyzer.compute_wcet()?;
        
        Ok(wcet)
    }
    
    /// Run multiple benchmarks
    pub fn run_suite(&self, benchmarks: &[Benchmark]) -> Result<Vec<BenchmarkResult>> {
        let mut results = Vec::new();
        
        for benchmark in benchmarks {
            println!("Running benchmark: {}", benchmark.name);
            match self.run_benchmark(benchmark) {
                Ok(result) => {
                    if result.success {
                        println!("  ✓ WCET: {} cycles ({} ms)", result.wcet_cycles, result.analysis_time_ms);
                    } else {
                        println!("  ✗ Failed: {}", result.error.as_ref().unwrap());
                    }
                    results.push(result);
                }
                Err(e) => {
                    println!("  ✗ Error: {}", e);
                }
            }
        }
        
        Ok(results)
    }
    
    /// Save results to JSON
    pub fn save_results(&self, results: &[BenchmarkResult], filename: &str) -> Result<()> {
        std::fs::create_dir_all(&self.results_dir)?;
        let path = self.results_dir.join(filename);
        let json = serde_json::to_string_pretty(results)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

/// Discover benchmarks in directory
pub fn discover_benchmarks(suite: BenchmarkSuite, ir_dir: &Path) -> Result<Vec<Benchmark>> {
    let mut benchmarks = Vec::new();
    let suite_dir = ir_dir.join(suite.name());
    
    if !suite_dir.exists() {
        return Ok(benchmarks);
    }
    
    for entry in walkdir::WalkDir::new(suite_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("ll") {
            let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            
            benchmarks.push(Benchmark::new(name, suite, path.to_path_buf()));
        }
    }
    
    Ok(benchmarks)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_benchmark_suite_names() {
        assert_eq!(BenchmarkSuite::TACLeBench.name(), "tacle-bench");
        assert_eq!(BenchmarkSuite::Malardalen.name(), "malardalen");
        assert_eq!(BenchmarkSuite::MRTC.name(), "mrtc");
    }
    
    #[test]
    fn test_benchmark_creation() {
        let bench = Benchmark::new(
            "test".to_string(),
            BenchmarkSuite::TACLeBench,
            PathBuf::from("test.ll"),
        );
        
        assert_eq!(bench.name, "test");
        assert_eq!(bench.suite, BenchmarkSuite::TACLeBench);
        assert!(bench.expected_wcet.is_none());
    }
}
