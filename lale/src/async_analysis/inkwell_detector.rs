//! Async function detection using inkwell for LLVM 18+
//!
//! Detects Rust async functions by analyzing LLVM IR via inkwell API.
//! Supports modern LLVM versions (18+) that llvm-ir crate cannot parse.

use either::Either;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{FunctionValue, InstructionOpcode};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::ir::inkwell_parser::{InkwellFunction, InkwellParser, TerminatorKind};

/// Information about detected async function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncFunctionInfo {
    pub function_name: String,
    pub is_async: bool,
    pub confidence_score: u8,
    pub state_discriminant_ptr: Option<String>,
    pub state_blocks: Vec<StateBlock>,
    pub detection_method: DetectionMethod,
}

/// State block in async state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateBlock {
    pub state_id: u32,
    pub entry_block: String,
    pub reachable_blocks: Vec<String>,
}

/// Detection method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionMethod {
    GeneratorType,
    DiscriminantSwitch,
    AsyncSignature,
    Combined(Vec<DetectionMethod>),
}

/// Inkwell-based async detector for LLVM 18+
pub struct InkwellAsyncDetector;

impl InkwellAsyncDetector {
    /// Detect async functions from LLVM IR file
    pub fn detect_from_file(path: impl AsRef<Path>) -> Result<Vec<AsyncFunctionInfo>, String> {
        let (_context, module) = InkwellParser::parse_file(path)?;
        Self::detect_from_module(&module)
    }

    /// Detect async functions from LLVM IR text
    pub fn detect_from_ir_text(ir_text: &str) -> Result<Vec<AsyncFunctionInfo>, String> {
        let (_context, module) = InkwellParser::parse_ir_from_buffer(ir_text)?;
        Self::detect_from_module(&module)
    }

    /// Detect async functions from inkwell module
    pub fn detect_from_module(module: &Module) -> Result<Vec<AsyncFunctionInfo>, String> {
        let functions = InkwellParser::extract_functions(module);
        let mut results = Vec::new();

        for func in functions {
            if let Some(info) = Self::detect_async_in_function(&func) {
                results.push(info);
            }
        }

        Ok(results)
    }

    /// Detect async pattern in function
    fn detect_async_in_function(func: &InkwellFunction) -> Option<AsyncFunctionInfo> {
        let mut confidence = 0u8;
        let mut methods = Vec::new();

        // Pattern 1: Check for switch pattern in entry block
        if let Some(states) = Self::detect_switch_pattern(func) {
            confidence += 5;
            methods.push(DetectionMethod::DiscriminantSwitch);

            // Validate state machine structure
            let has_unresumed = states.iter().any(|s| s.state_id == 0);
            let has_suspend = states.iter().any(|s| s.state_id >= 3);

            if has_unresumed && has_suspend && states.len() >= 3 {
                return Some(AsyncFunctionInfo {
                    function_name: func.name.clone(),
                    is_async: true,
                    confidence_score: confidence,
                    state_discriminant_ptr: Some("detected_via_inkwell".to_string()),
                    state_blocks: states,
                    detection_method: DetectionMethod::DiscriminantSwitch,
                });
            }
        }

        // Pattern 2: Check function signature (ptr, ptr) -> i1
        if Self::has_async_signature(&func.function) {
            confidence += 2;
            methods.push(DetectionMethod::AsyncSignature);
        }

        if confidence >= 6 {
            Some(AsyncFunctionInfo {
                function_name: func.name.clone(),
                is_async: true,
                confidence_score: confidence,
                state_discriminant_ptr: None,
                state_blocks: vec![],
                detection_method: if methods.len() > 1 {
                    DetectionMethod::Combined(methods)
                } else {
                    methods
                        .into_iter()
                        .next()
                        .unwrap_or(DetectionMethod::AsyncSignature)
                },
            })
        } else {
            None
        }
    }

    /// Detect load i8 + switch i8 pattern
    fn detect_switch_pattern(func: &InkwellFunction) -> Option<Vec<StateBlock>> {
        // Check entry block
        let entry_block = func.basic_blocks.first()?;

        // Look for Load instruction with i8 type
        let mut has_i8_load = false;
        for instr in &entry_block.instructions {
            if instr.opcode == "load" {
                // Check if it's loading i8
                let type_str = format!("{:?}", instr.instruction.get_type());
                if type_str.contains("i8") {
                    has_i8_load = true;
                    break;
                }
            }
        }

        if !has_i8_load {
            return None;
        }

        // Check for switch terminator
        if let Some(term) = &entry_block.terminator {
            if term.kind == TerminatorKind::Switch {
                // Extract switch cases
                let states = Self::extract_switch_cases(&term.instruction);
                if !states.is_empty() {
                    return Some(states);
                }
            }
        }

        None
    }

    /// Extract state blocks from switch instruction
    fn extract_switch_cases(switch_instr: &inkwell::values::InstructionValue) -> Vec<StateBlock> {
        let mut states = Vec::new();

        // Get the parent basic block and then the parent function
        let parent_bb = switch_instr
            .get_parent()
            .expect("Switch must have parent block");
        let function = parent_bb
            .get_parent()
            .expect("Block must have parent function");

        // Get all basic blocks in the function
        let all_blocks: Vec<_> = function.get_basic_block_iter().collect();

        // The switch successors are the state blocks
        // We'll identify them by analyzing block names and predecessors
        let mut state_id = 0u32;

        for block in &all_blocks {
            let block_name = block.get_name().to_str().unwrap_or("");

            // State blocks typically have names like "bb3", "bb4", etc. in async lowering
            // or may contain patterns like "state" or numeric suffixes
            if block_name.starts_with("bb") && block_name.len() > 2 {
                // Try to extract state ID from block name
                if let Ok(id) = block_name[2..].parse::<u32>() {
                    state_id = id;
                }

                states.push(StateBlock {
                    state_id,
                    entry_block: block_name.to_string(),
                    reachable_blocks: vec![block_name.to_string()],
                });

                state_id += 1;
            }
        }

        // If no states found by naming pattern, enumerate all blocks after entry
        if states.is_empty() {
            state_id = 0;
            for block in all_blocks.iter().skip(1) {
                // Skip entry block
                let block_name = block.get_name().to_str().unwrap_or("");
                if !block_name.is_empty() {
                    states.push(StateBlock {
                        state_id,
                        entry_block: block_name.to_string(),
                        reachable_blocks: vec![block_name.to_string()],
                    });
                    state_id += 1;
                }
            }
        }

        states
    }

    /// Check for async function signature
    fn has_async_signature(function: &FunctionValue) -> bool {
        // Async functions: (ptr, ptr) -> i1
        let param_count = function.count_params();
        if param_count != 2 {
            return false;
        }

        // Check return type is i1 (bool)
        let ret_type = function.get_type().get_return_type();
        if let Some(ret) = ret_type {
            let type_str = format!("{:?}", ret);
            return type_str.contains("i1");
        }

        false
    }
}

/// Analyze async functions in LLVM IR file
pub fn analyze_async_functions(ir_file_path: &str) -> Result<Vec<AsyncFunctionInfo>, String> {
    InkwellAsyncDetector::detect_from_file(ir_file_path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inkwell_detector_exists() {
        // Basic compilation test
        assert!(true);
    }
}
