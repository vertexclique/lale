//! Control Flow Graph - Minimal stub for compatibility
//!
//! This is a minimal CFG implementation to maintain compatibility with
//! existing analysis modules (IPET, loops, cache analysis).
//! For new code, use InkwellCFG instead.

use ahash::AHashMap;
use petgraph::graph::{DiGraph, NodeIndex};

/// Edge type in CFG
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    Direct,
    Conditional,
    ConditionalTrue,
    ConditionalFalse,
    Switch,
    LoopBack,
}

/// Basic block in CFG
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub label: String,
    pub instructions: Vec<String>,
    pub execution_count_var: usize,
}

/// Control Flow Graph
pub struct CFG {
    pub graph: DiGraph<BasicBlock, EdgeType>,
    pub entry: NodeIndex,
    pub exits: Vec<NodeIndex>,
    pub label_to_node: AHashMap<String, NodeIndex>,
}
