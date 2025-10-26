use super::{CacheState, MemorySystemState, PipelineState};
use ahash::AHashMap;

/// Address in program
pub type Address = u64;

/// Complete microarchitectural state at a point in execution
#[derive(Debug, Clone)]
pub struct MicroArchState {
    /// Current program counter
    pub program_counter: Address,

    /// Pipeline state (instruction in each stage)
    pub pipeline: PipelineState,

    /// Abstract cache state
    pub cache: CacheState,

    /// Load/store buffer and memory system state
    pub memory_system: MemorySystemState,

    /// Cycle count since entering current basic block
    pub local_cycles: u32,

    /// Unique identifier for this state
    state_id: usize,
}

impl MicroArchState {
    /// Create initial microarchitectural state
    pub fn initial(platform_config: &PlatformConfig) -> Self {
        Self {
            program_counter: 0,
            pipeline: PipelineState::new(platform_config.pipeline_depth),
            cache: CacheState::new(&platform_config.cache_config),
            memory_system: MemorySystemState::new(&platform_config.memory_config),
            local_cycles: 0,
            state_id: 0,
        }
    }

    /// Create a unique key for this state (for hashing/comparison)
    pub fn key(&self) -> StateKey {
        StateKey {
            pc: self.program_counter,
            pipeline_hash: self.pipeline.hash(),
            cache_hash: self.cache.hash(),
        }
    }

    /// Check if two states can be joined (merged)
    pub fn is_joinable(&self, other: &Self) -> bool {
        // States must be at same program point
        if self.program_counter != other.program_counter {
            return false;
        }

        // Pipeline states must be compatible
        if !self.pipeline.is_compatible(&other.pipeline) {
            return false;
        }

        // Cache states must be joinable
        if !self.cache.is_joinable(&other.cache) {
            return false;
        }

        true
    }

    /// Join (merge) two compatible states
    pub fn join(&self, other: &Self) -> Self {
        assert!(self.is_joinable(other), "Cannot join incompatible states");

        Self {
            program_counter: self.program_counter,
            pipeline: self.pipeline.join(&other.pipeline),
            cache: self.cache.join(&other.cache),
            memory_system: self.memory_system.join(&other.memory_system),
            local_cycles: self.local_cycles.max(other.local_cycles),
            state_id: self.state_id, // Keep first state's ID
        }
    }

    /// Advance program counter
    pub fn advance_pc(&mut self, offset: i64) {
        self.program_counter = (self.program_counter as i64 + offset) as u64;
    }

    /// Increment cycle counter
    pub fn tick(&mut self) {
        self.local_cycles += 1;
    }

    /// Reset local cycle counter (when entering new basic block)
    pub fn reset_local_cycles(&mut self) {
        self.local_cycles = 0;
    }
}

/// Key for identifying unique states
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StateKey {
    pub pc: Address,
    pub pipeline_hash: u64,
    pub cache_hash: u64,
}

/// Platform configuration for state initialization
#[derive(Debug, Clone)]
pub struct PlatformConfig {
    pub pipeline_depth: usize,
    pub cache_config: CacheConfig,
    pub memory_config: MemoryConfig,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub instruction_cache: Option<CacheLevelConfig>,
    pub data_cache: Option<CacheLevelConfig>,
}

/// Single cache level configuration
#[derive(Debug, Clone)]
pub struct CacheLevelConfig {
    pub size_kb: usize,
    pub line_size_bytes: usize,
    pub associativity: usize,
    pub replacement_policy: ReplacementPolicy,
}

/// Cache replacement policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacementPolicy {
    LRU,
    PLRU,
    FIFO,
}

/// Memory system configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    pub load_buffer_size: usize,
    pub store_buffer_size: usize,
    pub memory_latency: MemoryLatency,
}

/// Memory access latency
#[derive(Debug, Clone)]
pub enum MemoryLatency {
    Fixed { cycles: u32 },
    Variable { min: u32, max: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> PlatformConfig {
        PlatformConfig {
            pipeline_depth: 5,
            cache_config: CacheConfig {
                instruction_cache: Some(CacheLevelConfig {
                    size_kb: 16,
                    line_size_bytes: 32,
                    associativity: 4,
                    replacement_policy: ReplacementPolicy::LRU,
                }),
                data_cache: Some(CacheLevelConfig {
                    size_kb: 16,
                    line_size_bytes: 32,
                    associativity: 4,
                    replacement_policy: ReplacementPolicy::LRU,
                }),
            },
            memory_config: MemoryConfig {
                load_buffer_size: 4,
                store_buffer_size: 4,
                memory_latency: MemoryLatency::Fixed { cycles: 10 },
            },
        }
    }

    #[test]
    fn test_state_creation() {
        let config = test_config();
        let state = MicroArchState::initial(&config);

        assert_eq!(state.program_counter, 0);
        assert_eq!(state.local_cycles, 0);
    }

    #[test]
    fn test_state_key() {
        let config = test_config();
        let state1 = MicroArchState::initial(&config);
        let state2 = MicroArchState::initial(&config);

        // Same initial states should have same key
        assert_eq!(state1.key(), state2.key());
    }

    #[test]
    fn test_pc_advance() {
        let config = test_config();
        let mut state = MicroArchState::initial(&config);

        state.advance_pc(4);
        assert_eq!(state.program_counter, 4);

        state.advance_pc(-2);
        assert_eq!(state.program_counter, 2);
    }

    #[test]
    fn test_cycle_tick() {
        let config = test_config();
        let mut state = MicroArchState::initial(&config);

        assert_eq!(state.local_cycles, 0);
        state.tick();
        assert_eq!(state.local_cycles, 1);
        state.tick();
        assert_eq!(state.local_cycles, 2);
    }
}
