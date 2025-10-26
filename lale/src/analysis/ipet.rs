use crate::analysis::{Cycles, Loop, LoopBounds};
use crate::ir::CFG;
use ahash::AHashMap;
use good_lp::*;
use petgraph::graph::NodeIndex;
use petgraph::Direction;

/// IPET solver for WCET calculation using Integer Linear Programming
pub struct IPETSolver;

impl IPETSolver {
    /// Solve WCET using full IPET with ILP solver
    pub fn solve_wcet(
        cfg: &CFG,
        timings: &AHashMap<NodeIndex, Cycles>,
        loops: &[Loop],
    ) -> Result<u64, String> {
        // Step 1: Create ILP variables
        let mut vars = ProblemVariables::new();
        let mut block_vars = AHashMap::new();

        for node in cfg.graph.node_indices() {
            block_vars.insert(node, vars.add(variable().integer().min(0)));
        }

        // Step 2: Build objective function (maximize execution time)
        let objective: Expression = block_vars
            .iter()
            .map(|(&node, &var)| {
                let cycles = timings.get(&node).map(|c| c.worst_case).unwrap_or(1) as i32;
                var * cycles
            })
            .sum();

        // Step 3: Start building problem with constraints
        let mut problem = vars.maximise(objective.clone()).using(default_solver);

        // Suppress output. Carry me back home.
        problem.set_parameter("loglevel", "0");

        // Constraint: Entry block executes exactly once
        problem = problem.with(constraint!(block_vars[&cfg.entry] == 1));

        // Constraint: Flow conservation (incoming = outgoing for non-entry/exit blocks)
        for node in cfg.graph.node_indices() {
            if node == cfg.entry || cfg.exits.contains(&node) {
                continue;
            }

            let incoming: Expression = cfg
                .graph
                .neighbors_directed(node, Direction::Incoming)
                .map(|pred| block_vars[&pred])
                .sum();

            let outgoing: Expression = cfg
                .graph
                .neighbors_directed(node, Direction::Outgoing)
                .map(|succ| block_vars[&succ])
                .sum();

            problem = problem.with(constraint!(incoming == outgoing));
        }

        // Constraint: Loop bounds
        for loop_info in loops {
            let header_var = block_vars[&loop_info.header];

            let max_iterations = match &loop_info.bounds {
                LoopBounds::Constant { max, .. } => *max as i32,
                _ => 100, // Conservative default
            };

            // Each loop body block executes at most max_iterations * header executions
            for &body_block in &loop_info.body_blocks {
                if body_block != loop_info.header {
                    let body_var = block_vars[&body_block];
                    problem = problem.with(constraint!(body_var <= max_iterations * header_var));
                }
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

    /// Extract execution counts from ILP solution
    pub fn extract_execution_counts(
        cfg: &CFG,
        timings: &AHashMap<NodeIndex, Cycles>,
        loops: &[Loop],
    ) -> Result<AHashMap<NodeIndex, u64>, String> {
        // Solve IPET to get solution
        let mut vars = ProblemVariables::new();
        let mut block_vars = AHashMap::new();

        for node in cfg.graph.node_indices() {
            block_vars.insert(node, vars.add(variable().integer().min(0)));
        }

        let objective: Expression = block_vars
            .iter()
            .map(|(&node, &var)| {
                let cycles = timings.get(&node).map(|c| c.worst_case).unwrap_or(1) as i32;
                var * cycles
            })
            .sum();

        let mut problem = vars.maximise(objective).using(default_solver);
        problem.set_parameter("loglevel", "0");
        problem = problem.with(constraint!(block_vars[&cfg.entry] == 1));

        for node in cfg.graph.node_indices() {
            if node == cfg.entry || cfg.exits.contains(&node) {
                continue;
            }

            let incoming: Expression = cfg
                .graph
                .neighbors_directed(node, Direction::Incoming)
                .map(|pred| block_vars[&pred])
                .sum();

            let outgoing: Expression = cfg
                .graph
                .neighbors_directed(node, Direction::Outgoing)
                .map(|succ| block_vars[&succ])
                .sum();

            problem = problem.with(constraint!(incoming == outgoing));
        }

        for loop_info in loops {
            let header_var = block_vars[&loop_info.header];
            let max_iterations = match &loop_info.bounds {
                LoopBounds::Constant { max, .. } => *max as i32,
                _ => 100,
            };

            for &body_block in &loop_info.body_blocks {
                if body_block != loop_info.header {
                    let body_var = block_vars[&body_block];
                    problem = problem.with(constraint!(body_var <= max_iterations * header_var));
                }
            }
        }

        let solution = problem
            .solve()
            .map_err(|e| format!("ILP solver failed: {:?}", e))?;

        // Extract execution counts
        let mut counts = AHashMap::new();
        for (&node, &var) in &block_vars {
            let count = solution.value(var) as u64;
            counts.insert(node, count);
        }

        Ok(counts)
    }

    /// Extract critical path from solution
    pub fn extract_critical_path(cfg: &CFG, solution: &AHashMap<NodeIndex, u64>) -> Vec<NodeIndex> {
        let mut path = Vec::new();
        let mut current = cfg.entry;

        path.push(current);

        // Follow path with highest execution counts
        while !cfg.exits.contains(&current) {
            let next = cfg
                .graph
                .neighbors_directed(current, Direction::Outgoing)
                .max_by_key(|&succ| solution.get(&succ).copied().unwrap_or(0));

            if let Some(next_node) = next {
                path.push(next_node);
                current = next_node;
            } else {
                break;
            }
        }

        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{LoopAnalyzer, TimingCalculator};
    use crate::ir::parser::IRParser;
    use crate::ir::CFG;
    use crate::platform::CortexM4Model;

    #[test]
    fn test_ipet_solver() {
        let sample_path = "data/armv7e-m/56e3741adeae4068.ll";
        if std::path::Path::new(sample_path).exists() {
            let module = IRParser::parse_file(sample_path).unwrap();
            let platform = CortexM4Model::new();

            if let Some(function) = module.functions.first() {
                let cfg = CFG::from_function(function);
                let loops = LoopAnalyzer::analyze_loops(&cfg);
                let timings = TimingCalculator::calculate_block_timings(function, &cfg, &platform);

                let result = IPETSolver::solve_wcet(&cfg, &timings, &loops);

                if let Ok(wcet) = result {
                    assert!(wcet > 0, "WCET should be positive");
                    println!("Calculated WCET: {} cycles", wcet);

                    // Test execution count extraction
                    let counts = IPETSolver::extract_execution_counts(&cfg, &timings, &loops);
                    assert!(counts.is_ok(), "Should extract execution counts");
                }
            }
        }
    }
}
