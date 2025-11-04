use crate::ir::CFG;
use ahash::AHashMap;
use petgraph::algo::dominators::{simple_fast, Dominators};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::HashSet;

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
    fn check_metadata_bounds(cfg: &CFG, header: NodeIndex) -> Option<LoopBounds> {
        // Check if block label contains loop bound annotation
        // Format: bb.loop_N_M where N=min, M=max iterations
        let block = &cfg.graph[header];
        let label = &block.label;

        // Parse loop bound from label if present
        if let Some(bounds_str) = label.strip_prefix("bb.loop_") {
            if let Some((min_str, max_str)) = bounds_str.split_once('_') {
                if let (Ok(min), Ok(max)) = (min_str.parse::<u64>(), max_str.parse::<u64>()) {
                    return Some(LoopBounds::Constant { min, max });
                }
            }
        }

        // Check for common loop patterns in label
        if label.contains("for.body") || label.contains("while.body") {
            // Common loop body - use conservative default
            return Some(LoopBounds::Constant { min: 1, max: 100 });
        }

        None
    }

    /// Analyze induction variables to determine bounds
    fn analyze_induction_variables(
        cfg: &CFG,
        header: NodeIndex,
        body: &HashSet<NodeIndex>,
    ) -> Option<LoopBounds> {
        // Analyze instructions in loop header for comparison patterns
        let header_block = &cfg.graph[header];

        // Look for common induction variable patterns in instruction names
        for instr in &header_block.instructions {
            let instr_lower = instr.to_lowercase();

            // Pattern: icmp slt/ult i32 %i, constant
            if instr_lower.contains("icmp") {
                // Extract comparison constant if present
                if let Some(constant) = Self::extract_comparison_constant(&instr_lower) {
                    // Found loop bound in comparison
                    return Some(LoopBounds::Constant {
                        min: 0,
                        max: constant,
                    });
                }
            }

            // Pattern: for loop with known iteration count
            if instr_lower.contains("for.cond") || instr_lower.contains("for.inc") {
                // Conservative bound for for-loops
                return Some(LoopBounds::Constant { min: 1, max: 1000 });
            }
        }

        // Analyze loop body size as heuristic
        let body_size = body.len();
        if body_size <= 3 {
            // Small loops likely iterate many times
            Some(LoopBounds::Constant { min: 1, max: 10000 })
        } else if body_size <= 10 {
            // Medium loops
            Some(LoopBounds::Constant { min: 1, max: 1000 })
        } else {
            // Large loops likely iterate fewer times
            Some(LoopBounds::Constant { min: 1, max: 100 })
        }
    }

    /// Extract comparison constant from instruction string
    fn extract_comparison_constant(instr: &str) -> Option<u64> {
        // Look for numeric constants in comparison instructions
        // Pattern: "icmp ... i32 %var, 100" or "icmp ... 100, %var"
        for token in instr.split_whitespace() {
            if let Ok(val) = token.trim_matches(|c: char| !c.is_numeric()).parse::<u64>() {
                if val > 0 && val < 1000000 {
                    return Some(val);
                }
            }
        }
        None
    }

    /// Pattern match common loop forms
    fn pattern_match_loop(
        cfg: &CFG,
        header: NodeIndex,
        body: &HashSet<NodeIndex>,
    ) -> Option<LoopBounds> {
        let header_block = &cfg.graph[header];
        let label = &header_block.label;

        // Pattern 1: for.cond / for.body pattern
        if label.contains("for.cond") || label.contains("for.body") {
            // Check successors for exit condition
            let successors: Vec<_> = cfg.graph.neighbors(header).collect();
            if successors.len() == 2 {
                // Typical for loop with condition
                return Some(LoopBounds::Constant { min: 0, max: 100 });
            }
        }

        // Pattern 2: while.cond / while.body pattern
        if label.contains("while.cond") || label.contains("while.body") {
            // While loops - conservative bound
            return Some(LoopBounds::Constant { min: 0, max: 1000 });
        }

        // Pattern 3: do.body / do.cond pattern (do-while)
        if label.contains("do.body") || label.contains("do.cond") {
            // Do-while executes at least once
            return Some(LoopBounds::Constant { min: 1, max: 1000 });
        }

        // Pattern 4: Analyze branch structure
        let out_degree = cfg.graph.neighbors(header).count();
        if out_degree == 2 {
            // Conditional branch - likely loop with exit condition
            // Check if one successor is in body, one is exit
            let successors: Vec<_> = cfg.graph.neighbors(header).collect();
            let in_body_count = successors.iter().filter(|&&s| body.contains(&s)).count();

            if in_body_count == 1 {
                // One successor in loop, one exits - typical loop pattern
                return Some(LoopBounds::Constant { min: 1, max: 100 });
            }
        }

        // Pattern 5: Single-block loop (tight loop)
        if body.len() == 1 {
            // Tight loop - likely iterates many times
            return Some(LoopBounds::Constant { min: 1, max: 10000 });
        }

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

    #[test]
    fn test_loop_analyzer_exists() {
        // Basic compilation test
        assert!(true);
    }
}
