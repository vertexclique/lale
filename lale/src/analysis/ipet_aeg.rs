use crate::aeg::compression::CompressedAEG;
use crate::analysis::Loop;
use ahash::AHashMap;
use good_lp::*;
use petgraph::visit::EdgeRef;
use petgraph::Direction;

/// IPET solver for AEG (Abstract Execution Graph)
/// Uses ILP with variables per AEG edge instead of per basic block
pub struct AEGIPETSolver;

impl AEGIPETSolver {
    /// Solve WCET using AEG-based IPET
    /// Key difference: variables per edge, edges have context-dependent weights
    pub fn solve_wcet(aeg: &CompressedAEG, loops: &[Loop]) -> Result<u64, String> {
        // Step 1: Create ILP variables (one per AEG edge)
        let mut vars = ProblemVariables::new();
        let mut edge_vars = AHashMap::new();

        for edge_idx in aeg.graph.edge_indices() {
            edge_vars.insert(edge_idx, vars.add(variable().integer().min(0)));
        }

        // Step 2: Build objective function (maximize execution time)
        let objective: Expression = edge_vars
            .iter()
            .map(|(&edge_idx, &var)| {
                let edge = &aeg.graph[edge_idx];
                // Use worst-case cycles for this edge
                var * (edge.max_cycles as i32)
            })
            .sum();

        // Step 3: Start building problem with constraints
        let mut problem = vars.maximise(objective.clone()).using(default_solver);

        // Predefined determinism
        problem.set_parameter("randomSeed", "42");

        // Suppress solver output
        problem.set_parameter("loglevel", "0");

        // Constraint 1: Entry node - sum of incoming edges = 1
        let entry_incoming: Expression = aeg
            .graph
            .edges_directed(aeg.entry, Direction::Incoming)
            .map(|e| edge_vars[&e.id()])
            .sum();

        // If no incoming edges to entry, it executes once
        if aeg
            .graph
            .edges_directed(aeg.entry, Direction::Incoming)
            .count()
            == 0
        {
            // Entry executes once - represented by outgoing edges
            let entry_outgoing: Expression = aeg
                .graph
                .edges_directed(aeg.entry, Direction::Outgoing)
                .map(|e| edge_vars[&e.id()])
                .sum();
            problem = problem.with(constraint!(entry_outgoing == 1));
        } else {
            problem = problem.with(constraint!(entry_incoming == 1));
        }

        // Constraint 2: Flow conservation at each node
        for node in aeg.graph.node_indices() {
            // Skip entry and exit nodes
            if node == aeg.entry || aeg.exits.contains(&node) {
                continue;
            }

            let incoming: Expression = aeg
                .graph
                .edges_directed(node, Direction::Incoming)
                .map(|e| edge_vars[&e.id()])
                .sum();

            let outgoing: Expression = aeg
                .graph
                .edges_directed(node, Direction::Outgoing)
                .map(|e| edge_vars[&e.id()])
                .sum();

            problem = problem.with(constraint!(incoming == outgoing));
        }

        // Constraint 3: Loop bounds
        // For each loop: sum of backedges ≤ max_iter * sum of entry edges
        for loop_info in loops {
            // Find edges corresponding to loop header
            let header_label = &loop_info.header;

            // Find header node in compressed AEG
            if let Some(&header_node) = aeg.block_map.get(&format!("{:?}", header_label)) {
                // Entry edges to loop
                let entry_edges: Expression = aeg
                    .graph
                    .edges_directed(header_node, Direction::Incoming)
                    .filter(|e| {
                        let src = e.source();
                        // Entry edge if source is not in loop body
                        !loop_info.body_blocks.contains(&src)
                    })
                    .map(|e| edge_vars[&e.id()])
                    .sum();

                // Backedges in loop
                let backedges: Expression = aeg
                    .graph
                    .edges_directed(header_node, Direction::Incoming)
                    .filter(|e| {
                        let src = e.source();
                        // Backedge if source is in loop body
                        loop_info.body_blocks.contains(&src)
                    })
                    .map(|e| edge_vars[&e.id()])
                    .sum();

                // Get max iterations
                let max_iter = match &loop_info.bounds {
                    crate::analysis::LoopBounds::Constant { max, .. } => *max as i32,
                    _ => 100, // Conservative default
                };

                problem = problem.with(constraint!(backedges <= max_iter * entry_edges));
            }
        }

        // Step 4: Solve the ILP problem
        let solution = problem
            .solve()
            .map_err(|e| format!("ILP solver failed: {:?}", e))?;

        // Step 5: Extract WCET from solution
        let wcet = solution.eval(&objective);

        Ok(wcet as u64)
    }

    /// Extract execution counts for each edge
    pub fn extract_edge_counts(
        aeg: &CompressedAEG,
        _loops: &[Loop],
    ) -> Result<AHashMap<petgraph::graph::EdgeIndex, u64>, String> {
        // Simplified: just solve WCET and return edge counts
        // In practice, would extract from the ILP solution
        let mut counts = AHashMap::new();

        // For now, return 1 for each edge (simplified)
        for edge_idx in aeg.graph.edge_indices() {
            counts.insert(edge_idx, 1);
        }

        Ok(counts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aeg::compression::{BlockEdge, BlockNode, CompressedAEG};

    fn create_simple_aeg() -> CompressedAEG {
        let mut aeg = CompressedAEG::new();

        // Create simple linear graph: A -> B -> C
        let node_a = aeg.graph.add_node(BlockNode {
            label: "A".to_string(),
            entry_states: 1,
            exit_states: 1,
        });

        let node_b = aeg.graph.add_node(BlockNode {
            label: "B".to_string(),
            entry_states: 1,
            exit_states: 1,
        });

        let node_c = aeg.graph.add_node(BlockNode {
            label: "C".to_string(),
            entry_states: 1,
            exit_states: 1,
        });

        aeg.graph.add_edge(node_a, node_b, BlockEdge::new(10, 10));
        aeg.graph.add_edge(node_b, node_c, BlockEdge::new(20, 20));

        aeg.entry = node_a;
        aeg.exits.push(node_c);

        aeg.block_map.insert("A".to_string(), node_a);
        aeg.block_map.insert("B".to_string(), node_b);
        aeg.block_map.insert("C".to_string(), node_c);

        aeg
    }

    #[test]
    fn test_simple_linear_path() {
        let aeg = create_simple_aeg();
        let loops = vec![];

        let result = AEGIPETSolver::solve_wcet(&aeg, &loops);

        assert!(result.is_ok());
        let wcet = result.unwrap();

        // Should be 10 + 20 = 30 cycles
        assert_eq!(wcet, 30);
    }

    #[test]
    fn test_edge_count_extraction() {
        let aeg = create_simple_aeg();
        let loops = vec![];

        let result = AEGIPETSolver::extract_edge_counts(&aeg, &loops);

        assert!(result.is_ok());
        let counts = result.unwrap();

        // Each edge should execute once
        for (_edge_idx, &count) in &counts {
            assert_eq!(count, 1);
        }
    }
}
