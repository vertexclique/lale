use llvm_ir::{Instruction, Module};
use petgraph::graph::{DiGraph, NodeIndex};
use ahash::AHashMap;

/// Call graph representing function call relationships
pub struct CallGraph {
    pub graph: DiGraph<String, ()>, // Nodes are function names
    pub name_to_node: AHashMap<String, NodeIndex>,
}

impl CallGraph {
    /// Build call graph from LLVM IR module
    pub fn from_module(module: &Module) -> Self {
        let mut graph = DiGraph::new();
        let mut name_to_node = AHashMap::new();

        // Create nodes for all functions
        for function in &module.functions {
            let name = function.name.to_string();
            let node = graph.add_node(name.clone());
            name_to_node.insert(name, node);
        }

        // Add edges for function calls
        for function in &module.functions {
            let caller_name = function.name.to_string();
            let caller_node = name_to_node[&caller_name];

            // Scan all basic blocks for call instructions
            for bb in &function.basic_blocks {
                for instr in &bb.instrs {
                    if let Instruction::Call(call) = instr {
                        // Extract callee name
                        let callee_name = Self::extract_callee_name(call);

                        // Add edge if callee exists in module
                        if let Some(&callee_node) = name_to_node.get(&callee_name) {
                            graph.add_edge(caller_node, callee_node, ());
                        }
                    }
                }
            }
        }

        CallGraph {
            graph,
            name_to_node,
        }
    }

    /// Extract callee function name from call instruction
    fn extract_callee_name(call: &llvm_ir::instruction::Call) -> String {
        // Extract function name from operand
        format!("{:?}", call.function)
            .split_whitespace()
            .last()
            .unwrap_or("unknown")
            .trim_matches(|c| c == '@' || c == '"')
            .to_string()
    }

    /// Get callees of a function
    pub fn callees(&self, function_name: &str) -> Vec<String> {
        if let Some(&node) = self.name_to_node.get(function_name) {
            self.graph
                .neighbors(node)
                .map(|n| self.graph[n].clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get callers of a function
    pub fn callers(&self, function_name: &str) -> Vec<String> {
        if let Some(&node) = self.name_to_node.get(function_name) {
            self.graph
                .neighbors_directed(node, petgraph::Direction::Incoming)
                .map(|n| self.graph[n].clone())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Check if call graph has cycles (recursion)
    pub fn has_cycles(&self) -> bool {
        petgraph::algo::is_cyclic_directed(&self.graph)
    }

    /// Get function count
    pub fn function_count(&self) -> usize {
        self.graph.node_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::parser::IRParser;

    #[test]
    fn test_callgraph_construction() {
        let sample_path = "data/armv7e-m/56e3741adeae4068.ll";
        if std::path::Path::new(sample_path).exists() {
            let module = IRParser::parse_file(sample_path).unwrap();
            let callgraph = CallGraph::from_module(&module);

            assert!(
                callgraph.function_count() > 0,
                "Call graph should have functions"
            );
        }
    }
}
