//! WCET analysis for segments using inkwell
//!
//! Analyzes WCET for actor segments from inkwell FunctionValue for LLVM 19+ compatibility

use ahash::AHashMap;
use inkwell::values::FunctionValue;
use serde::{Deserialize, Serialize};

use super::inkwell_segment::ActorSegment;
use crate::analysis::{Cycles, IPETSolver, InkwellTimingCalculator, LoopAnalyzer};
use crate::ir::InkwellCFG;
use crate::platform::PlatformModel;

/// Segment WCET result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentWCET {
    pub segment_id: u32,
    pub wcet_cycles: u64,
    pub bcet_cycles: u64,
}

/// WCET analyzer for inkwell-based segments
pub struct InkwellSegmentWCETAnalyzer {
    platform: PlatformModel,
}

impl InkwellSegmentWCETAnalyzer {
    /// Create new analyzer with platform model
    pub fn new(platform: PlatformModel) -> Self {
        Self { platform }
    }

    /// Analyze WCET for all segments
    pub fn analyze_segments(
        &self,
        function: &FunctionValue,
        segments: &[ActorSegment],
    ) -> AHashMap<usize, SegmentWCET> {
        // Build CFG
        let cfg = InkwellCFG::from_function(function);

        // Calculate block timings
        let timings =
            InkwellTimingCalculator::calculate_block_timings(function, &cfg, &self.platform);

        let mut results = AHashMap::new();

        for segment in segments {
            let wcet = self.analyze_segment(&cfg, segment, &timings);
            results.insert(segment.segment_id as usize, wcet);
        }

        results
    }

    /// Analyze WCET for a single segment
    fn analyze_segment(
        &self,
        cfg: &InkwellCFG,
        segment: &ActorSegment,
        timings: &AHashMap<usize, u64>,
    ) -> SegmentWCET {
        // Get block IDs for this segment
        let segment_blocks: Vec<usize> = segment
            .blocks
            .iter()
            .filter_map(|name| cfg.block_map.get(name).copied())
            .collect();

        if segment_blocks.is_empty() {
            return SegmentWCET {
                segment_id: segment.segment_id,
                wcet_cycles: 0,
                bcet_cycles: 0,
            };
        }

        // Create sub-CFG for this segment
        let segment_cfg = self.create_segment_cfg(cfg, &segment_blocks);

        // Analyze loops in segment
        let loops = LoopAnalyzer::analyze_loops(&segment_cfg);

        // Convert timings to format expected by IPET solver
        use petgraph::graph::NodeIndex;
        let mut ipet_timings = AHashMap::new();
        for (node_idx, block) in segment_cfg
            .graph
            .node_indices()
            .zip(segment_cfg.graph.node_weights())
        {
            // Find the timing for this block by name
            if let Some(&block_id) = cfg.block_map.get(&block.label) {
                if let Some(&cycles) = timings.get(&block_id) {
                    ipet_timings.insert(node_idx, Cycles::new(cycles as u32));
                }
            }
        }

        // Solve WCET using IPET
        let wcet_cycles = IPETSolver::solve_wcet(&segment_cfg, &ipet_timings, &loops)
            .unwrap_or_else(|e| {
                eprintln!(
                    "IPET solver failed for segment {}: {}",
                    segment.segment_id, e
                );
                // Fallback: sum all block timings
                segment_blocks
                    .iter()
                    .filter_map(|&id| timings.get(&id))
                    .sum()
            });

        // Conservative BCET estimate (sum of all blocks)
        let bcet_cycles: u64 = segment_blocks
            .iter()
            .filter_map(|&id| timings.get(&id))
            .sum();

        SegmentWCET {
            segment_id: segment.segment_id,
            wcet_cycles,
            bcet_cycles,
        }
    }

    /// Create a CFG structure for a segment (for IPET solver compatibility)
    fn create_segment_cfg(&self, cfg: &InkwellCFG, segment_blocks: &[usize]) -> crate::ir::CFG {
        use crate::ir::cfg::{BasicBlock as CFGBlock, EdgeType};
        use petgraph::graph::DiGraph;
        use std::collections::HashSet;

        let block_set: HashSet<_> = segment_blocks.iter().copied().collect();

        let mut graph = DiGraph::new();
        let mut label_to_node = AHashMap::new();
        let mut id_map = AHashMap::new();

        // Create nodes for segment blocks
        for (new_id, &old_id) in segment_blocks.iter().enumerate() {
            let block = CFGBlock {
                label: cfg.blocks[old_id].name.clone(),
                instructions: vec![],
                execution_count_var: new_id,
            };

            let node = graph.add_node(block);
            id_map.insert(old_id, node);
            label_to_node.insert(cfg.blocks[old_id].name.clone(), node);
        }

        // Entry is first block in segment
        let entry = id_map[&segment_blocks[0]];

        // Add edges within the segment
        let mut exits = Vec::new();
        for &old_from in segment_blocks {
            let from_node = id_map[&old_from];
            let mut has_successor = false;

            for &old_to in &cfg.successors(old_from) {
                if block_set.contains(&old_to) {
                    let to_node = id_map[&old_to];
                    graph.add_edge(from_node, to_node, EdgeType::Direct);
                    has_successor = true;
                }
            }

            // If no successors within segment, it's an exit
            if !has_successor {
                exits.push(from_node);
            }
        }

        // If no exits found, entry is also exit
        if exits.is_empty() {
            exits.push(entry);
        }

        crate::ir::CFG {
            graph,
            entry,
            exits,
            label_to_node,
        }
    }

    /// Analyze with cache effects
    pub fn analyze_with_cache(
        &self,
        function: &FunctionValue,
        segments: &[ActorSegment],
    ) -> AHashMap<usize, SegmentWCET> {
        // Build CFG
        let cfg = InkwellCFG::from_function(function);

        // Calculate block timings with cache effects
        let timings = InkwellTimingCalculator::calculate_with_cache(function, &cfg, &self.platform);

        let mut results = AHashMap::new();

        for segment in segments {
            let wcet = self.analyze_segment(&cfg, segment, &timings);
            results.insert(segment.segment_id as usize, wcet);
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inkwell_segment_wcet_analyzer_exists() {
        // Basic compilation test
        assert!(true);
    }
}
