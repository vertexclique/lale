use super::types::{MemoryBlock, CacheSet};
use crate::ir::CFG;
use ahash::AHashMap;
use petgraph::graph::NodeIndex;
use petgraph::Direction;

/// May cache analysis - tracks blocks possibly in cache
/// Optimistic: includes all blocks that might be cached
#[derive(Debug, Clone)]
pub struct MayAnalysis {
    /// Cache configuration
    cache_size: usize,
    line_size: usize,
    associativity: usize,
}

impl MayAnalysis {
    pub fn new(cache_size: usize, line_size: usize, associativity: usize) -> Self {
        Self {
            cache_size,
            line_size,
            associativity,
        }
    }
    
    /// Perform may analysis on CFG
    /// Returns may cache state at each program point
    pub fn analyze(&self, cfg: &CFG) -> MayCacheState {
        let mut state = MayCacheState::new();
        let mut worklist = vec![cfg.entry];
        let mut visited = AHashMap::new();
        
        // Initialize entry with empty cache
        visited.insert(cfg.entry, CacheAbstractState::empty());
        
        while let Some(node) = worklist.pop() {
            let current_state = visited.get(&node).unwrap().clone();
            
            // Get block address
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
                        // Join with existing state (optimistic)
                        let joined = old_state.may_join(&new_state);
                        
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

/// May cache state at program points
#[derive(Debug, Clone)]
pub struct MayCacheState {
    /// Cache state at each CFG node
    pub states: AHashMap<NodeIndex, CacheAbstractState>,
}

impl MayCacheState {
    pub fn new() -> Self {
        Self {
            states: AHashMap::new(),
        }
    }
    
    /// Get may cache state at a program point
    pub fn get_state(&self, node: NodeIndex) -> Option<&CacheAbstractState> {
        self.states.get(&node)
    }
}

impl Default for MayCacheState {
    fn default() -> Self {
        Self::new()
    }
}

/// Abstract cache state for may analysis
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheAbstractState {
    /// Set of memory blocks possibly in cache
    sets: Vec<CacheSet>,
}

impl CacheAbstractState {
    pub fn empty() -> Self {
        Self {
            sets: Vec::new(),
        }
    }
    
    /// Access a memory block
    pub fn access(&mut self, addr: u64, associativity: usize) {
        let block = MemoryBlock::new(addr);
        let set_index = block.set_index(32, 4);
        
        // Ensure set exists
        while self.sets.len() <= set_index {
            self.sets.push(CacheSet::new(associativity));
        }
        
        // Access the set
        self.sets[set_index].access(block);
    }
    
    /// Optimistic join for may analysis
    /// Keeps blocks present in EITHER state (union)
    pub fn may_join(&self, other: &Self) -> Self {
        let mut result = Self::empty();
        
        let max_sets = self.sets.len().max(other.sets.len());
        result.sets.resize(max_sets, CacheSet::new(4));
        
        for i in 0..max_sets {
            let set1 = self.sets.get(i);
            let set2 = other.sets.get(i);
            
            match (set1, set2) {
                (Some(s1), Some(s2)) => {
                    result.sets[i] = s1.may_join(s2);
                }
                (Some(s1), None) => {
                    result.sets[i] = s1.clone();
                }
                (None, Some(s2)) => {
                    result.sets[i] = s2.clone();
                }
                (None, None) => {
                    result.sets[i] = CacheSet::new(4);
                }
            }
        }
        
        result
    }
    
    /// Check if a block might be in cache
    pub fn may_contain(&self, addr: u64) -> bool {
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
    fn test_may_analysis_creation() {
        let analysis = MayAnalysis::new(16384, 32, 4);
        assert_eq!(analysis.cache_size, 16384);
        assert_eq!(analysis.associativity, 4);
    }
    
    #[test]
    fn test_cache_abstract_state() {
        let mut state = CacheAbstractState::empty();
        state.access(0x1000, 4);
        
        assert!(state.may_contain(0x1000));
        assert!(!state.may_contain(0x2000));
    }
    
    #[test]
    fn test_may_join() {
        let mut state1 = CacheAbstractState::empty();
        state1.access(0x1000, 4);
        state1.access(0x2000, 4);
        
        let mut state2 = CacheAbstractState::empty();
        state2.access(0x1000, 4);
        state2.access(0x3000, 4);
        
        let joined = state1.may_join(&state2);
        
        // All blocks from both states
        assert!(joined.may_contain(0x1000));
        assert!(joined.may_contain(0x2000));
        assert!(joined.may_contain(0x3000));
    }
}
