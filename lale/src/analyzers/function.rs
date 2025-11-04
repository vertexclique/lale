//! Function-level WCET analysis
//!
//! Provides detailed analysis of individual functions.

use crate::analysis::{IPETSolver, InkwellTimingCalculator, Loop, LoopAnalyzer};
use crate::ir::{BasicBlock, EdgeType, InkwellCFG, CFG};
use crate::platform::PlatformModel;
use ahash::AHashMap;
use inkwell::values::FunctionValue;
use petgraph::graph::DiGraph;

/// Detailed result of analyzing a function
#[derive(Debug, Clone)]
pub struct FunctionAnalysisResult {
    /// Function name
    pub function_name: String,

    /// WCET in cycles (using IPET if possible)
    pub wcet_cycles: u64,

    /// BCET in cycles (best case)
    pub bcet_cycles: u64,

    /// Number of basic blocks
    pub block_count: usize,

    /// Number of CFG edges
    pub edge_count: usize,

    /// Loops detected
    pub loops: Vec<Loop>,

    /// Per-block timing information
    pub block_timings: AHashMap<usize, u64>,
}

/// Analyzer for individual functions with detailed analysis
pub struct FunctionAnalyzer {
    platform: PlatformModel,
}

impl FunctionAnalyzer {
    /// Create a new function analyzer with the given platform
    pub fn new(platform: PlatformModel) -> Self {
        Self { platform }
    }

    /// Analyze a function with full IPET-based WCET analysis
    pub fn analyze(&self, function: &FunctionValue) -> Result<FunctionAnalysisResult, String> {
        let func_name = function.get_name().to_str().unwrap_or("").to_string();

        // Build CFG
        let inkwell_cfg = InkwellCFG::from_function(function);

        // Calculate block timings
        let block_timings = InkwellTimingCalculator::calculate_block_timings(
            function,
            &inkwell_cfg,
            &self.platform,
        );

        // Convert to CFG format for IPET solver
        let cfg = self.convert_to_cfg(&inkwell_cfg);

        // Analyze loops
        let loops = LoopAnalyzer::analyze_loops(&cfg);

        // Convert timings to Cycles format for IPET
        let ipet_timings: AHashMap<_, _> = block_timings
            .iter()
            .filter_map(|(&block_id, &cycles)| {
                // Find corresponding node in CFG
                cfg.graph
                    .node_indices()
                    .find(|&idx| cfg.graph[idx].execution_count_var == block_id)
                    .map(|idx| (idx, crate::analysis::Cycles::new(cycles as u32)))
            })
            .collect();

        // Solve WCET using IPET
        let wcet_cycles =
            IPETSolver::solve_wcet(&cfg, &ipet_timings, &loops).unwrap_or_else(|_| {
                // Fallback: sum all block timings
                block_timings.values().sum()
            });

        // BCET is sum of minimum path (conservative estimate)
        let bcet_cycles: u64 = block_timings.values().copied().min().unwrap_or(0);

        let block_count = inkwell_cfg.blocks.len();
        let edge_count: usize = inkwell_cfg
            .blocks
            .iter()
            .map(|b| inkwell_cfg.successors(b.id).len())
            .sum();

        Ok(FunctionAnalysisResult {
            function_name: func_name,
            wcet_cycles,
            bcet_cycles,
            block_count,
            edge_count,
            loops,
            block_timings,
        })
    }

    /// Analyze with simple timing (no IPET)
    pub fn analyze_simple(
        &self,
        function: &FunctionValue,
    ) -> Result<FunctionAnalysisResult, String> {
        let func_name = function.get_name().to_str().unwrap_or("").to_string();

        // Build CFG
        let cfg = InkwellCFG::from_function(function);

        // Calculate block timings
        let block_timings =
            InkwellTimingCalculator::calculate_block_timings(function, &cfg, &self.platform);

        // Simple WCET: sum all blocks
        let wcet_cycles: u64 = block_timings.values().sum();
        let bcet_cycles: u64 = block_timings.values().copied().min().unwrap_or(0);

        let block_count = cfg.blocks.len();
        let edge_count: usize = cfg.blocks.iter().map(|b| cfg.successors(b.id).len()).sum();

        Ok(FunctionAnalysisResult {
            function_name: func_name,
            wcet_cycles,
            bcet_cycles,
            block_count,
            edge_count,
            loops: vec![],
            block_timings,
        })
    }

    /// Convert InkwellCFG to CFG for IPET solver
    fn convert_to_cfg(&self, inkwell_cfg: &InkwellCFG) -> CFG {
        let mut graph = DiGraph::new();
        let mut label_to_node = AHashMap::new();
        let mut id_to_node = AHashMap::new();

        // Create nodes
        for block in &inkwell_cfg.blocks {
            let cfg_block = BasicBlock {
                label: block.name.clone(),
                instructions: vec![],
                execution_count_var: block.id,
            };

            let node = graph.add_node(cfg_block);
            label_to_node.insert(block.name.clone(), node);
            id_to_node.insert(block.id, node);
        }

        // Create edges
        for block in &inkwell_cfg.blocks {
            let from_node = id_to_node[&block.id];
            for &succ_id in &inkwell_cfg.successors(block.id) {
                let to_node = id_to_node[&succ_id];
                graph.add_edge(from_node, to_node, EdgeType::Direct);
            }
        }

        let entry = id_to_node[&inkwell_cfg.entry_block];

        // Find exit nodes (nodes with no successors)
        let exits: Vec<_> = graph
            .node_indices()
            .filter(|&idx| graph.neighbors(idx).count() == 0)
            .collect();

        CFG {
            graph,
            entry,
            exits,
            label_to_node,
        }
    }
}
