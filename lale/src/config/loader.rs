use super::types::*;
use std::fs;
use std::path::Path;

/// Configuration loader with hierarchical composition
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load complete platform configuration from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<PlatformConfiguration, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read config file: {}", e))?;

        toml::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))
    }

    /// Load configuration with hierarchical composition
    /// Loads ISA → Core → SoC → Board in sequence
    pub fn load_hierarchical(
        isa_path: &str,
        core_path: &str,
        soc_path: Option<&str>,
        board_path: Option<&str>,
    ) -> Result<PlatformConfiguration, String> {
        // Load ISA config
        let isa: ISAConfig = Self::load_toml(isa_path)?;

        // Load Core config
        let core: CoreConfig = Self::load_toml(core_path)?;

        // Load optional SoC config
        let soc = soc_path
            .map(|path| Self::load_toml::<SoCConfig>(path))
            .transpose()?;

        // Load optional Board config
        let board = board_path
            .map(|path| Self::load_toml::<BoardConfig>(path))
            .transpose()?;

        Ok(PlatformConfiguration {
            isa,
            core,
            soc,
            board,
        })
    }

    /// Load TOML file
    fn load_toml<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;

        toml::from_str(&content).map_err(|e| format!("Failed to parse {}: {}", path, e))
    }

    /// Convert to microarch PlatformConfig
    pub fn to_platform_config(
        config: &PlatformConfiguration,
    ) -> crate::microarch::state::PlatformConfig {
        use crate::microarch::state::*;

        crate::microarch::state::PlatformConfig {
            pipeline_depth: config.core.pipeline.stages,
            cache_config: CacheConfig {
                instruction_cache: config.core.cache.instruction_cache.as_ref().map(|c| {
                    CacheLevelConfig {
                        size_kb: c.size_kb,
                        line_size_bytes: c.line_size_bytes,
                        associativity: c.associativity,
                        replacement_policy: match c.replacement_policy {
                            super::types::ReplacementPolicy::LRU => ReplacementPolicy::LRU,
                            super::types::ReplacementPolicy::PLRU => ReplacementPolicy::PLRU,
                            super::types::ReplacementPolicy::FIFO => ReplacementPolicy::FIFO,
                            super::types::ReplacementPolicy::Random => ReplacementPolicy::LRU, // Default
                        },
                    }
                }),
                data_cache: config
                    .core
                    .cache
                    .data_cache
                    .as_ref()
                    .map(|c| CacheLevelConfig {
                        size_kb: c.size_kb,
                        line_size_bytes: c.line_size_bytes,
                        associativity: c.associativity,
                        replacement_policy: match c.replacement_policy {
                            super::types::ReplacementPolicy::LRU => ReplacementPolicy::LRU,
                            super::types::ReplacementPolicy::PLRU => ReplacementPolicy::PLRU,
                            super::types::ReplacementPolicy::FIFO => ReplacementPolicy::FIFO,
                            super::types::ReplacementPolicy::Random => ReplacementPolicy::LRU,
                        },
                    }),
            },
            memory_config: MemoryConfig {
                load_buffer_size: config.core.memory.load_buffer_size,
                store_buffer_size: config.core.memory.store_buffer_size,
                memory_latency: match &config.core.memory.memory_latency {
                    super::types::MemoryLatencyConfig::Fixed { cycles } => {
                        MemoryLatency::Fixed { cycles: *cycles }
                    }
                    super::types::MemoryLatencyConfig::Variable { min, max } => {
                        MemoryLatency::Variable {
                            min: *min,
                            max: *max,
                        }
                    }
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_platform_config() {
        use crate::config::types::ReplacementPolicy;

        let config = PlatformConfiguration {
            isa: ISAConfig {
                name: "armv7e-m".to_string(),
                instruction_timings: InstructionTimings::default(),
            },
            core: CoreConfig {
                name: "cortex-m4".to_string(),
                pipeline: PipelineConfig {
                    stages: 3,
                    pipeline_type: PipelineType::InOrder,
                },
                cache: CacheConfiguration {
                    instruction_cache: Some(CacheLevelConfig {
                        size_kb: 16,
                        line_size_bytes: 32,
                        associativity: 4,
                        replacement_policy: ReplacementPolicy::LRU,
                        hit_latency: 1,
                        miss_latency: 10,
                    }),
                    data_cache: None,
                    l2_cache: None,
                },
                memory: MemoryConfiguration {
                    load_buffer_size: 4,
                    store_buffer_size: 4,
                    memory_latency: MemoryLatencyConfig::Fixed { cycles: 10 },
                },
            },
            soc: None,
            board: None,
        };

        let platform_config = ConfigLoader::to_platform_config(&config);
        assert_eq!(platform_config.pipeline_depth, 3);
    }
}
