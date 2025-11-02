//! Inkwell-based LLVM IR parser for LLVM 18+
//!
//! Provides parsing of LLVM IR using inkwell (LLVM C API bindings)
//! to support modern LLVM versions (18+) that llvm-ir crate cannot parse.

use inkwell::basic_block::BasicBlock;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::values::{BasicValueEnum, FunctionValue, InstructionValue};
use std::collections::HashMap;
use std::path::Path;

/// Inkwell-based IR parser
pub struct InkwellParser;

/// Function information extracted via inkwell
#[derive(Debug, Clone)]
pub struct InkwellFunction<'ctx> {
    pub name: String,
    pub function: FunctionValue<'ctx>,
    pub basic_blocks: Vec<InkwellBasicBlock<'ctx>>,
}

/// Basic block information
#[derive(Debug, Clone)]
pub struct InkwellBasicBlock<'ctx> {
    pub name: String,
    pub block: BasicBlock<'ctx>,
    pub instructions: Vec<InkwellInstruction<'ctx>>,
    pub terminator: Option<InkwellTerminator<'ctx>>,
}

/// Instruction information
#[derive(Debug, Clone)]
pub struct InkwellInstruction<'ctx> {
    pub opcode: String,
    pub instruction: InstructionValue<'ctx>,
}

/// Terminator instruction
#[derive(Debug, Clone)]
pub struct InkwellTerminator<'ctx> {
    pub kind: TerminatorKind,
    pub instruction: InstructionValue<'ctx>,
}

/// Terminator types
#[derive(Debug, Clone, PartialEq)]
pub enum TerminatorKind {
    Return,
    Branch,
    ConditionalBranch,
    Switch,
    Unreachable,
    Other,
}

impl InkwellParser {
    /// Parse LLVM IR file using inkwell
    /// Returns (Context, Module) - caller must keep context alive
    pub fn parse_file(path: impl AsRef<Path>) -> Result<(Context, Module<'static>), String> {
        let context = Context::create();
        // SAFETY: We return both context and module together
        // Caller must ensure context outlives module usage
        let module = unsafe {
            std::mem::transmute::<Module, Module<'static>>(
                Module::parse_bitcode_from_path(path.as_ref(), &context)
                    .map_err(|e| format!("Failed to parse bitcode: {:?}", e))?,
            )
        };

        Ok((context, module))
    }

    /// Parse LLVM IR from memory buffer
    pub fn parse_ir_from_buffer(ir_text: &str) -> Result<(Context, Module<'static>), String> {
        let context = Context::create();
        let module = unsafe {
            std::mem::transmute::<Module, Module<'static>>(
                context
                    .create_module_from_ir(
                        inkwell::memory_buffer::MemoryBuffer::create_from_memory_range(
                            ir_text.as_bytes(),
                            "ir_module",
                        ),
                    )
                    .map_err(|e| format!("Failed to parse IR: {:?}", e))?,
            )
        };

        Ok((context, module))
    }

    /// Extract function information from module
    pub fn extract_functions<'ctx>(module: &Module<'ctx>) -> Vec<InkwellFunction<'ctx>> {
        let mut functions = Vec::new();

        for function in module.get_functions() {
            let name = function.get_name().to_str().unwrap_or("").to_string();
            let basic_blocks = Self::extract_basic_blocks(&function);

            functions.push(InkwellFunction {
                name,
                function,
                basic_blocks,
            });
        }

        functions
    }

    /// Extract basic blocks from function
    fn extract_basic_blocks<'ctx>(function: &FunctionValue<'ctx>) -> Vec<InkwellBasicBlock<'ctx>> {
        let mut blocks = Vec::new();

        let mut bb = function.get_first_basic_block();
        while let Some(block) = bb {
            let name = block.get_name().to_str().unwrap_or("").to_string();
            let instructions = Self::extract_instructions(&block);
            let terminator = Self::extract_terminator(&block);

            blocks.push(InkwellBasicBlock {
                name,
                block,
                instructions,
                terminator,
            });

            bb = block.get_next_basic_block();
        }

        blocks
    }

    /// Extract instructions from basic block
    fn extract_instructions<'ctx>(block: &BasicBlock<'ctx>) -> Vec<InkwellInstruction<'ctx>> {
        let mut instructions = Vec::new();

        let mut instr = block.get_first_instruction();
        while let Some(instruction) = instr {
            // Check if it's a terminator by checking if it's the last instruction
            let is_terminator = instruction.get_next_instruction().is_none();

            if !is_terminator {
                let opcode = format!("{:?}", instruction.get_opcode());
                instructions.push(InkwellInstruction {
                    opcode,
                    instruction,
                });
            }
            instr = instruction.get_next_instruction();
        }

        instructions
    }

    /// Extract terminator instruction
    fn extract_terminator<'ctx>(block: &BasicBlock<'ctx>) -> Option<InkwellTerminator<'ctx>> {
        let terminator = block.get_terminator()?;

        let kind = match terminator.get_opcode() {
            inkwell::values::InstructionOpcode::Return => TerminatorKind::Return,
            inkwell::values::InstructionOpcode::Br => TerminatorKind::Branch,
            inkwell::values::InstructionOpcode::Switch => TerminatorKind::Switch,
            inkwell::values::InstructionOpcode::Unreachable => TerminatorKind::Unreachable,
            _ => TerminatorKind::Other,
        };

        Some(InkwellTerminator {
            kind,
            instruction: terminator,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inkwell_parser_exists() {
        // Basic test to ensure module compiles
        assert!(true);
    }
}
