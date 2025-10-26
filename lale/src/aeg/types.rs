use crate::microarch::state::{MicroArchState, StateKey};
use ahash::AHashMap;
use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};

/// Abstract Execution Graph
/// Nodes are microarchitectural states, edges are processor cycles
#[derive(Debug, Clone)]
pub struct AEG {
    /// Graph where nodes are states and edges are cycle transitions
    pub graph: DiGraph<AEGNode, AEGEdge>,

    /// Initial state (entry point)
    pub initial_state: NodeIndex,

    /// Final states (exit points)
    pub final_states: Vec<NodeIndex>,

    /// Mapping from state key to node index
    pub state_map: AHashMap<StateKey, NodeIndex>,
}

impl AEG {
    /// Create new empty AEG
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            initial_state: NodeIndex::new(0),
            final_states: Vec::new(),
            state_map: AHashMap::new(),
        }
    }

    /// Add a state node to the graph
    pub fn add_state(&mut self, state: MicroArchState) -> NodeIndex {
        let key = state.key();

        // Check if state already exists
        if let Some(&node_idx) = self.state_map.get(&key) {
            return node_idx;
        }

        // Add new node
        let node = AEGNode {
            state,
            is_final: false,
        };

        let node_idx = self.graph.add_node(node);
        self.state_map.insert(key, node_idx);

        node_idx
    }

    /// Add an edge between two states
    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, edge: AEGEdge) -> EdgeIndex {
        self.graph.add_edge(from, to, edge)
    }

    /// Mark a state as final
    pub fn mark_final(&mut self, node: NodeIndex) {
        if let Some(node_data) = self.graph.node_weight_mut(node) {
            node_data.is_final = true;
            if !self.final_states.contains(&node) {
                self.final_states.push(node);
            }
        }
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

impl Default for AEG {
    fn default() -> Self {
        Self::new()
    }
}

/// Node in AEG (microarchitectural state)
#[derive(Debug, Clone)]
pub struct AEGNode {
    /// The microarchitectural state
    pub state: MicroArchState,

    /// Whether this is a final state
    pub is_final: bool,
}

/// Edge in AEG (processor cycle transition)
#[derive(Debug, Clone)]
pub struct AEGEdge {
    /// Number of cycles this edge represents
    pub cycles: u32,

    /// Additional metrics collected during simulation
    pub metrics: EdgeMetrics,
}

impl AEGEdge {
    /// Create new edge with cycle count
    pub fn new(cycles: u32) -> Self {
        Self {
            cycles,
            metrics: EdgeMetrics::default(),
        }
    }

    /// Create edge with metrics
    pub fn with_metrics(cycles: u32, metrics: EdgeMetrics) -> Self {
        Self { cycles, metrics }
    }
}

/// Metrics collected during edge traversal
#[derive(Debug, Clone, Default)]
pub struct EdgeMetrics {
    /// Number of cache misses
    pub cache_misses: u32,

    /// Number of memory accesses
    pub memory_accesses: u32,

    /// Number of pipeline stalls
    pub pipeline_stalls: u32,

    /// Number of branch mispredictions
    pub branch_mispredictions: u32,
}

impl EdgeMetrics {
    /// Create new empty metrics
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge two metrics (take maximum)
    pub fn merge(&self, other: &Self) -> Self {
        Self {
            cache_misses: self.cache_misses.max(other.cache_misses),
            memory_accesses: self.memory_accesses.max(other.memory_accesses),
            pipeline_stalls: self.pipeline_stalls.max(other.pipeline_stalls),
            branch_mispredictions: self.branch_mispredictions.max(other.branch_mispredictions),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::microarch::state::PlatformConfig;

    fn test_config() -> PlatformConfig {
        use crate::microarch::state::{
            CacheConfig, CacheLevelConfig, MemoryConfig, MemoryLatency, ReplacementPolicy,
        };

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
    fn test_aeg_creation() {
        let aeg = AEG::new();
        assert_eq!(aeg.node_count(), 0);
        assert_eq!(aeg.edge_count(), 0);
    }

    #[test]
    fn test_add_state() {
        let mut aeg = AEG::new();
        let config = test_config();
        let state = MicroArchState::initial(&config);

        let node = aeg.add_state(state);
        assert_eq!(aeg.node_count(), 1);

        // Adding same state again should return same node
        let config2 = test_config();
        let state2 = MicroArchState::initial(&config2);
        let node2 = aeg.add_state(state2);
        assert_eq!(node, node2);
        assert_eq!(aeg.node_count(), 1);
    }

    #[test]
    fn test_add_edge() {
        let mut aeg = AEG::new();
        let config = test_config();

        let state1 = MicroArchState::initial(&config);
        let mut state2 = state1.clone();
        state2.tick();

        let node1 = aeg.add_state(state1);
        let node2 = aeg.add_state(state2);

        let edge = AEGEdge::new(1);
        aeg.add_edge(node1, node2, edge);

        assert_eq!(aeg.edge_count(), 1);
    }

    #[test]
    fn test_mark_final() {
        let mut aeg = AEG::new();
        let config = test_config();
        let state = MicroArchState::initial(&config);

        let node = aeg.add_state(state);
        aeg.mark_final(node);

        assert_eq!(aeg.final_states.len(), 1);
        assert!(aeg.graph[node].is_final);
    }

    #[test]
    fn test_edge_metrics() {
        let metrics1 = EdgeMetrics {
            cache_misses: 5,
            memory_accesses: 10,
            pipeline_stalls: 2,
            branch_mispredictions: 1,
        };

        let metrics2 = EdgeMetrics {
            cache_misses: 3,
            memory_accesses: 15,
            pipeline_stalls: 4,
            branch_mispredictions: 0,
        };

        let merged = metrics1.merge(&metrics2);

        assert_eq!(merged.cache_misses, 5);
        assert_eq!(merged.memory_accesses, 15);
        assert_eq!(merged.pipeline_stalls, 4);
        assert_eq!(merged.branch_mispredictions, 1);
    }
}
