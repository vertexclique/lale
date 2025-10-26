use super::may::MayCacheState;
use super::must::MustCacheState;
use super::types::{CacheSet, MemoryBlock};
use crate::ir::CFG;
use ahash::{AHashMap, AHashSet};
use petgraph::graph::NodeIndex;

/// Persistence analysis - tracks blocks that survive within a scope
/// Useful for loop analysis to reduce pessimism
#[derive(Debug, Clone)]
pub struct PersistenceAnalysis {
    /// Cache configuration
    cache_size: usize,
    line_size: usize,
    associativity: usize,
}

impl PersistenceAnalysis {
    pub fn new(cache_size: usize, line_size: usize, associativity: usize) -> Self {
        Self {
            cache_size,
            line_size,
            associativity,
        }
    }

    /// Analyze persistence within a scope (e.g., loop body)
    /// Returns blocks that are persistent (survive) within the scope
    pub fn analyze_scope(
        &self,
        cfg: &CFG,
        scope_nodes: &[NodeIndex],
        must_state: &MustCacheState,
        may_state: &MayCacheState,
    ) -> PersistentBlocks {
        let mut persistent = PersistentBlocks::new();

        // A block is persistent if:
        // 1. It's in the must set at scope entry
        // 2. It's not evicted by any path through the scope

        // Get entry node of scope
        if let Some(&entry) = scope_nodes.first() {
            if let Some(entry_must) = must_state.get_state(entry) {
                // Check each block in must set
                for addr in self.get_cached_blocks(entry_must) {
                    if self.is_persistent_in_scope(addr, scope_nodes, must_state, may_state) {
                        persistent.blocks.insert(addr);
                    }
                }
            }
        }

        persistent
    }

    /// Check if a block is persistent within a scope
    fn is_persistent_in_scope(
        &self,
        addr: u64,
        scope_nodes: &[NodeIndex],
        must_state: &MustCacheState,
        _may_state: &MayCacheState,
    ) -> bool {
        // Block is persistent if it's in must set at all scope nodes
        for &node in scope_nodes {
            if let Some(state) = must_state.get_state(node) {
                if !state.contains(addr) {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// Extract cached block addresses from must state
    fn get_cached_blocks(&self, _state: &super::must::CacheAbstractState) -> Vec<u64> {
        // Simplified: would extract actual addresses
        vec![0x1000, 0x2000]
    }

    /// Analyze persistence for loop
    pub fn analyze_loop(
        &self,
        cfg: &CFG,
        loop_header: NodeIndex,
        loop_body: &[NodeIndex],
        must_state: &MustCacheState,
        may_state: &MayCacheState,
    ) -> LoopPersistence {
        let persistent = self.analyze_scope(cfg, loop_body, must_state, may_state);

        LoopPersistence {
            header: loop_header,
            persistent_blocks: persistent,
        }
    }
}

/// Set of persistent blocks
#[derive(Debug, Clone)]
pub struct PersistentBlocks {
    /// Addresses of persistent blocks
    pub blocks: AHashSet<u64>,
}

impl PersistentBlocks {
    pub fn new() -> Self {
        Self {
            blocks: AHashSet::new(),
        }
    }

    /// Check if a block is persistent
    pub fn is_persistent(&self, addr: u64) -> bool {
        self.blocks.contains(&addr)
    }

    /// Get number of persistent blocks
    pub fn count(&self) -> usize {
        self.blocks.len()
    }
}

impl Default for PersistentBlocks {
    fn default() -> Self {
        Self::new()
    }
}

/// Persistence information for a loop
#[derive(Debug, Clone)]
pub struct LoopPersistence {
    /// Loop header node
    pub header: NodeIndex,

    /// Blocks persistent in this loop
    pub persistent_blocks: PersistentBlocks,
}

impl LoopPersistence {
    /// Check if accessing this block will be a hit in loop iterations
    pub fn is_loop_hit(&self, addr: u64) -> bool {
        self.persistent_blocks.is_persistent(addr)
    }
}

/// Combined cache analysis result
#[derive(Debug, Clone)]
pub struct CacheAnalysisResult {
    /// Must analysis result
    pub must: MustCacheState,

    /// May analysis result  
    pub may: MayCacheState,

    /// Persistence analysis for loops
    pub persistence: AHashMap<NodeIndex, LoopPersistence>,
}

impl CacheAnalysisResult {
    pub fn new(must: MustCacheState, may: MayCacheState) -> Self {
        Self {
            must,
            may,
            persistence: AHashMap::new(),
        }
    }

    /// Classify cache access at a program point
    pub fn classify_access(&self, node: NodeIndex, addr: u64) -> CacheAccessClass {
        let in_must = self
            .must
            .get_state(node)
            .map(|s| s.contains(addr))
            .unwrap_or(false);

        let in_may = self
            .may
            .get_state(node)
            .map(|s| s.may_contain(addr))
            .unwrap_or(false);

        match (in_must, in_may) {
            (true, _) => CacheAccessClass::AlwaysHit,
            (false, false) => CacheAccessClass::AlwaysMiss,
            (false, true) => CacheAccessClass::Unknown,
        }
    }
}

/// Cache access classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheAccessClass {
    /// Definitely a cache hit
    AlwaysHit,

    /// Definitely a cache miss
    AlwaysMiss,

    /// Unknown (may hit or miss)
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistence_analysis_creation() {
        let analysis = PersistenceAnalysis::new(16384, 32, 4);
        assert_eq!(analysis.cache_size, 16384);
        assert_eq!(analysis.associativity, 4);
    }

    #[test]
    fn test_persistent_blocks() {
        let mut blocks = PersistentBlocks::new();
        blocks.blocks.insert(0x1000);
        blocks.blocks.insert(0x2000);

        assert!(blocks.is_persistent(0x1000));
        assert!(blocks.is_persistent(0x2000));
        assert!(!blocks.is_persistent(0x3000));
        assert_eq!(blocks.count(), 2);
    }

    #[test]
    fn test_cache_access_classification() {
        let must = MustCacheState::new();
        let may = MayCacheState::new();
        let result = CacheAnalysisResult::new(must, may);

        // With empty states, should be AlwaysMiss
        let node = NodeIndex::new(0);
        let class = result.classify_access(node, 0x1000);
        assert_eq!(class, CacheAccessClass::AlwaysMiss);
    }
}
