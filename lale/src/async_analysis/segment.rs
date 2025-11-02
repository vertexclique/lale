//! Execution segment extraction for async functions
//!
//! Extracts code segments between await points in async functions.
//! Each segment represents a run-to-completion execution unit.

use crate::async_analysis::detector::{AsyncFunctionInfo, StateBlock};
use ahash::{AHashMap, AHashSet};
use llvm_ir::{BasicBlock, Function, Instruction, Terminator};
use serde::{Deserialize, Serialize};

/// Actor execution segment (code between await points)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSegment {
    /// Segment ID (corresponds to state)
    pub segment_id: u32,

    /// Entry basic block
    pub entry_block: String,

    /// All basic blocks in segment
    pub blocks: Vec<String>,

    /// Exit blocks (update state or return)
    pub exit_blocks: Vec<String>,

    /// Next possible segments
    pub next_segments: Vec<u32>,

    /// Segment type
    pub segment_type: SegmentType,
}

/// Type of segment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SegmentType {
    /// Initial segment (state 0)
    Initial,

    /// Resume after await
    Suspended,

    /// Final segment (returns)
    Completion,
}

/// Segment extractor
pub struct SegmentExtractor;

impl SegmentExtractor {
    /// Extract execution segments from async function
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use lale::async_analysis::{AsyncDetector, SegmentExtractor};
    ///
    /// let async_info = AsyncDetector::detect(&function);
    /// let segments = SegmentExtractor::extract_segments(&function, &async_info);
    /// ```
    pub fn extract_segments(
        function: &Function,
        async_info: &AsyncFunctionInfo,
    ) -> Vec<ActorSegment> {
        if !async_info.is_async || async_info.state_blocks.is_empty() {
            return vec![];
        }

        let mut segments = Vec::new();

        for state_block in &async_info.state_blocks {
            let segment_type = match state_block.state_id {
                0 => SegmentType::Initial,
                1 => SegmentType::Completion,
                _ => SegmentType::Suspended,
            };

            let segment = Self::build_segment(
                function,
                state_block,
                &async_info.state_discriminant_ptr,
                segment_type,
            );

            segments.push(segment);
        }

        segments
    }

    /// Build segment by exploring reachable blocks
    fn build_segment(
        function: &Function,
        state_block: &StateBlock,
        disc_ptr: &Option<String>,
        segment_type: SegmentType,
    ) -> ActorSegment {
        let mut visited = AHashSet::new();
        let mut blocks = Vec::new();
        let mut exit_blocks = Vec::new();
        let mut next_segments = Vec::new();

        // BFS from entry block
        let mut queue = vec![state_block.entry_block.clone()];

        while let Some(bb_name) = queue.pop() {
            if visited.contains(&bb_name) {
                continue;
            }
            visited.insert(bb_name.clone());
            blocks.push(bb_name.clone());

            // Find basic block in function
            let bb = match Self::find_block(function, &bb_name) {
                Some(b) => b,
                None => continue,
            };

            // Check if this block updates state (segment boundary)
            if Self::updates_state(bb, disc_ptr) {
                exit_blocks.push(bb_name.clone());

                // Extract next state value
                if let Some(next_state) = Self::extract_next_state(bb) {
                    next_segments.push(next_state);
                }
                continue;
            }

            // Check if this block returns (segment end)
            if Self::is_return_block(bb) {
                exit_blocks.push(bb_name);
                continue;
            }

            // Add successors to queue
            for succ in Self::get_successors(bb) {
                if !visited.contains(&succ) {
                    queue.push(succ);
                }
            }
        }

        ActorSegment {
            segment_id: state_block.state_id,
            entry_block: state_block.entry_block.clone(),
            blocks,
            exit_blocks,
            next_segments,
            segment_type,
        }
    }

    /// Find basic block by name
    fn find_block<'a>(function: &'a Function, name: &str) -> Option<&'a BasicBlock> {
        function.basic_blocks.iter().find(|bb| match &bb.name {
            llvm_ir::Name::Name(n) => n.as_str() == name,
            llvm_ir::Name::Number(_) => false,
        })
    }

    /// Check if block updates state discriminant
    fn updates_state(bb: &BasicBlock, disc_ptr: &Option<String>) -> bool {
        if let Some(ptr) = disc_ptr {
            for instr in &bb.instrs {
                if let Instruction::Store(store) = instr {
                    let dest = format!("{:?}", store.address);
                    if dest.contains(ptr) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Extract next state value from store instruction
    fn extract_next_state(bb: &BasicBlock) -> Option<u32> {
        for instr in &bb.instrs {
            if let Instruction::Store(store) = instr {
                if let llvm_ir::Operand::ConstantOperand(c) = &store.value {
                    if let llvm_ir::Constant::Int { value, .. } = c.as_ref() {
                        return Some(*value as u32);
                    }
                }
            }
        }
        None
    }

    /// Check if block is a return block
    fn is_return_block(bb: &BasicBlock) -> bool {
        matches!(bb.term, Terminator::Ret(_))
    }

    /// Get successor blocks
    fn get_successors(bb: &BasicBlock) -> Vec<String> {
        match &bb.term {
            Terminator::Br(br) => vec![br.dest.to_string()],
            Terminator::CondBr(cbr) => vec![cbr.true_dest.to_string(), cbr.false_dest.to_string()],
            Terminator::Switch(sw) => sw.dests.iter().map(|(_, dest)| dest.to_string()).collect(),
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_extractor_exists() {
        // Placeholder test
        assert!(true);
    }
}
