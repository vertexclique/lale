//! Async function detection using inkwell for LLVM 18+
//!
//! Detects Rust async functions by analyzing LLVM IR via inkwell API.
//! Supports modern LLVM versions (18+) that llvm-ir crate cannot parse.

use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{FunctionValue, InstructionOpcode};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::detector::{AsyncFunctionInfo, DetectionMethod, StateBlock};
use crate::ir::inkwell_parser::{InkwellFunction, InkwellParser, TerminatorKind};

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

        // Get number of operands (switch has: condition + default + cases)
        let num_operands = switch_instr.get_num_operands();

        // Skip first 2 operands (condition and default label)
        // Then iterate pairs: (case_value, case_label)
        let mut i = 2;
        let mut state_id = 0u32;

        while i < num_operands {
            if let Some(operand) = switch_instr.get_operand(i) {
                // Try to extract constant value
                if let Some(const_val) = operand.left() {
                    if let Some(int_val) = const_val.into_int_value().get_zero_extended_constant() {
                        state_id = int_val as u32;
                    }
                }
            }

            // Get label (next operand)
            if i + 1 < num_operands {
                if let Some(label_op) = switch_instr.get_operand(i + 1) {
                    if let Some(bb) = label_op.right() {
                        let label_name = bb.get_name().to_str().unwrap_or("").to_string();

                        states.push(StateBlock {
                            state_id,
                            entry_block: label_name.clone(),
                            reachable_blocks: vec![label_name],
                        });
                    }
                }
            }

            i += 2; // Move to next case pair
        }

        states.sort_by_key(|s| s.state_id);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inkwell_detector_exists() {
        // Basic compilation test
        assert!(true);
    }
}
