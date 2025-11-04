//! Segment extraction using inkwell
//!
//! Extracts actor segments from inkwell FunctionValue for LLVM 19+ compatibility

use ahash::AHashMap;
use inkwell::values::FunctionValue;
use serde::{Deserialize, Serialize};

use super::inkwell_detector::AsyncFunctionInfo;
use crate::ir::InkwellCFG;

/// Actor segment type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SegmentType {
    Initial,
    Suspended,
    Completion,
}

/// Actor execution segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSegment {
    pub segment_id: u32,
    pub entry_block: String,
    pub blocks: Vec<String>,
    pub exit_blocks: Vec<String>,
    pub next_segments: Vec<u32>,
    pub segment_type: SegmentType,
}

/// Segment extractor for inkwell-based analysis
pub struct InkwellSegmentExtractor;

impl InkwellSegmentExtractor {
    /// Extract segments from inkwell function
    pub fn extract_segments(
        function: &FunctionValue,
        async_info: &AsyncFunctionInfo,
    ) -> Vec<ActorSegment> {
        // Build CFG
        let cfg = InkwellCFG::from_function(function);

        let mut segments = Vec::new();

        // If we have state blocks from async detection, use them
        if !async_info.state_blocks.is_empty() {
            segments = Self::extract_from_state_blocks(&cfg, async_info);
        } else {
            // Fallback: create a single segment for the entire function
            segments = Self::extract_single_segment(&cfg);
        }

        segments
    }

    /// Extract segments based on state blocks from async detection
    fn extract_from_state_blocks(
        cfg: &InkwellCFG,
        async_info: &AsyncFunctionInfo,
    ) -> Vec<ActorSegment> {
        let mut segments = Vec::new();

        for state_block in &async_info.state_blocks {
            // Find the basic block corresponding to this state
            let entry_block_id = cfg
                .block_map
                .get(&state_block.entry_block)
                .copied()
                .unwrap_or(0);

            // Determine segment type based on state ID
            let segment_type = match state_block.state_id {
                0 => SegmentType::Initial,
                1 => SegmentType::Completion,
                _ => SegmentType::Suspended,
            };

            // Collect reachable blocks from this state
            let reachable = Self::collect_reachable_blocks(cfg, entry_block_id, &segments);

            let blocks: Vec<String> = reachable
                .iter()
                .map(|&id| cfg.blocks[id].name.clone())
                .collect();

            // Detect exit blocks (blocks that transition to other segments or return)
            let exit_blocks = Self::detect_exit_blocks(cfg, &reachable, &async_info.state_blocks);

            // Detect next segments (segments reachable from this one)
            let next_segments =
                Self::detect_next_segments(cfg, &reachable, &async_info.state_blocks);

            segments.push(ActorSegment {
                segment_id: state_block.state_id,
                entry_block: state_block.entry_block.clone(),
                blocks: blocks.clone(),
                exit_blocks,
                next_segments,
                segment_type,
            });
        }

        segments
    }

    /// Extract a single segment for the entire function (fallback)
    fn extract_single_segment(cfg: &InkwellCFG) -> Vec<ActorSegment> {
        let reachable = cfg.reachable_blocks();

        let blocks: Vec<String> = reachable
            .iter()
            .map(|&id| cfg.blocks[id].name.clone())
            .collect();

        vec![ActorSegment {
            segment_id: 0,
            entry_block: cfg.blocks[cfg.entry_block].name.clone(),
            blocks: blocks.clone(),
            exit_blocks: vec![],
            next_segments: vec![],
            segment_type: SegmentType::Initial,
        }]
    }

    /// Collect reachable blocks from entry, stopping at other segment entries
    fn collect_reachable_blocks(
        cfg: &InkwellCFG,
        entry_id: usize,
        existing_segments: &[ActorSegment],
    ) -> Vec<usize> {
        use std::collections::{HashSet, VecDeque};

        let mut reachable = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Get entry blocks of existing segments
        let segment_entries: HashSet<_> = existing_segments
            .iter()
            .filter_map(|s| cfg.block_map.get(&s.entry_block).copied())
            .collect();

        queue.push_back(entry_id);
        visited.insert(entry_id);

        while let Some(current) = queue.pop_front() {
            reachable.push(current);

            for &succ in &cfg.successors(current) {
                // Don't cross into other segments
                if segment_entries.contains(&succ) && succ != entry_id {
                    continue;
                }

                if !visited.contains(&succ) {
                    visited.insert(succ);
                    queue.push_back(succ);
                }
            }
        }

        reachable
    }

    /// Detect exit blocks - blocks that transition to other segments or return
    fn detect_exit_blocks(
        cfg: &InkwellCFG,
        reachable: &[usize],
        state_blocks: &[super::inkwell_detector::StateBlock],
    ) -> Vec<String> {
        use std::collections::HashSet;

        let mut exit_blocks = Vec::new();

        // Build set of all segment entry blocks
        let segment_entries: HashSet<_> = state_blocks
            .iter()
            .filter_map(|s| cfg.block_map.get(&s.entry_block).copied())
            .collect();

        for &block_id in reachable {
            let successors = cfg.successors(block_id);

            // Exit block if:
            // 1. Has no successors (returns)
            // 2. Has successor that's an entry to another segment
            let is_exit = successors.is_empty()
                || successors
                    .iter()
                    .any(|&succ| segment_entries.contains(&succ) && !reachable.contains(&succ));

            if is_exit {
                exit_blocks.push(cfg.blocks[block_id].name.clone());
            }
        }

        exit_blocks
    }

    /// Detect next segments - segments reachable from this segment's exit blocks
    fn detect_next_segments(
        cfg: &InkwellCFG,
        reachable: &[usize],
        state_blocks: &[super::inkwell_detector::StateBlock],
    ) -> Vec<u32> {
        use std::collections::HashSet;

        let mut next_segments = HashSet::new();

        // Build map of entry block -> segment ID
        let entry_to_segment: AHashMap<_, _> = state_blocks
            .iter()
            .filter_map(|s| {
                cfg.block_map
                    .get(&s.entry_block)
                    .map(|&id| (id, s.state_id))
            })
            .collect();

        // Check successors of all blocks in this segment
        for &block_id in reachable {
            for &succ in &cfg.successors(block_id) {
                // If successor is entry to another segment, add it
                if let Some(&segment_id) = entry_to_segment.get(&succ) {
                    if !reachable.contains(&succ) {
                        next_segments.insert(segment_id);
                    }
                }
            }
        }

        next_segments.into_iter().collect()
    }

    /// Extract segments with detailed control flow analysis
    pub fn extract_with_analysis<'ctx>(
        function: &FunctionValue<'ctx>,
        async_info: &AsyncFunctionInfo,
    ) -> (Vec<ActorSegment>, InkwellCFG<'ctx>) {
        let cfg = InkwellCFG::from_function(function);
        let segments = if !async_info.state_blocks.is_empty() {
            Self::extract_from_state_blocks(&cfg, async_info)
        } else {
            Self::extract_single_segment(&cfg)
        };

        (segments, cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inkwell_segment_extractor_exists() {
        // Basic compilation test
        assert!(true);
    }
}
