use serde::{Deserialize, Serialize};

/// Complete platform configuration (hierarchical)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfiguration {
    /// ISA-level configuration
    pub isa: ISAConfig,

    /// Core-level configuration
    pub core: CoreConfig,

    /// SoC-level configuration (optional)
    pub soc: Option<SoCConfig>,

    /// Board-level configuration (optional)
    pub board: Option<BoardConfig>,
}

/// ISA (Instruction Set Architecture) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ISAConfig {
    /// ISA name (e.g., "armv7e-m", "riscv32")
    pub name: String,

    /// Instruction timings
    pub instruction_timings: InstructionTimings,
}

/// Core-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreConfig {
    /// Core name (e.g., "cortex-m4", "cortex-a53")
    pub name: String,

    /// Pipeline configuration
    pub pipeline: PipelineConfig,

    /// Cache configuration
    pub cache: CacheConfiguration,

    /// Memory configuration
    pub memory: MemoryConfiguration,
}

/// SoC (System on Chip) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoCConfig {
    /// SoC name (e.g., "stm32f746", "bcm2837")
    pub name: String,

    /// CPU frequency in MHz
    pub cpu_frequency_mhz: u32,

    /// Memory regions
    pub memory_regions: Vec<MemoryRegion>,
}

/// Board-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardConfig {
    /// Board name (e.g., "stm32f746-discovery", "raspberry-pi-3")
    pub name: String,

    /// External memory configuration
    pub external_memory: Option<ExternalMemoryConfig>,
}

/// Pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Number of pipeline stages
    pub stages: usize,

    /// Pipeline type
    pub pipeline_type: PipelineType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineType {
    InOrder,
    OutOfOrder,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfiguration {
    /// Instruction cache
    pub instruction_cache: Option<CacheLevelConfig>,

    /// Data cache
    pub data_cache: Option<CacheLevelConfig>,

    /// L2 cache (optional)
    pub l2_cache: Option<CacheLevelConfig>,
}

/// Single cache level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheLevelConfig {
    /// Size in KB
    pub size_kb: usize,

    /// Line size in bytes
    pub line_size_bytes: usize,

    /// Associativity
    pub associativity: usize,

    /// Replacement policy
    pub replacement_policy: ReplacementPolicy,

    /// Hit latency in cycles
    pub hit_latency: u32,

    /// Miss latency in cycles
    pub miss_latency: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ReplacementPolicy {
    LRU,
    PLRU,
    FIFO,
    Random,
}

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfiguration {
    /// Load buffer size
    pub load_buffer_size: usize,

    /// Store buffer size
    pub store_buffer_size: usize,

    /// Memory latency
    pub memory_latency: MemoryLatencyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MemoryLatencyConfig {
    Fixed { cycles: u32 },
    Variable { min: u32, max: u32 },
}

/// Memory region
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRegion {
    /// Region name
    pub name: String,

    /// Start address
    pub start: u64,

    /// Size in bytes
    pub size: u64,

    /// Access latency in cycles
    pub latency: u32,
}

/// External memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalMemoryConfig {
    /// Type (e.g., "SDRAM", "Flash")
    pub memory_type: String,

    /// Size in MB
    pub size_mb: usize,

    /// Access latency in cycles
    pub latency: u32,
}

/// Instruction timings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionTimings {
    /// ALU operations
    pub alu: u32,

    /// Load operations
    pub load: u32,

    /// Store operations
    pub store: u32,

    /// Branch operations
    pub branch: u32,

    /// Multiply operations
    pub multiply: u32,

    /// Divide operations
    pub divide: u32,
}

impl Default for InstructionTimings {
    fn default() -> Self {
        Self {
            alu: 1,
            load: 2,
            store: 2,
            branch: 1,
            multiply: 3,
            divide: 12,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_timings_default() {
        let timings = InstructionTimings::default();
        assert_eq!(timings.alu, 1);
        assert_eq!(timings.load, 2);
    }

    #[test]
    fn test_pipeline_config() {
        let config = PipelineConfig {
            stages: 5,
            pipeline_type: PipelineType::InOrder,
        };
        assert_eq!(config.stages, 5);
    }
}
