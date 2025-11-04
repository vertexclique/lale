use ahash::AHashMap;
use serde::{Deserialize, Serialize};

/// Real-time task model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub function: String,
    pub wcet_cycles: u64,
    pub wcet_us: f64,
    pub period_us: Option<f64>,
    pub deadline_us: Option<f64>,
    pub priority: Option<u8>,
    pub preemptible: bool,
    pub dependencies: Vec<String>,
}

/// Task attributes from annotations
#[derive(Debug, Clone)]
pub struct TaskAttributes {
    pub name: String,
    pub period: Option<f64>,
    pub deadline: Option<f64>,
    pub priority: Option<u8>,
    pub preemptible: Option<bool>,
}

/// Task extractor - LEGACY (llvm_ir based)
pub struct TaskExtractor;

impl TaskExtractor {
    /// Convert cycles to microseconds
    pub fn cycles_to_us(cycles: u64, cpu_freq_mhz: u32) -> f64 {
        cycles as f64 / cpu_freq_mhz as f64
    }
}
