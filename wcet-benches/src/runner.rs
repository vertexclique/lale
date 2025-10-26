use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::Instant;
use serde::{Deserialize, Serialize};
use crate::suite::BenchmarkSuite;
use crate::flow_facts::FlowFacts;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub name: String,
    pub wcet_cycles: u64,
    pub reference_wcet: Option<u64>,
    pub analysis_time_ms: u64,
    pub success: bool,
    pub error: Option<String>,
    pub details: ResultDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultDetails {
    pub basic_blocks: usize,
    pub cfg_edges: usize,
    pub accuracy: Option<f64>,
    pub loop_bounds_found: usize,
    pub entry_point: Option<String>,
}

pub struct BenchmarkRunner {
    platform_config_path: String,
    metadata: Option<BenchmarkSuite>,
}

impl BenchmarkRunner {
    pub fn new(platform_config: String) -> Self {
        // Try to load validation metadata
        let metadata = Self::load_metadata("data/metadata/validation.json").ok();
        
        Self {
            platform_config_path: platform_config,
            metadata,
        }
    }
    
    fn load_metadata(path: &str) -> Result<BenchmarkSuite> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read metadata file")?;
        let suite: BenchmarkSuite = serde_json::from_str(&content)
            .context("Failed to parse metadata JSON")?;
        Ok(suite)
    }
    
    fn get_reference_wcet(&self, benchmark_name: &str) -> Option<u64> {
        self.metadata.as_ref().and_then(|meta| {
            meta.benchmarks.iter()
                .find(|b| b.name == benchmark_name)
                .and_then(|b| b.reference_wcet.as_ref())
                .map(|r| r.cycles)
        })
    }
    
    pub fn run_benchmark(&self, name: &str, ir_path: &Path) -> BenchmarkResult {
        let start = Instant::now();
        let reference_wcet = self.get_reference_wcet(name);
        
        match self.analyze_internal(ir_path) {
            Ok((wcet, mut details)) => {
                // Calculate accuracy if reference exists
                if let Some(ref_wcet) = reference_wcet {
                    let accuracy = if ref_wcet > 0 {
                        ((wcet as f64 / ref_wcet as f64) * 100.0).min(200.0)
                    } else {
                        0.0
                    };
                    details.accuracy = Some(accuracy);
                }
                
                BenchmarkResult {
                    name: name.to_string(),
                    wcet_cycles: wcet,
                    reference_wcet,
                    analysis_time_ms: start.elapsed().as_millis() as u64,
                    success: true,
                    error: None,
                    details,
                }
            },
            Err(e) => BenchmarkResult {
                name: name.to_string(),
                wcet_cycles: 0,
                reference_wcet,
                analysis_time_ms: start.elapsed().as_millis() as u64,
                success: false,
                error: Some(e.to_string()),
                details: ResultDetails::default(),
            },
        }
    }
    
    fn analyze_internal(&self, ir_path: &Path) -> Result<(u64, ResultDetails)> {
        // Try to find corresponding source file
        let source_path = self.find_source_file(ir_path);
        let flow_facts = source_path
            .and_then(|p| FlowFacts::parse_from_source(&p).ok());
        
        // Parse LLVM IR
        let module = lale::ir::parser::IRParser::parse_file(ir_path)
            .context("Failed to parse LLVM IR")?;
        
        // Find function to analyze
        let function = if let Some(ref facts) = flow_facts {
            if let Some(ref entry) = facts.entry_point {
                // Use entry point from flow facts
                module.functions.iter()
                    .find(|f| f.name == *entry)
                    .or_else(|| module.functions.iter().find(|f| f.name == "main"))
                    .or_else(|| module.functions.first())
            } else {
                module.functions.iter().find(|f| f.name == "main")
                    .or_else(|| module.functions.first())
            }
        } else {
            module.functions.iter().find(|f| f.name == "main")
                .or_else(|| module.functions.first())
        }.context("No functions found in module")?;
        
        // Build CFG
        let cfg = lale::ir::cfg::CFG::from_function(function);
        
        let mut details = ResultDetails {
            basic_blocks: cfg.block_count(),
            cfg_edges: cfg.edge_count(),
            accuracy: None,
            loop_bounds_found: flow_facts.as_ref().map(|f| f.loop_count()).unwrap_or(0),
            entry_point: flow_facts.as_ref().and_then(|f| f.entry_point.clone()),
        };
        
        // Calculate WCET estimate
        // If we have loop bounds, use them for better estimate
        let wcet = if let Some(ref facts) = flow_facts {
            let base_cycles = cfg.block_count() as u64 * 10;
            let loop_cycles = facts.max_total_iterations() as u64 * 5;
            base_cycles + loop_cycles
        } else {
            cfg.block_count() as u64 * 10
        };
        
        Ok((wcet, details))
    }
    
    fn find_source_file(&self, ir_path: &Path) -> Option<PathBuf> {
        // Convert IR path to source path
        // e.g., data/llvm-ir/taclebench/binarysearch.ll -> data/sources/taclebench/kernel/binarysearch/binarysearch.c
        
        let file_stem = ir_path.file_stem()?.to_str()?;
        
        // Check in taclebench directories
        let base = PathBuf::from("data/sources/taclebench");
        
        for category in &["kernel", "sequential", "parallel", "test"] {
            let source_dir = base.join(category).join(file_stem);
            let source_file = source_dir.join(format!("{}.c", file_stem));
            
            if source_file.exists() {
                return Some(source_file);
            }
        }
        
        // Check validation directory
        let validation_path = PathBuf::from("data/sources/validation")
            .join(format!("{}.c", file_stem));
        if validation_path.exists() {
            return Some(validation_path);
        }
        
        None
    }
}
