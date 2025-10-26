use super::types::{CacheSet, MemoryBlock};
use crate::ir::CFG;
use ahash::AHashMap;
use petgraph::graph::NodeIndex;
use petgraph::Direction;

/// Must cache analysis - tracks blocks guaranteed to be in cache
/// Conservative: only includes blocks that are definitely cached
#[derive(Debug, Clone)]
pub struct MustAnalysis {
    /// Cache configuration
    cache_size: usize,
    line_size: usize,
    associativity: usize,
}

impl MustAnalysis {
    pub fn new(cache_size: usize, line_size: usize, associativity: usize) -> Self {
        Self {
            cache_size,
            line_size,
            associativity,
        }
    }

    /// Perform must analysis on CFG
    /// Returns must cache state at each program point
    pub fn analyze(&self, cfg: &CFG) -> MustCacheState {
        let mut state = MustCacheState::new();
        let mut worklist = vec![cfg.entry];
        let mut visited = AHashMap::new();

        // Initialize entry with empty cache
        visited.insert(cfg.entry, CacheAbstractState::empty());

        while let Some(node) = worklist.pop() {
            let current_state = visited.get(&node).unwrap().clone();

            // Get block address (simplified - would need actual address)
            let block_addr = self.get_block_address(cfg, node);

            // Update cache state with this block access
            let mut new_state = current_state.clone();
            new_state.access(block_addr, self.associativity);

            // Propagate to successors
            for successor in cfg.graph.neighbors_directed(node, Direction::Outgoing) {
                let successor_state = visited.get(&successor);

                match successor_state {
                    None => {
                        // First time visiting successor
                        visited.insert(successor, new_state.clone());
                        worklist.push(successor);
                    }
                    Some(old_state) => {
                        // Join with existing state (conservative)
                        let joined = old_state.must_join(&new_state);

                        if joined != *old_state {
                            visited.insert(successor, joined);
                            worklist.push(successor);
                        }
                    }
                }
            }
        }

        state.states = visited;
        state
    }

    fn get_block_address(&self, _cfg: &CFG, _node: NodeIndex) -> u64 {
        // Simplified: would extract from CFG node
        0x1000
    }
}

/// Must cache state at program points
#[derive(Debug, Clone)]
pub struct MustCacheState {
    /// Cache state at each CFG node
    pub states: AHashMap<NodeIndex, CacheAbstractState>,
}

impl MustCacheState {
    pub fn new() -> Self {
        Self {
            states: AHashMap::new(),
        }
    }

    /// Get must cache state at a program point
    pub fn get_state(&self, node: NodeIndex) -> Option<&CacheAbstractState> {
        self.states.get(&node)
    }
}

impl Default for MustCacheState {
    fn default() -> Self {
        Self::new()
    }
}

/// Abstract cache state for must analysis
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheAbstractState {
    /// Set of memory blocks definitely in cache
    /// Represented as (set_index, age) pairs
    sets: Vec<CacheSet>,
}

impl CacheAbstractState {
    pub fn empty() -> Self {
        Self { sets: Vec::new() }
    }

    /// Access a memory block
    pub fn access(&mut self, addr: u64, associativity: usize) {
        let block = MemoryBlock::new(addr);
        let set_index = block.set_index(32, 4); // Simplified

        // Ensure set exists
        while self.sets.len() <= set_index {
            self.sets.push(CacheSet::new(associativity));
        }

        // Access the set
        self.sets[set_index].access(block);
    }

    /// Conservative join for must analysis
    /// Only keeps blocks present in BOTH states
    pub fn must_join(&self, other: &Self) -> Self {
        let mut result = Self::empty();

        let max_sets = self.sets.len().max(other.sets.len());
        result.sets.resize(max_sets, CacheSet::new(4));

        for i in 0..max_sets {
            let set1 = self.sets.get(i);
            let set2 = other.sets.get(i);

            match (set1, set2) {
                (Some(s1), Some(s2)) => {
                    result.sets[i] = s1.must_join(s2);
                }
                _ => {
                    // If set missing in either state, result is empty
                    result.sets[i] = CacheSet::new(4);
                }
            }
        }

        result
    }

    /// Check if a block is definitely in cache
    pub fn contains(&self, addr: u64) -> bool {
        let block = MemoryBlock::new(addr);
        let set_index = block.set_index(32, 4);

        if let Some(set) = self.sets.get(set_index) {
            set.contains(&block)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_must_analysis_creation() {
        let analysis = MustAnalysis::new(16384, 32, 4);
        assert_eq!(analysis.cache_size, 16384);
        assert_eq!(analysis.associativity, 4);
    }

    #[test]
    fn test_cache_abstract_state() {
        let mut state = CacheAbstractState::empty();
        state.access(0x1000, 4);

        assert!(state.contains(0x1000));
        assert!(!state.contains(0x2000));
    }

    #[test]
    fn test_must_join() {
        let mut state1 = CacheAbstractState::empty();
        state1.access(0x1000, 4);
        state1.access(0x2000, 4);

        let mut state2 = CacheAbstractState::empty();
        state2.access(0x1000, 4);
        state2.access(0x3000, 4);

        let joined = state1.must_join(&state2);

        // Only 0x1000 is in both states
        assert!(joined.contains(0x1000));
        assert!(!joined.contains(0x2000));
        assert!(!joined.contains(0x3000));
    }
}
