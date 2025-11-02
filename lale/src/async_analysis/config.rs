//! Actor configuration loading from TOML files
//!
//! Supports loading actor timing constraints and platform configurations.

use crate::async_analysis::actor::{Actor, ActorConfig};
use crate::config::loader::ConfigManager;
use crate::platform::PlatformModel;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Veecle OS Model.toml structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VeecleModel {
    pub metadata: VeecleMetadata,

    #[serde(default)]
    pub services: std::collections::HashMap<String, VeecleService>,
}

/// Veecle metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VeecleMetadata {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
}

/// Veecle service definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VeecleService {
    #[serde(default)]
    pub implements: Vec<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub actors: std::collections::HashMap<String, VeecleActor>,
}

/// Veecle actor definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VeecleActor {
    pub path: String,
}

/// Actor system configuration file format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSystemConfig {
    /// System metadata
    pub system: SystemMetadata,

    /// Platform configuration
    pub platform: PlatformConfig,

    /// Actor definitions
    pub actors: Vec<ActorConfigEntry>,
}

/// System metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetadata {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
}

/// Platform configuration reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    /// Platform name (references config/platforms/*.toml)
    pub name: String,

    /// Number of cores
    #[serde(default = "default_num_cores")]
    pub num_cores: usize,

    /// Scheduling policy
    #[serde(default)]
    pub scheduling_policy: SchedulingPolicy,
}

fn default_num_cores() -> usize {
    1
}

/// Scheduling policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum SchedulingPolicy {
    RMA,
    EDF,
}

impl Default for SchedulingPolicy {
    fn default() -> Self {
        SchedulingPolicy::RMA
    }
}

/// Actor configuration entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorConfigEntry {
    pub name: String,
    pub function: String,
    pub priority: u8,
    pub deadline_ms: f64,
    pub period_ms: Option<f64>,
    pub core_affinity: Option<usize>,
}

impl ActorConfigEntry {
    /// Convert to Actor (without WCET data)
    pub fn to_actor(&self) -> Actor {
        Actor::new(
            self.name.clone(),
            self.function.clone(),
            self.priority,
            self.deadline_ms * 1000.0, // ms to us
            self.period_ms.map(|p| p * 1000.0),
            self.core_affinity,
        )
    }
}

/// Actor configuration loader
pub struct ActorConfigLoader {
    config_manager: ConfigManager,
}

impl ActorConfigLoader {
    /// Create new loader with config directory
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            config_manager: ConfigManager::new(config_dir.into()),
        }
    }

    /// Load actor system configuration from file
    pub fn load_system_config(&self, path: impl AsRef<Path>) -> Result<ActorSystemConfig, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;

        toml::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))
    }

    /// Load platform model from configuration
    pub fn load_platform_model(&mut self, platform_name: &str) -> Result<PlatformModel, String> {
        // Load platform configuration
        let platform_config = self.config_manager.load_platform(platform_name)?;

        // Extract CPU frequency
        let cpu_freq_mhz = platform_config
            .soc
            .as_ref()
            .map(|soc| soc.cpu_frequency_mhz)
            .ok_or_else(|| format!("Platform {} has no SoC configuration", platform_name))?;

        // Convert to PlatformModel
        let platform_model = PlatformModel {
            name: platform_name.to_string(),
            cpu_frequency_mhz: cpu_freq_mhz,
            instruction_timings: Self::build_instruction_timings(&platform_config),
        };

        Ok(platform_model)
    }

    /// Build instruction timings from platform config
    fn build_instruction_timings(
        config: &crate::config::types::PlatformConfiguration,
    ) -> ahash::AHashMap<crate::analysis::timing::InstructionClass, crate::analysis::Cycles> {
        use crate::analysis::{timing::InstructionClass, Cycles};
        use ahash::AHashMap;

        let mut timings = AHashMap::new();
        let isa_timings = &config.isa.instruction_timings;

        // Map ISA timings to instruction classes
        timings.insert(
            InstructionClass::Add,
            Cycles {
                best_case: isa_timings.alu,
                worst_case: isa_timings.alu,
            },
        );
        timings.insert(
            InstructionClass::Sub,
            Cycles {
                best_case: isa_timings.alu,
                worst_case: isa_timings.alu,
            },
        );
        timings.insert(
            InstructionClass::Mul,
            Cycles {
                best_case: isa_timings.multiply,
                worst_case: isa_timings.multiply,
            },
        );
        timings.insert(
            InstructionClass::Div,
            Cycles {
                best_case: isa_timings.divide,
                worst_case: isa_timings.divide,
            },
        );
        timings.insert(
            InstructionClass::Load(crate::analysis::timing::AccessType::Ram),
            Cycles {
                best_case: isa_timings.load,
                worst_case: isa_timings.load * 5,
            },
        );
        timings.insert(
            InstructionClass::Store(crate::analysis::timing::AccessType::Ram),
            Cycles {
                best_case: isa_timings.store,
                worst_case: isa_timings.store * 5,
            },
        );
        timings.insert(
            InstructionClass::Branch,
            Cycles {
                best_case: isa_timings.branch,
                worst_case: isa_timings.branch * 3,
            },
        );
        timings.insert(
            InstructionClass::Call,
            Cycles {
                best_case: isa_timings.branch,
                worst_case: isa_timings.branch * 3,
            },
        );

        timings
    }

    /// Load Veecle OS Model.toml
    pub fn load_veecle_model(&self, path: impl AsRef<Path>) -> Result<VeecleModel, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read Model.toml: {}", e))?;

        toml::from_str(&content).map_err(|e| format!("Failed to parse Model.toml: {}", e))
    }

    /// Extract actor paths from Veecle model
    pub fn extract_actor_paths(&self, model: &VeecleModel) -> Vec<(String, String)> {
        let mut actors = Vec::new();

        for (service_name, service) in &model.services {
            for (actor_name, actor) in &service.actors {
                let full_name = format!("{}::{}", service_name, actor_name);
                actors.push((full_name, actor.path.clone()));
            }
        }

        actors
    }

    /// Load Veecle OS project for analysis
    ///
    /// # Arguments
    /// * `project_dir` - Path to Veecle OS project directory containing Model.toml
    /// * `platform_name` - LALE platform configuration name (e.g., "platforms/stm32f746-discovery")
    ///
    /// # Returns
    /// Tuple of (actor_paths, platform_model) where actor_paths contains (name, function_path) pairs
    pub fn load_veecle_project(
        &mut self,
        project_dir: impl AsRef<Path>,
        platform_name: &str,
    ) -> Result<(Vec<(String, String)>, PlatformModel), String> {
        // Load Model.toml
        let model_path = project_dir.as_ref().join("Model.toml");
        let model = self.load_veecle_model(&model_path)?;

        // Extract actor paths
        let actor_paths = self.extract_actor_paths(&model);

        // Load platform model
        let platform_model = self.load_platform_model(platform_name)?;

        Ok((actor_paths, platform_model))
    }

    /// Load complete actor system with platform
    pub fn load_complete_system(
        &mut self,
        config_path: impl AsRef<Path>,
    ) -> Result<(ActorSystemConfig, PlatformModel), String> {
        let system_config = self.load_system_config(config_path)?;
        let platform_model = self.load_platform_model(&system_config.platform.name)?;

        Ok((system_config, platform_model))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_actor_config_entry() {
        let entry = ActorConfigEntry {
            name: "sensor".to_string(),
            function: "sensor_task".to_string(),
            priority: 10,
            deadline_ms: 100.0,
            period_ms: Some(50.0),
            core_affinity: Some(0),
        };

        let actor = entry.to_actor();
        assert_eq!(actor.name, "sensor");
        assert_eq!(actor.deadline_us, 100000.0);
        assert_eq!(actor.period_us, Some(50000.0));
    }
}
