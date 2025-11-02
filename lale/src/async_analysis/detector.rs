//! Async function detection using LLVM IR pattern matching
//!
//! Detects Rust async functions by analyzing LLVM IR for characteristic patterns:
//! 1. Generator type names in function metadata
//! 2. Discriminant switch patterns (state machine dispatch)
//! 3. Async function signatures

use llvm_ir::{BasicBlock, Function, Instruction, Terminator};
use serde::{Deserialize, Serialize};

/// Async function detector using LLVM IR pattern matching
pub struct AsyncDetector;

/// Information about detected async function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncFunctionInfo {
    /// Function name
    pub function_name: String,

    /// Whether function is async
    pub is_async: bool,

    /// Confidence score (0-10)
    pub confidence_score: u8,

    /// Pointer to state discriminant field
    pub state_discriminant_ptr: Option<String>,

    /// Detected state blocks
    pub state_blocks: Vec<StateBlock>,

    /// Detection method used
    pub detection_method: DetectionMethod,
}

/// State block in async state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateBlock {
    /// State ID (discriminant value)
    pub state_id: u32,

    /// Entry basic block name
    pub entry_block: String,

    /// All reachable blocks in this state
    pub reachable_blocks: Vec<String>,
}

/// Detection method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionMethod {
    /// Detected via generator type names
    GeneratorType,

    /// Detected via discriminant switch pattern
    DiscriminantSwitch,

    /// Detected via function signature
    AsyncSignature,

    /// Multiple detection methods
    Combined(Vec<DetectionMethod>),
}

impl AsyncDetector {
    /// Detect if function is Rust async
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use lale::async_analysis::AsyncDetector;
    ///
    /// let info = AsyncDetector::detect(&function);
    /// if info.is_async {
    ///     println!("Found async function with {} states", info.state_blocks.len());
    /// }
    /// ```
    pub fn detect(function: &Function) -> AsyncFunctionInfo {
        let mut confidence = 0u8;
        let mut methods = Vec::new();
        let func_name = function.name.to_string();

        // Pattern 1: Generator type detection
        if Self::has_generator_type(function) {
            confidence += 4;
            methods.push(DetectionMethod::GeneratorType);
        }

        // Pattern 2: Discriminant switch detection (strongest indicator)
        if let Some((disc_ptr, states)) = Self::find_discriminant_switch(function) {
            confidence += 5;
            methods.push(DetectionMethod::DiscriminantSwitch);

            return AsyncFunctionInfo {
                function_name: func_name,
                is_async: true,
                confidence_score: confidence,
                state_discriminant_ptr: Some(disc_ptr),
                state_blocks: states,
                detection_method: if methods.len() > 1 {
                    DetectionMethod::Combined(methods)
                } else {
                    DetectionMethod::DiscriminantSwitch
                },
            };
        }

        // Pattern 3: Async signature
        if Self::has_async_signature(function) {
            confidence += 2;
            methods.push(DetectionMethod::AsyncSignature);
        }

        AsyncFunctionInfo {
            function_name: func_name,
            is_async: confidence >= 6,
            confidence_score: confidence,
            state_discriminant_ptr: None,
            state_blocks: vec![],
            detection_method: if methods.len() > 1 {
                DetectionMethod::Combined(methods)
            } else if !methods.is_empty() {
                methods[0].clone()
            } else {
                DetectionMethod::AsyncSignature
            },
        }
    }

    /// Detect all async functions in module
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use lale::async_analysis::AsyncDetector;
    ///
    /// let async_funcs = AsyncDetector::detect_all(&module);
    /// println!("Found {} async functions", async_funcs.len());
    /// ```
    pub fn detect_all(module: &llvm_ir::Module) -> Vec<AsyncFunctionInfo> {
        module
            .functions
            .iter()
            .map(|f| Self::detect(f))
            .filter(|info| info.is_async)
            .collect()
    }

    /// Check if function references generator types
    fn has_generator_type(function: &Function) -> bool {
        let func_str = format!("{:?}", function);
        func_str.contains("generator@")
            || func_str.contains("{{closure}}")
            || func_str.contains("async_fn")
            || func_str.contains("{async_fn_env")
    }

    /// Find discriminant switch pattern in entry block
    fn find_discriminant_switch(function: &Function) -> Option<(String, Vec<StateBlock>)> {
        let entry_bb = function.basic_blocks.first()?;

        // Look for load → switch pattern
        let mut disc_load = None;
        for instr in &entry_bb.instrs {
            if let Instruction::Load(load) = instr {
                // Check if load is of i8 (state discriminant type)
                let load_type = format!("{:?}", load.dest);
                if load_type.contains("i8") {
                    disc_load = Some(load);
                    break;
                }
            }
        }

        let load = disc_load?;

        // Check terminator for switch on loaded value
        if let Terminator::Switch(switch) = &entry_bb.term {
            // Verify switch operand matches loaded value
            let switch_operand = format!("{:?}", switch.operand);
            let load_dest = format!("{:?}", load.dest);

            if !switch_operand.contains(&load_dest) {
                return None;
            }

            let states = Self::extract_state_blocks(switch, function);

            // Validate state machine pattern:
            // - At least 3 states (Unresumed=0, Returned=1, Suspend0=3)
            // - Has state 0 (Unresumed)
            // - Has state >= 3 (Suspend states)
            if states.len() >= 3 {
                let has_unresumed = states.iter().any(|s| s.state_id == 0);
                let has_suspend = states.iter().any(|s| s.state_id >= 3);

                if has_unresumed && has_suspend {
                    let disc_ptr = format!("{:?}", load.address);
                    return Some((disc_ptr, states));
                }
            }
        }

        None
    }

    /// Extract state blocks from switch instruction
    fn extract_state_blocks(
        switch: &llvm_ir::terminator::Switch,
        _function: &Function,
    ) -> Vec<StateBlock> {
        let mut states = Vec::new();

        // Extract each switch case as a state
        for (idx, (value, dest)) in switch.dests.iter().enumerate() {
            let state_id = match value.as_ref() {
                llvm_ir::Constant::Int { value, .. } => *value as u32,
                _ => idx as u32,
            };

            let entry_block = dest.to_string();

            states.push(StateBlock {
                state_id,
                entry_block: entry_block.clone(),
                reachable_blocks: vec![entry_block],
            });
        }

        // Sort by state ID
        states.sort_by_key(|s| s.state_id);
        states
    }

    /// Check for async function signature pattern
    fn has_async_signature(function: &Function) -> bool {
        // Async functions typically have signature:
        // (ptr, ptr) -> i1  (generator, context) -> bool
        let params = &function.parameters;
        let has_two_ptr_params = params.len() == 2;
        let returns_bool = function.return_type.to_string().contains("i1");

        has_two_ptr_params && returns_bool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_async_detector_exists() {
        // Placeholder test - will be expanded with real LLVM IR fixtures
        assert!(true);
    }
}
