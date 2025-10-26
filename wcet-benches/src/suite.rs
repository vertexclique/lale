use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct BenchmarkSuite {
    pub name: String,
    pub version: String,
    pub benchmarks: Vec<BenchmarkInfo>,
}

impl BenchmarkSuite {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .context("Failed to read benchmark suite metadata")?;
        let suite: BenchmarkSuite =
            serde_json::from_str(&content).context("Failed to parse benchmark suite metadata")?;
        Ok(suite)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkInfo {
    pub id: String,
    pub name: String,
    pub category: BenchmarkCategory,
    pub source_files: Vec<String>,
    pub entry_function: String,
    pub properties: BenchmarkProperties,
    pub flow_facts: Option<FlowFacts>,
    pub reference_wcet: Option<ReferenceWCET>,
    pub measured_execution_time: Option<MeasuredTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasuredTime {
    pub min_cycles: u64,
    pub max_cycles: u64,
    pub avg_cycles: u64,
    pub platform: String,
    pub num_runs: usize,
    pub input_description: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum BenchmarkCategory {
    Kernel,
    Sequential,
    Parallel,
    Test,
    Numeric,
    Control,
    Mixed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkProperties {
    pub loc: usize,
    pub has_loops: bool,
    pub has_nested_loops: bool,
    pub has_recursion: bool,
    pub has_arrays: bool,
    pub has_bitops: bool,
    pub is_single_path: bool,
    pub max_loop_depth: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowFacts {
    pub loop_bounds: HashMap<String, LoopBound>,
    pub infeasible_paths: Vec<InfeasiblePath>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopBound {
    pub location: String,
    pub min_iterations: u32,
    pub max_iterations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InfeasiblePath {
    pub source: String,
    pub target: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceWCET {
    pub tool: String,
    pub platform: String,
    pub cycles: u64,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_category() {
        let cat = BenchmarkCategory::Kernel;
        let json = serde_json::to_string(&cat).unwrap();
        let parsed: BenchmarkCategory = serde_json::from_str(&json).unwrap();
        assert_eq!(cat, parsed);
    }

    #[test]
    fn test_benchmark_info_serialization() {
        let info = BenchmarkInfo {
            id: "test-bench".to_string(),
            name: "test".to_string(),
            category: BenchmarkCategory::Test,
            source_files: vec!["test.c".to_string()],
            entry_function: "main".to_string(),
            properties: BenchmarkProperties {
                loc: 100,
                has_loops: true,
                has_nested_loops: false,
                has_recursion: false,
                has_arrays: false,
                has_bitops: false,
                is_single_path: false,
                max_loop_depth: 1,
            },
            flow_facts: None,
            reference_wcet: None,
            measured_execution_time: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        let parsed: BenchmarkInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(info.id, parsed.id);
    }
}
