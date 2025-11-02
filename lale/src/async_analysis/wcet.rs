//! Per-segment WCET analysis
//!
//! Analyzes WCET for individual execution segments using existing LALE infrastructure.

use crate::analysis::{IPETSolver, LoopAnalyzer, TimingCalculator};
use crate::async_analysis::segment::{ActorSegment, SegmentType};
use crate::ir::CFG;
use crate::platform::PlatformModel;
use ahash::AHashMap;
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};

/// Segment WCET result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentWCET {
    /// Segment ID
    pub segment_id: u32,

    /// WCET in cycles
    pub wcet_cycles: u64,

    /// BCET in cycles (conservative estimate)
    pub bcet_cycles: u64,
}

/// Per-segment WCET analyzer
pub struct SegmentWCETAnalyzer {
    platform: PlatformModel,
}

impl SegmentWCETAnalyzer {
    /// Create analyzer with platform model
    pub fn new(platform: PlatformModel) -> Self {
        Self { platform }
    }

    /// Analyze WCET for all segments
    ///
    /// Builds segment-restricted CFGs and uses IPET solver for accurate WCET.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use lale::async_analysis::SegmentWCETAnalyzer;
    /// use lale::platform::CortexM7Model;
    ///
    /// let analyzer = SegmentWCETAnalyzer::new(CortexM7Model::new());
    /// let wcets = analyzer.analyze_segments(&function, &segments);
    /// ```
    pub fn analyze_segments(
        &self,
        function: &llvm_ir::Function,
        segments: &[ActorSegment],
    ) -> AHashMap<u32, SegmentWCET> {
        let mut results = AHashMap::new();

        // Build full CFG once
        let full_cfg = CFG::from_function(function);

        for segment in segments {
            let wcet = self.analyze_single_segment(function, segment, &full_cfg);
            results.insert(segment.segment_id, wcet);
        }

        results
    }

    /// Analyze WCET for a single segment
    fn analyze_single_segment(
        &self,
        function: &llvm_ir::Function,
        segment: &ActorSegment,
        full_cfg: &CFG,
    ) -> SegmentWCET {
        // Build segment-restricted CFG
        let segment_cfg = self.build_segment_cfg(segment, full_cfg);

        // Calculate block timings
        let timings =
            TimingCalculator::calculate_block_timings(function, &segment_cfg, &self.platform);

        // Analyze loops within segment
        let loops = LoopAnalyzer::analyze_loops(&segment_cfg);

        // Solve WCET using IPET
        let wcet_cycles = match IPETSolver::solve_wcet(&segment_cfg, &timings, &loops) {
            Ok(wcet) => wcet,
            Err(e) => {
                eprintln!(
                    "WCET calculation failed for segment {}: {}",
                    segment.segment_id, e
                );
                // Fallback: sum of all block worst-case timings
                timings.values().map(|c| c.worst_case as u64).sum()
            }
        };

        // Conservative BCET: sum of best-case timings
        let bcet_cycles: u64 = timings.values().map(|c| c.best_case as u64).sum();

        SegmentWCET {
            segment_id: segment.segment_id,
            wcet_cycles,
            bcet_cycles,
        }
    }

    /// Build CFG restricted to segment blocks
    fn build_segment_cfg(&self, segment: &ActorSegment, full_cfg: &CFG) -> CFG {
        use crate::ir::{BasicBlock, EdgeType};
        use petgraph::graph::DiGraph;

        let mut graph = DiGraph::new();
        let mut label_to_node = AHashMap::new();
        let mut exits = Vec::new();

        // Create nodes for segment blocks only
        for block_label in &segment.blocks {
            if let Some(&full_node) = full_cfg.label_to_node.get(block_label) {
                let full_block = &full_cfg.graph[full_node];

                let block = BasicBlock {
                    label: full_block.label.clone(),
                    instructions: full_block.instructions.clone(),
                    execution_count_var: full_block.execution_count_var,
                };

                let node = graph.add_node(block);
                label_to_node.insert(block_label.clone(), node);
            }
        }

        // Entry is segment entry block
        let entry = label_to_node
            .get(&segment.entry_block)
            .copied()
            .expect("Segment entry block must exist");

        // Add edges between segment blocks
        for block_label in &segment.blocks {
            if let Some(&full_node) = full_cfg.label_to_node.get(block_label) {
                let source = label_to_node[block_label];

                // Copy edges that stay within segment
                use petgraph::visit::EdgeRef;
                for edge in full_cfg.graph.edges(full_node) {
                    let target_block = &full_cfg.graph[edge.target()];

                    if segment.blocks.contains(&target_block.label) {
                        if let Some(&target) = label_to_node.get(&target_block.label) {
                            graph.add_edge(source, target, *edge.weight());
                        }
                    } else {
                        // Edge leaves segment - mark source as exit
                        if !exits.contains(&source) {
                            exits.push(source);
                        }
                    }
                }

                // Check if this is an exit block
                if segment.exit_blocks.contains(block_label) {
                    if !exits.contains(&source) {
                        exits.push(source);
                    }
                }
            }
        }

        // If no exits found, entry is also exit (single-block segment)
        if exits.is_empty() {
            exits.push(entry);
        }

        CFG {
            graph,
            entry,
            exits,
            label_to_node,
        }
    }

    /// Get platform model
    pub fn platform(&self) -> &PlatformModel {
        &self.platform
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::CortexM4Model;

    #[test]
    fn test_wcet_analyzer_creation() {
        let platform = CortexM4Model::new();
        let _analyzer = SegmentWCETAnalyzer::new(platform);
    }
}
