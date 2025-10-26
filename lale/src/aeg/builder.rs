use super::types::{AEGEdge, EdgeMetrics, AEG};
use crate::microarch::simulator::MicroArchSimulator;
use crate::microarch::state::{MicroArchState, StateKey};
use ahash::AHashMap;
use std::collections::VecDeque;

/// AEG Builder - constructs Abstract Execution Graph via state space exploration
pub struct AEGBuilder {
    /// Microarchitectural simulator
    simulator: MicroArchSimulator,

    /// Maximum number of states to explore (prevents explosion)
    max_states: usize,

    /// Enable state joining to bound state space
    enable_joining: bool,
}

impl AEGBuilder {
    /// Create new AEG builder
    pub fn new(simulator: MicroArchSimulator) -> Self {
        Self {
            simulator,
            max_states: 100_000,
            enable_joining: true,
        }
    }

    /// Set maximum states limit
    pub fn with_max_states(mut self, max: usize) -> Self {
        self.max_states = max;
        self
    }

    /// Enable or disable state joining
    pub fn with_joining(mut self, enable: bool) -> Self {
        self.enable_joining = enable;
        self
    }

    /// Build AEG from initial state
    /// Uses worklist algorithm with state joining
    pub fn build(&self, initial_state: MicroArchState, max_cycles: u32) -> Result<AEG, String> {
        let mut aeg = AEG::new();
        let mut worklist = VecDeque::new();
        let mut visited = AHashMap::new();

        // Add initial state
        let initial_node = aeg.add_state(initial_state.clone());
        aeg.initial_state = initial_node;
        worklist.push_back((initial_state, initial_node, 0u32)); // (state, node, cycles)

        while let Some((current_state, current_node, cycles)) = worklist.pop_front() {
            // Check limits
            if aeg.node_count() >= self.max_states {
                return Err(format!(
                    "State space explosion: exceeded {} states",
                    self.max_states
                ));
            }

            if cycles >= max_cycles {
                // Reached cycle limit - mark as final
                aeg.mark_final(current_node);
                continue;
            }

            // Check if already visited
            let state_key = current_state.key();
            if visited.contains_key(&state_key) {
                continue;
            }
            visited.insert(state_key, current_node);

            // Simulate one cycle - get successor states
            let successors = self.simulator.cycle(&current_state);

            for successor in successors {
                let successor_key = successor.key();

                // Try to join with existing state if enabled
                if self.enable_joining {
                    if let Some(existing_node) = self.find_joinable_state(&aeg, &successor) {
                        // Join with existing state
                        let joined = self
                            .simulator
                            .join(&aeg.graph[existing_node].state, &successor);

                        // Update existing node with joined state
                        // (In practice, we'd need mutable access - simplified here)

                        // Add edge to existing node
                        let edge = AEGEdge::new(1);
                        aeg.add_edge(current_node, existing_node, edge);
                        continue;
                    }
                }

                // Add new state
                let successor_node = aeg.add_state(successor.clone());

                // Add edge
                let edge = AEGEdge::new(1);
                aeg.add_edge(current_node, successor_node, edge);

                // Add to worklist if not visited
                if !visited.contains_key(&successor_key) {
                    worklist.push_back((successor, successor_node, cycles + 1));
                }
            }
        }

        Ok(aeg)
    }

    /// Find a joinable state in the AEG
    fn find_joinable_state(
        &self,
        aeg: &AEG,
        state: &MicroArchState,
    ) -> Option<petgraph::graph::NodeIndex> {
        // Look for states at same program counter
        for (key, &node_idx) in &aeg.state_map {
            if key.pc == state.program_counter {
                let existing_state = &aeg.graph[node_idx].state;
                if self.simulator.is_joinable(existing_state, state) {
                    return Some(node_idx);
                }
            }
        }
        None
    }

    /// Build AEG for a basic block
    /// Simpler version that explores until pipeline is empty
    pub fn build_for_block(
        &self,
        initial_state: MicroArchState,
        target_pc: u64,
    ) -> Result<AEG, String> {
        let mut aeg = AEG::new();
        let mut worklist = VecDeque::new();
        let mut visited = AHashMap::new();

        // Add initial state
        let initial_node = aeg.add_state(initial_state.clone());
        aeg.initial_state = initial_node;
        worklist.push_back((initial_state, initial_node));

        while let Some((current_state, current_node)) = worklist.pop_front() {
            // Check limits
            if aeg.node_count() >= self.max_states {
                return Err(format!(
                    "State space explosion: exceeded {} states",
                    self.max_states
                ));
            }

            // Check if final state
            if self.simulator.is_final(&current_state, target_pc) {
                aeg.mark_final(current_node);
                continue;
            }

            // Check if already visited
            let state_key = current_state.key();
            if visited.contains_key(&state_key) {
                continue;
            }
            visited.insert(state_key, current_node);

            // Simulate one cycle
            let successors = self.simulator.cycle(&current_state);

            for successor in successors {
                let successor_key = successor.key();

                // Add new state
                let successor_node = aeg.add_state(successor.clone());

                // Add edge
                let edge = AEGEdge::new(1);
                aeg.add_edge(current_node, successor_node, edge);

                // Add to worklist if not visited
                if !visited.contains_key(&successor_key) {
                    worklist.push_back((successor, successor_node));
                }
            }
        }

        Ok(aeg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::microarch::state::{
        CacheConfig, CacheLevelConfig, MemoryConfig, MemoryLatency, PlatformConfig,
        ReplacementPolicy,
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
    fn test_builder_creation() {
        let config = test_config();
        let simulator = MicroArchSimulator::new(config);
        let builder = AEGBuilder::new(simulator);

        assert_eq!(builder.max_states, 100_000);
        assert!(builder.enable_joining);
    }

    #[test]
    fn test_builder_configuration() {
        let config = test_config();
        let simulator = MicroArchSimulator::new(config);
        let builder = AEGBuilder::new(simulator)
            .with_max_states(1000)
            .with_joining(false);

        assert_eq!(builder.max_states, 1000);
        assert!(!builder.enable_joining);
    }

    #[test]
    fn test_build_simple() {
        let config = test_config();
        let initial_state = MicroArchState::initial(&config);
        let simulator = MicroArchSimulator::new(config);
        let builder = AEGBuilder::new(simulator);

        // Build for a few cycles
        let result = builder.build(initial_state, 10);

        assert!(result.is_ok());
        let aeg = result.unwrap();

        // Should have explored some states
        assert!(aeg.node_count() > 0);
        assert!(aeg.edge_count() >= 0);
    }

    #[test]
    fn test_build_for_block() {
        let config = test_config();
        let initial_state = MicroArchState::initial(&config);
        let simulator = MicroArchSimulator::new(config);
        let builder = AEGBuilder::new(simulator);

        let result = builder.build_for_block(initial_state, 0x1000);

        assert!(result.is_ok());
        let aeg = result.unwrap();

        assert!(aeg.node_count() > 0);
    }

    #[test]
    fn test_state_limit() {
        let config = test_config();
        let initial_state = MicroArchState::initial(&config);
        let simulator = MicroArchSimulator::new(config);
        let builder = AEGBuilder::new(simulator).with_max_states(5);

        // Try to build with very low limit
        let result = builder.build(initial_state, 100);

        // Should hit limit or succeed with small graph
        match result {
            Ok(aeg) => assert!(aeg.node_count() <= 5),
            Err(msg) => assert!(msg.contains("State space explosion")),
        }
    }
}
