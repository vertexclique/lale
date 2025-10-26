use super::cache::types::AccessClassification;
use super::pipeline::{AbstractAddress, InstructionSlot, MemoryAccessType, Opcode, StageType};
use super::state::{MicroArchState, PlatformConfig};

/// Microarchitectural simulator - advances state cycle-by-cycle
pub struct MicroArchSimulator {
    pub config: PlatformConfig,
}

impl MicroArchSimulator {
    /// Create new simulator with platform configuration
    pub fn new(config: PlatformConfig) -> Self {
        Self { config }
    }

    /// Simulate one processor cycle
    /// Returns successor states (may split on non-determinism)
    pub fn cycle(&self, state: &MicroArchState) -> Vec<MicroArchState> {
        let mut successors = Vec::new();
        let mut current = state.clone();

        // 1. Advance pipeline stages (back to front to avoid conflicts)
        self.advance_pipeline(&mut current);

        // 2. Process memory system
        current.memory_system.tick();

        // 3. Handle cache accesses (may cause state splitting)
        if let Some(split_states) = self.handle_cache_accesses(&current) {
            successors.extend(split_states);
        } else {
            successors.push(current);
        }

        // 4. Increment cycle counter for all successors
        for succ in &mut successors {
            succ.tick();
        }

        successors
    }

    /// Advance pipeline stages by one cycle
    fn advance_pipeline(&self, state: &mut MicroArchState) {
        let num_stages = state.pipeline.stages.len();

        // Process stages from back to front to avoid overwriting
        for i in (0..num_stages).rev() {
            if state.pipeline.stages[i].stalled {
                continue;
            }

            if i == num_stages - 1 {
                // Last stage (WriteBack) - instruction completes
                state.pipeline.stages[i].instruction = None;
            } else {
                // Move instruction to next stage
                let next_stage_stalled = state.pipeline.stages[i + 1].stalled;

                if !next_stage_stalled {
                    let instr = state.pipeline.stages[i].instruction.take();
                    state.pipeline.stages[i + 1].instruction = instr;
                }
            }
        }
    }

    /// Handle cache accesses (may split state on uncertainty)
    fn handle_cache_accesses(&self, state: &MicroArchState) -> Option<Vec<MicroArchState>> {
        // Check if any stage is accessing memory
        for stage in &state.pipeline.stages {
            if let Some(instr) = &stage.instruction {
                if let Some(mem_access) = &instr.memory_access {
                    // Determine address
                    let address = match &mem_access.address {
                        AbstractAddress::Concrete(addr) => *addr,
                        AbstractAddress::Range { min, .. } => *min, // Use min for now
                        AbstractAddress::Unknown => return None,    // Can't analyze
                    };

                    // Classify cache access
                    let classification = match mem_access.access_type {
                        MemoryAccessType::Load => state
                            .cache
                            .d_cache
                            .as_ref()
                            .map(|c| c.classify(address))
                            .unwrap_or(AccessClassification::AlwaysHit),
                        MemoryAccessType::Store => state
                            .cache
                            .d_cache
                            .as_ref()
                            .map(|c| c.classify(address))
                            .unwrap_or(AccessClassification::AlwaysHit),
                    };

                    // Split state if uncertain
                    match classification {
                        AccessClassification::Unknown => {
                            return Some(self.split_on_cache_uncertainty(state, address));
                        }
                        _ => {}
                    }
                }
            }
        }

        None
    }

    /// Split state into hit and miss scenarios
    fn split_on_cache_uncertainty(
        &self,
        state: &MicroArchState,
        address: u64,
    ) -> Vec<MicroArchState> {
        let mut hit_state = state.clone();
        let mut miss_state = state.clone();

        // Hit scenario: access cache (updates state)
        if let Some(cache) = &mut hit_state.cache.d_cache {
            cache.access(address);
        }

        // Miss scenario: access cache (updates state differently)
        if let Some(cache) = &mut miss_state.cache.d_cache {
            cache.access(address);
        }

        vec![hit_state, miss_state]
    }

    /// Check if instruction has completed execution
    pub fn is_final(&self, state: &MicroArchState, target_address: u64) -> bool {
        // Check if target instruction just left WriteBack stage
        // For now, simplified: check if pipeline is empty after target PC
        state.program_counter > target_address && state.pipeline.is_empty()
    }

    /// Check if two states can be joined
    pub fn is_joinable(&self, s1: &MicroArchState, s2: &MicroArchState) -> bool {
        s1.is_joinable(s2)
    }

    /// Join two compatible states
    pub fn join(&self, s1: &MicroArchState, s2: &MicroArchState) -> MicroArchState {
        s1.join(s2)
    }

    /// Insert instruction into pipeline (fetch stage)
    pub fn fetch_instruction(&self, state: &mut MicroArchState, instr: InstructionSlot) {
        if let Some(fetch_stage) = state.pipeline.get_stage_mut(StageType::Fetch) {
            fetch_stage.instruction = Some(instr);
        }
    }
}

/// Precomputed information for analysis
#[derive(Debug, Clone)]
pub struct PrecomputedInfo {
    /// Address information for memory accesses
    pub addresses: std::collections::HashMap<u64, AbstractAddress>,

    /// Loop bounds
    pub loop_bounds: std::collections::HashMap<u64, u32>,
}

impl PrecomputedInfo {
    pub fn new() -> Self {
        Self {
            addresses: std::collections::HashMap::new(),
            loop_bounds: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::microarch::pipeline::{DepType, MemoryAccess, RegisterDep};
    use crate::microarch::state::{
        CacheConfig, CacheLevelConfig, MemoryConfig, MemoryLatency, ReplacementPolicy,
    };

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
    fn test_simulator_creation() {
        let config = test_config();
        let simulator = MicroArchSimulator::new(config);
        assert_eq!(simulator.config.pipeline_depth, 5);
    }

    #[test]
    fn test_cycle_simulation() {
        let config = test_config();
        let state = MicroArchState::initial(&config);
        let simulator = MicroArchSimulator::new(config);

        let successors = simulator.cycle(&state);
        assert!(!successors.is_empty());
    }

    #[test]
    fn test_pipeline_advancement() {
        let config = test_config();
        let mut state = MicroArchState::initial(&config);
        let simulator = MicroArchSimulator::new(config);

        // Insert instruction in fetch stage
        let instr = InstructionSlot {
            address: 0x1000,
            opcode: Opcode::ALU,
            dependencies: vec![],
            memory_access: None,
        };

        simulator.fetch_instruction(&mut state, instr);

        // Verify instruction is in fetch
        assert!(state
            .pipeline
            .get_stage(StageType::Fetch)
            .unwrap()
            .instruction
            .is_some());

        // Advance pipeline
        simulator.advance_pipeline(&mut state);

        // Instruction should have moved to decode
        assert!(state
            .pipeline
            .get_stage(StageType::Fetch)
            .unwrap()
            .instruction
            .is_none());
        assert!(state
            .pipeline
            .get_stage(StageType::Decode)
            .unwrap()
            .instruction
            .is_some());
    }

    #[test]
    fn test_state_joining() {
        let config = test_config();

        let state1 = MicroArchState::initial(&config);
        let state2 = MicroArchState::initial(&config);
        let simulator = MicroArchSimulator::new(config);

        assert!(simulator.is_joinable(&state1, &state2));

        let joined = simulator.join(&state1, &state2);
        assert_eq!(joined.program_counter, state1.program_counter);
    }

    #[test]
    fn test_cache_access_splitting() {
        let config = test_config();
        let mut state = MicroArchState::initial(&config);
        let simulator = MicroArchSimulator::new(config);

        // Insert load instruction
        let instr = InstructionSlot {
            address: 0x1000,
            opcode: Opcode::Load,
            dependencies: vec![],
            memory_access: Some(MemoryAccess {
                address: AbstractAddress::Concrete(0x2000),
                access_type: MemoryAccessType::Load,
                size: 4,
            }),
        };

        // Put instruction in memory stage
        if let Some(mem_stage) = state.pipeline.get_stage_mut(StageType::Memory) {
            mem_stage.instruction = Some(instr);
        }

        // First access should cause miss
        let successors = simulator.cycle(&state);
        assert!(!successors.is_empty());
    }

    #[test]
    fn test_fetch_instruction() {
        let config = test_config();
        let mut state = MicroArchState::initial(&config);
        let simulator = MicroArchSimulator::new(config);

        let instr = InstructionSlot {
            address: 0x1000,
            opcode: Opcode::ALU,
            dependencies: vec![
                RegisterDep {
                    register: 1,
                    dep_type: DepType::Read,
                },
                RegisterDep {
                    register: 2,
                    dep_type: DepType::Write,
                },
            ],
            memory_access: None,
        };

        simulator.fetch_instruction(&mut state, instr.clone());

        let fetch_stage = state.pipeline.get_stage(StageType::Fetch).unwrap();
        assert!(fetch_stage.instruction.is_some());
        assert_eq!(fetch_stage.instruction.as_ref().unwrap().address, 0x1000);
    }
}
