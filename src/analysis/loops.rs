use crate::ir::CFG;
use petgraph::algo::dominators::{simple_fast, Dominators};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::HashSet;
use ahash::AHashMap;

/// Loop bounds information
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopBounds {
    Constant { min: u64, max: u64 },
    Parametric { expr: String },
    Unknown,
}

/// Loop structure
#[derive(Debug, Clone)]
pub struct Loop {
    pub header: NodeIndex,
    pub back_edges: Vec<(NodeIndex, NodeIndex)>,
    pub body_blocks: HashSet<NodeIndex>,
    pub nesting_level: usize,
    pub bounds: LoopBounds,
}

/// Loop analyzer
pub struct LoopAnalyzer;

impl LoopAnalyzer {
    /// Analyze loops in CFG
    pub fn analyze_loops(cfg: &CFG) -> Vec<Loop> {
        // Step 1: Compute dominator tree
        let dominators = simple_fast(&cfg.graph, cfg.entry);

        // Step 2: Find back edges (edges where target dominates source)
        let back_edges = Self::find_back_edges(cfg, &dominators);

        // Step 3: Identify natural loops for each back edge
        let mut loops = Vec::new();
        for (tail, head) in back_edges {
            let body = Self::find_loop_body(cfg, head, tail);
            let bounds = Self::extract_bounds(cfg, head, &body);

            loops.push(Loop {
                header: head,
                back_edges: vec![(tail, head)],
                body_blocks: body,
                nesting_level: 0, // Will be computed later
                bounds,
            });
        }

        // Step 4: Compute nesting levels
        Self::compute_nesting_levels(&mut loops);

        loops
    }

    /// Find back edges (edges where target dominates source)
    fn find_back_edges(
        cfg: &CFG,
        dominators: &Dominators<NodeIndex>,
    ) -> Vec<(NodeIndex, NodeIndex)> {
        let mut back_edges = Vec::new();

        for edge in cfg.graph.edge_references() {
            let source = edge.source();
            let target = edge.target();

            // Back edge: target dominates source
            if dominators
                .dominators(source)
                .map_or(false, |mut doms| doms.any(|d| d == target))
            {
                back_edges.push((source, target));
            }
        }

        back_edges
    }

    /// Find loop body given header and tail of back edge
    fn find_loop_body(cfg: &CFG, header: NodeIndex, tail: NodeIndex) -> HashSet<NodeIndex> {
        let mut body = HashSet::new();
        body.insert(header);

        if header == tail {
            return body;
        }

        // Traverse backwards from tail to header
        let mut worklist = vec![tail];
        body.insert(tail);

        while let Some(node) = worklist.pop() {
            for pred in cfg.graph.neighbors_directed(node, Direction::Incoming) {
                if !body.contains(&pred) {
                    body.insert(pred);
                    worklist.push(pred);
                }
            }
        }

        body
    }

    /// Extract loop bounds from metadata or analysis
    fn extract_bounds(cfg: &CFG, header: NodeIndex, body: &HashSet<NodeIndex>) -> LoopBounds {
        // Priority order:
        // 1. Check for user annotations in metadata
        if let Some(bounds) = Self::check_metadata_bounds(cfg, header) {
            return bounds;
        }

        // 2. Analyze induction variables
        if let Some(bounds) = Self::analyze_induction_variables(cfg, header, body) {
            return bounds;
        }

        // 3. Pattern matching for common loop forms
        if let Some(bounds) = Self::pattern_match_loop(cfg, header, body) {
            return bounds;
        }

        // 4. Conservative default
        LoopBounds::Unknown
    }

    /// Check for loop bound metadata annotations
    fn check_metadata_bounds(_cfg: &CFG, _header: NodeIndex) -> Option<LoopBounds> {
        // In a real implementation, this would parse LLVM metadata
        // Format: !loop_bound !{ i64 min, i64 max }
        // For now, return None (no metadata found)
        None
    }

    /// Analyze induction variables to determine bounds
    fn analyze_induction_variables(
        _cfg: &CFG,
        _header: NodeIndex,
        _body: &HashSet<NodeIndex>,
    ) -> Option<LoopBounds> {
        // In a real implementation, this would:
        // 1. Identify induction variables (i, j, etc.)
        // 2. Track their initialization, increment, and exit condition
        // 3. Solve for iteration count
        //
        // Example: for (i = 0; i < N; i++) → bounds = [0, N]
        //
        // This requires data flow analysis and symbolic execution
        None
    }

    /// Pattern match common loop forms
    fn pattern_match_loop(
        _cfg: &CFG,
        _header: NodeIndex,
        _body: &HashSet<NodeIndex>,
    ) -> Option<LoopBounds> {
        // In a real implementation, this would recognize patterns like:
        // - for (i = 0; i < 10; i++) → Constant { min: 0, max: 10 }
        // - while (condition) → Unknown
        // - do-while → Unknown
        //
        // This requires analyzing the loop header's branch condition
        None
    }

    /// Compute nesting levels for loops
    fn compute_nesting_levels(loops: &mut [Loop]) {
        // Build containment relationships
        let mut nesting_map: AHashMap<usize, usize> = AHashMap::new();

        for (i, loop_i) in loops.iter().enumerate() {
            let mut max_nesting = 0;

            for (j, loop_j) in loops.iter().enumerate() {
                if i != j && loop_j.body_blocks.contains(&loop_i.header) {
                    // loop_i is nested inside loop_j
                    max_nesting = max_nesting.max(loop_j.nesting_level + 1);
                }
            }

            nesting_map.insert(i, max_nesting);
        }

        // Apply nesting levels
        for (i, loop_item) in loops.iter_mut().enumerate() {
            loop_item.nesting_level = nesting_map[&i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::parser::IRParser;
    use crate::ir::CFG;

    #[test]
    fn test_loop_detection() {
        let sample_path = "data/armv7e-m/56e3741adeae4068.ll";
        if std::path::Path::new(sample_path).exists() {
            let module = IRParser::parse_file(sample_path).unwrap();

            // Analyze loops in first function
            if let Some(function) = module.functions.first() {
                let cfg = CFG::from_function(function);
                let loops = LoopAnalyzer::analyze_loops(&cfg);

                // Just verify it doesn't crash
                println!("Found {} loops", loops.len());
            }
        }
    }
}
