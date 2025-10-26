use ahash::AHashMap;
use llvm_ir::{Function, Terminator};
use petgraph::graph::{DiGraph, NodeIndex};

/// Edge type in control flow graph
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeType {
    Direct,
    ConditionalTrue,
    ConditionalFalse,
    LoopBack,
}

/// Basic block in CFG
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub label: String,
    pub instructions: Vec<String>,  // Instruction names/types for now
    pub execution_count_var: usize, // For ILP formulation
}

/// Control Flow Graph
pub struct CFG {
    pub graph: DiGraph<BasicBlock, EdgeType>,
    pub entry: NodeIndex,
    pub exits: Vec<NodeIndex>,
    pub label_to_node: AHashMap<String, NodeIndex>,
}

impl CFG {
    /// Build CFG from LLVM IR function
    pub fn from_function(function: &Function) -> Self {
        let mut graph = DiGraph::new();
        let mut label_to_node = AHashMap::new();
        let mut exits = Vec::new();

        // Create nodes for each basic block
        for (idx, bb) in function.basic_blocks.iter().enumerate() {
            let label = bb.name.to_string();
            let instructions: Vec<String> = bb
                .instrs
                .iter()
                .map(|instr| format!("{:?}", instr))
                .collect();

            let block = BasicBlock {
                label: label.clone(),
                instructions,
                execution_count_var: idx,
            };

            let node = graph.add_node(block);
            label_to_node.insert(label, node);
        }

        // Entry is first basic block
        let entry = *label_to_node
            .values()
            .next()
            .expect("Function must have at least one basic block");

        // Add edges based on terminators
        for bb in &function.basic_blocks {
            let source_label = bb.name.to_string();
            let source = label_to_node[&source_label];

            match &bb.term {
                Terminator::Ret(_) => {
                    // Exit block
                    exits.push(source);
                }
                Terminator::Br(br) => {
                    // Unconditional branch
                    let target_label = br.dest.to_string();
                    if let Some(&target) = label_to_node.get(&target_label) {
                        graph.add_edge(source, target, EdgeType::Direct);
                    }
                }
                Terminator::CondBr(condbr) => {
                    // Conditional branch
                    let true_label = condbr.true_dest.to_string();
                    let false_label = condbr.false_dest.to_string();

                    if let Some(&true_target) = label_to_node.get(&true_label) {
                        graph.add_edge(source, true_target, EdgeType::ConditionalTrue);
                    }
                    if let Some(&false_target) = label_to_node.get(&false_label) {
                        graph.add_edge(source, false_target, EdgeType::ConditionalFalse);
                    }
                }
                Terminator::Switch(switch) => {
                    // Switch statement - treat all as direct edges
                    for (_, dest) in &switch.dests {
                        let dest_label = dest.to_string();
                        if let Some(&target) = label_to_node.get(&dest_label) {
                            graph.add_edge(source, target, EdgeType::Direct);
                        }
                    }
                    // Default destination
                    let default_label = switch.default_dest.to_string();
                    if let Some(&target) = label_to_node.get(&default_label) {
                        graph.add_edge(source, target, EdgeType::Direct);
                    }
                }
                _ => {
                    // Other terminators (unreachable, etc.)
                    // May be exit blocks
                    exits.push(source);
                }
            }
        }

        CFG {
            graph,
            entry,
            exits,
            label_to_node,
        }
    }

    /// Get number of basic blocks
    pub fn block_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get number of edges
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::parser::IRParser;

    #[test]
    fn test_cfg_construction() {
        let sample_path = "data/armv7e-m/56e3741adeae4068.ll";
        if std::path::Path::new(sample_path).exists() {
            let module = IRParser::parse_file(sample_path).unwrap();

            // Build CFG for first function
            if let Some(function) = module.functions.first() {
                let cfg = CFG::from_function(function);
                assert!(cfg.block_count() > 0, "CFG should have basic blocks");
                assert!(cfg.exits.len() > 0, "CFG should have exit blocks");
            }
        }
    }
}
