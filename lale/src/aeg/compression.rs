use super::types::{AEG, AEGEdge, EdgeMetrics};
use crate::ir::CFG;
use petgraph::graph::{DiGraph, NodeIndex};
use ahash::AHashMap;

/// Compressed AEG - reduced from cycle-granularity to block-level
#[derive(Debug, Clone)]
pub struct CompressedAEG {
    /// Compressed graph
    pub graph: DiGraph<BlockNode, BlockEdge>,
    
    /// Entry node
    pub entry: NodeIndex,
    
    /// Exit nodes
    pub exits: Vec<NodeIndex>,
    
    /// Mapping from basic block label to node
    pub block_map: AHashMap<String, NodeIndex>,
}

impl CompressedAEG {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            entry: NodeIndex::new(0),
            exits: Vec::new(),
            block_map: AHashMap::new(),
        }
    }
}

impl Default for CompressedAEG {
    fn default() -> Self {
        Self::new()
    }
}

/// Node in compressed AEG (represents basic block)
#[derive(Debug, Clone)]
pub struct BlockNode {
    /// Basic block label
    pub label: String,
    
    /// Number of entry states
    pub entry_states: usize,
    
    /// Number of exit states
    pub exit_states: usize,
}

/// Edge in compressed AEG (represents execution through block)
#[derive(Debug, Clone)]
pub struct BlockEdge {
    /// Minimum cycles through this path
    pub min_cycles: u32,
    
    /// Maximum cycles through this path
    pub max_cycles: u32,
    
    /// Aggregated metrics
    pub metrics: EdgeMetrics,
    
    /// ILP variable index (for IPET)
    pub ilp_var: Option<usize>,
}

impl BlockEdge {
    pub fn new(min: u32, max: u32) -> Self {
        Self {
            min_cycles: min,
            max_cycles: max,
            metrics: EdgeMetrics::default(),
            ilp_var: None,
        }
    }
}

/// Compression mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionMode {
    /// Preserve timing correlations (multiple states per block)
    Precise,
    
    /// Single edge per block (faster but less precise)
    Efficient,
    
    /// Choose based on complexity
    Adaptive,
}

/// Trait for AEG compression strategies
pub trait Compression {
    fn compress(&self, aeg: &AEG, cfg: &CFG) -> CompressedAEG;
}

/// Precise compression - preserves timing correlations
pub struct PreciseCompression;

impl Compression for PreciseCompression {
    fn compress(&self, aeg: &AEG, cfg: &CFG) -> CompressedAEG {
        let mut compressed = CompressedAEG::new();
        
        // Group AEG states by basic block (using PC)
        let mut block_states: AHashMap<String, Vec<NodeIndex>> = AHashMap::new();
        
        for node_idx in aeg.graph.node_indices() {
            let state = &aeg.graph[node_idx].state;
            let pc = state.program_counter;
            
            // Find which basic block this PC belongs to
            if let Some(block_label) = self.find_block_for_pc(cfg, pc) {
                block_states.entry(block_label.clone())
                    .or_insert_with(Vec::new)
                    .push(node_idx);
            }
        }
        
        // Create nodes for each basic block
        for (label, states) in &block_states {
            let node = BlockNode {
                label: label.clone(),
                entry_states: states.len(),
                exit_states: states.len(),
            };
            
            let node_idx = compressed.graph.add_node(node);
            compressed.block_map.insert(label.clone(), node_idx);
        }
        
        // Create edges between blocks
        // For each AEG edge, if it crosses block boundaries, create compressed edge
        for edge_idx in aeg.graph.edge_indices() {
            let (src, dst) = aeg.graph.edge_endpoints(edge_idx).unwrap();
            let edge_data = &aeg.graph[edge_idx];
            
            let src_pc = aeg.graph[src].state.program_counter;
            let dst_pc = aeg.graph[dst].state.program_counter;
            
            if let (Some(src_block), Some(dst_block)) = (
                self.find_block_for_pc(cfg, src_pc),
                self.find_block_for_pc(cfg, dst_pc)
            ) {
                // Only create edge if crossing block boundary
                if src_block != dst_block {
                    if let (Some(&src_node), Some(&dst_node)) = (
                        compressed.block_map.get(&src_block),
                        compressed.block_map.get(&dst_block)
                    ) {
                        // Check if edge already exists
                        let existing = compressed.graph
                            .edges_connecting(src_node, dst_node)
                            .next();
                        
                        if existing.is_none() {
                            let block_edge = BlockEdge::new(
                                edge_data.cycles,
                                edge_data.cycles
                            );
                            compressed.graph.add_edge(src_node, dst_node, block_edge);
                        }
                    }
                }
            }
        }
        
        // Set entry and exits
        if let Some(&entry_node) = compressed.block_map.values().next() {
            compressed.entry = entry_node;
        }
        
        compressed
    }
}

impl PreciseCompression {
    fn find_block_for_pc(&self, cfg: &CFG, pc: u64) -> Option<String> {
        // Simplified: use PC as block identifier
        // In practice, would map PC to actual basic block
        for (label, &_node_idx) in &cfg.label_to_node {
            // This is simplified - would need actual PC ranges
            return Some(label.clone());
        }
        None
    }
}

/// Efficient compression - single edge per block
pub struct EfficientCompression;

impl Compression for EfficientCompression {
    fn compress(&self, aeg: &AEG, cfg: &CFG) -> CompressedAEG {
        let mut compressed = CompressedAEG::new();
        
        // Create one node per basic block
        for (label, &cfg_node) in &cfg.label_to_node {
            let node = BlockNode {
                label: label.clone(),
                entry_states: 1,
                exit_states: 1,
            };
            
            let node_idx = compressed.graph.add_node(node);
            compressed.block_map.insert(label.clone(), node_idx);
        }
        
        // Create edges based on CFG structure
        for edge_idx in cfg.graph.edge_indices() {
            let (src, dst) = cfg.graph.edge_endpoints(edge_idx).unwrap();
            let src_label = &cfg.graph[src].label;
            let dst_label = &cfg.graph[dst].label;
            
            if let (Some(&src_node), Some(&dst_node)) = (
                compressed.block_map.get(src_label),
                compressed.block_map.get(dst_label)
            ) {
                // Compute worst-case timing for this edge
                let max_cycles = self.compute_worst_case_timing(aeg, src, dst);
                
                let edge = BlockEdge::new(max_cycles, max_cycles);
                compressed.graph.add_edge(src_node, dst_node, edge);
            }
        }
        
        // Set entry
        if let Some(&entry_node) = compressed.block_map.get(&cfg.graph[cfg.entry].label) {
            compressed.entry = entry_node;
        }
        
        // Set exits
        for &exit in &cfg.exits {
            if let Some(&exit_node) = compressed.block_map.get(&cfg.graph[exit].label) {
                compressed.exits.push(exit_node);
            }
        }
        
        compressed
    }
}

impl EfficientCompression {
    fn compute_worst_case_timing(&self, _aeg: &AEG, _src: NodeIndex, _dst: NodeIndex) -> u32 {
        // Simplified: return conservative estimate
        // In practice, would analyze all paths through AEG
        100
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::parser::IRParser;
    
    #[test]
    fn test_compressed_aeg_creation() {
        let compressed = CompressedAEG::new();
        assert_eq!(compressed.graph.node_count(), 0);
    }
    
    #[test]
    fn test_block_edge_creation() {
        let edge = BlockEdge::new(10, 20);
        assert_eq!(edge.min_cycles, 10);
        assert_eq!(edge.max_cycles, 20);
        assert!(edge.ilp_var.is_none());
    }
    
    #[test]
    fn test_compression_mode() {
        assert_eq!(CompressionMode::Precise, CompressionMode::Precise);
        assert_ne!(CompressionMode::Precise, CompressionMode::Efficient);
    }
}
