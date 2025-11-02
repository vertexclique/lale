use super::types::*;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration loader with hierarchical composition
pub struct ConfigLoader;

/// Configuration manager with caching and inheritance resolution
pub struct ConfigManager {
    config_dir: PathBuf,
    cache: HashMap<String, PlatformConfiguration>,
}

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

impl ConfigManager {
    /// Create new configuration manager
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            config_dir,
            cache: HashMap::new(),
        }
    }

    /// Load platform configuration by name with inheritance resolution
    /// Supports paths like "platforms/stm32f746-discovery" or "cores/cortex-m4"
    pub fn load_platform(&mut self, name: &str) -> Result<PlatformConfiguration, String> {
        self.load_platform_with_chain(name, &mut Vec::new())
    }

    /// Internal method to load platform with circular dependency detection
    fn load_platform_with_chain(
        &mut self,
        name: &str,
        chain: &mut Vec<String>,
    ) -> Result<PlatformConfiguration, String> {
        // Check for circular dependency
        if chain.contains(&name.to_string()) {
            return Err(format!(
                "Circular dependency detected: {} -> {}",
                chain.join(" -> "),
                name
            ));
        }

        // Check cache first
        if let Some(config) = self.cache.get(name) {
            return Ok(config.clone());
        }

        // Add to chain for circular dependency detection
        chain.push(name.to_string());

        // Build full path
        let path = self.config_dir.join(format!("{}.toml", name));

        // Load configuration
        let mut config = ConfigLoader::load_from_file(&path)?;

        // Handle inheritance if present
        if let Some(ref board) = config.board {
            if let Some(ref parent_path) = board.inherits {
                // Load parent configuration
                let parent_config = self.load_platform_with_chain(parent_path, chain)?;

                // Merge parent into current config
                config = self.merge_configs(parent_config, config)?;
            }
        }

        // Validate
        self.validate(&config)?;

        // Remove from chain
        chain.pop();

        // Cache and return
        self.cache.insert(name.to_string(), config.clone());
        Ok(config)
    }

    /// Merge parent and child configurations (child overrides parent)
    fn merge_configs(
        &self,
        parent: PlatformConfiguration,
        child: PlatformConfiguration,
    ) -> Result<PlatformConfiguration, String> {
        // Child values take precedence, but we keep parent values if child doesn't specify
        Ok(PlatformConfiguration {
            isa: child.isa,                // ISA from child
            core: child.core,              // Core from child
            soc: child.soc.or(parent.soc), // Use child SoC if present, otherwise parent
            board: child.board,            // Board from child
        })
    }

    /// List available platforms
    /// Only returns complete platform configurations (those in platforms/ directory)
    /// Core configs are incomplete and meant to be referenced by platforms
    pub fn list_platforms(&self) -> Result<Vec<String>, String> {
        let mut platforms = Vec::new();

        // Only scan platforms directory - these are complete configurations
        let platforms_dir = self.config_dir.join("platforms");
        if platforms_dir.exists() {
            self.scan_directory(&platforms_dir, "platforms", &mut platforms)?;
        }

        Ok(platforms)
    }

    /// Scan directory for TOML files
    fn scan_directory(
        &self,
        dir: &Path,
        prefix: &str,
        results: &mut Vec<String>,
    ) -> Result<(), String> {
        let entries = fs::read_dir(dir)
            .map_err(|e| format!("Failed to read directory {}: {}", dir.display(), e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("toml") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    results.push(format!("{}/{}", prefix, name));
                }
            }
        }

        Ok(())
    }

    /// Validate platform configuration
    pub fn validate(&self, config: &PlatformConfiguration) -> Result<(), String> {
        let mut errors = Vec::new();

        // Validate cache sizes are powers of 2
        if let Some(ref icache) = config.core.cache.instruction_cache {
            if !icache.size_kb.is_power_of_two() {
                errors.push(format!(
                    "Instruction cache size {} KB is not a power of 2",
                    icache.size_kb
                ));
            }
            if !icache.line_size_bytes.is_power_of_two() {
                errors.push(format!(
                    "Instruction cache line size {} bytes is not a power of 2",
                    icache.line_size_bytes
                ));
            }
            // Validate associativity
            let total_lines = (icache.size_kb * 1024) / icache.line_size_bytes;
            if icache.associativity > total_lines {
                errors.push(format!(
                    "Instruction cache associativity {} exceeds total lines {}",
                    icache.associativity, total_lines
                ));
            }
        }

        if let Some(ref dcache) = config.core.cache.data_cache {
            if !dcache.size_kb.is_power_of_two() {
                errors.push(format!(
                    "Data cache size {} KB is not a power of 2",
                    dcache.size_kb
                ));
            }
            if !dcache.line_size_bytes.is_power_of_two() {
                errors.push(format!(
                    "Data cache line size {} bytes is not a power of 2",
                    dcache.line_size_bytes
                ));
            }
            let total_lines = (dcache.size_kb * 1024) / dcache.line_size_bytes;
            if dcache.associativity > total_lines {
                errors.push(format!(
                    "Data cache associativity {} exceeds total lines {}",
                    dcache.associativity, total_lines
                ));
            }
        }

        // Validate SoC frequency if present
        if let Some(ref soc) = config.soc {
            if soc.cpu_frequency_mhz == 0 {
                errors.push("CPU frequency must be greater than 0".to_string());
            }

            // Check for overlapping memory regions
            for i in 0..soc.memory_regions.len() {
                for j in (i + 1)..soc.memory_regions.len() {
                    let r1 = &soc.memory_regions[i];
                    let r2 = &soc.memory_regions[j];

                    let r1_end = r1.start + r1.size;
                    let r2_end = r2.start + r2.size;

                    if r1.start < r2_end && r1_end > r2.start {
                        errors.push(format!(
                            "Memory regions '{}' and '{}' overlap",
                            r1.name, r2.name
                        ));
                    }
                }
            }
        }

        // Validate pipeline stages
        if config.core.pipeline.stages == 0 {
            errors.push("Pipeline stages must be greater than 0".to_string());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(format!("Validation errors:\n  - {}", errors.join("\n  - ")))
        }
    }

    /// Export resolved configuration to TOML string
    pub fn export_platform(&self, config: &PlatformConfiguration) -> Result<String, String> {
        toml::to_string_pretty(config).map_err(|e| format!("Failed to serialize config: {}", e))
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

    #[test]
    fn test_config_manager_validation() {
        let manager = ConfigManager::new(PathBuf::from("config"));

        // Test valid config
        let valid_config = PlatformConfiguration {
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
            soc: Some(SoCConfig {
                name: "test-soc".to_string(),
                cpu_frequency_mhz: 100,
                memory_regions: vec![],
            }),
            board: None,
        };

        assert!(manager.validate(&valid_config).is_ok());

        // Test invalid cache size (not power of 2)
        let mut invalid_config = valid_config.clone();
        if let Some(ref mut icache) = invalid_config.core.cache.instruction_cache {
            icache.size_kb = 15; // Not a power of 2
        }
        assert!(manager.validate(&invalid_config).is_err());
    }

    #[test]
    fn test_config_manager_list_platforms() {
        let manager = ConfigManager::new(PathBuf::from("config"));
        let platforms = manager.list_platforms();

        // Should succeed even if directories don't exist
        assert!(platforms.is_ok());
    }
}
