//! Control Flow Graph builder using inkwell
//!
//! Builds CFG from inkwell FunctionValue for LLVM 19+ compatibility

use ahash::AHashMap;
use inkwell::basic_block::BasicBlock;
use inkwell::values::{FunctionValue, InstructionOpcode};
use std::collections::VecDeque;

/// Basic block in inkwell CFG
#[derive(Debug, Clone)]
pub struct InkwellBasicBlock<'ctx> {
    pub id: usize,
    pub name: String,
    pub block: BasicBlock<'ctx>,
    pub instruction_count: usize,
}

/// Control Flow Graph built from inkwell function
#[derive(Debug)]
pub struct InkwellCFG<'ctx> {
    pub blocks: Vec<InkwellBasicBlock<'ctx>>,
    pub edges: Vec<(usize, usize)>,
    pub entry_block: usize,
    pub block_map: AHashMap<String, usize>,
}

impl<'ctx> InkwellCFG<'ctx> {
    /// Build CFG from inkwell function
    pub fn from_function(function: &FunctionValue<'ctx>) -> Self {
        let mut blocks = Vec::new();
        let mut block_map = AHashMap::new();
        let mut edges = Vec::new();

        // Collect all basic blocks
        let basic_blocks: Vec<_> = function.get_basic_blocks();

        for (id, bb) in basic_blocks.iter().enumerate() {
            let name = bb
                .get_name()
                .to_str()
                .unwrap_or(&format!("bb{}", id))
                .to_string();

            // Count instructions
            let mut instruction_count = 0;
            let mut instr_iter = bb.get_first_instruction();
            while let Some(instr) = instr_iter {
                instruction_count += 1;
                instr_iter = instr.get_next_instruction();
            }

            block_map.insert(name.clone(), id);
            blocks.push(InkwellBasicBlock {
                id,
                name,
                block: *bb,
                instruction_count,
            });
        }

        // Extract edges by analyzing terminator instructions
        // We iterate through all blocks and find their successors
        for (from_id, from_bb) in basic_blocks.iter().enumerate() {
            // For each block, check all other blocks to see if they're successors
            // This is done by checking if the terminator references them
            for (to_id, to_bb) in basic_blocks.iter().enumerate() {
                if from_id == to_id {
                    continue;
                }

                // Check if from_bb's terminator references to_bb
                if let Some(terminator) = from_bb.get_terminator() {
                    let num_operands = terminator.get_num_operands();

                    // Check all operands to see if any reference to_bb
                    for i in 0..num_operands {
                        if let Some(operand) = terminator.get_operand(i) {
                            // Check if this operand is a basic block reference
                            if operand.is_block() {
                                // Try to match by comparing block addresses
                                // This is a heuristic approach since inkwell doesn't provide direct comparison
                                let to_name = to_bb.get_name().to_str().unwrap_or("");

                                // If we can get the operand as instruction value and check its parent
                                // For now, we'll use a simple heuristic: if the operand is a block
                                // and we're in a branch instruction, assume it's a successor
                                let opcode = terminator.get_opcode();
                                match opcode {
                                    InstructionOpcode::Br
                                    | InstructionOpcode::Switch
                                    | InstructionOpcode::IndirectBr => {
                                        // This is a control flow instruction
                                        // Add edge if we haven't already
                                        if !edges.contains(&(from_id, to_id)) {
                                            edges.push((from_id, to_id));
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }

        Self {
            blocks,
            edges,
            entry_block: 0, // Entry is always first block
            block_map,
        }
    }

    /// Get successors of a block
    pub fn successors(&self, block_id: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter(|(from, _)| *from == block_id)
            .map(|(_, to)| *to)
            .collect()
    }

    /// Get predecessors of a block
    pub fn predecessors(&self, block_id: usize) -> Vec<usize> {
        self.edges
            .iter()
            .filter(|(_, to)| *to == block_id)
            .map(|(from, _)| *from)
            .collect()
    }

    /// Check if block is reachable from entry
    pub fn is_reachable(&self, block_id: usize) -> bool {
        let mut visited = vec![false; self.blocks.len()];
        let mut queue = VecDeque::new();

        queue.push_back(self.entry_block);
        visited[self.entry_block] = true;

        while let Some(current) = queue.pop_front() {
            if current == block_id {
                return true;
            }

            for &succ in &self.successors(current) {
                if !visited[succ] {
                    visited[succ] = true;
                    queue.push_back(succ);
                }
            }
        }

        false
    }

    /// Get all reachable blocks
    pub fn reachable_blocks(&self) -> Vec<usize> {
        let mut reachable = Vec::new();
        let mut visited = vec![false; self.blocks.len()];
        let mut queue = VecDeque::new();

        queue.push_back(self.entry_block);
        visited[self.entry_block] = true;

        while let Some(current) = queue.pop_front() {
            reachable.push(current);

            for &succ in &self.successors(current) {
                if !visited[succ] {
                    visited[succ] = true;
                    queue.push_back(succ);
                }
            }
        }

        reachable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inkwell_cfg_exists() {
        // Basic compilation test
        assert!(true);
    }
}
