//! Timing calculator using inkwell
//!
//! Calculates instruction timing from inkwell FunctionValue for LLVM 19+ compatibility

use ahash::AHashMap;
use inkwell::basic_block::BasicBlock;
use inkwell::values::{FunctionValue, InstructionOpcode};

use crate::ir::InkwellCFG;
use crate::platform::PlatformModel;

/// Timing calculator for inkwell-based analysis
pub struct InkwellTimingCalculator;

impl InkwellTimingCalculator {
    /// Calculate timing for all basic blocks in a function
    pub fn calculate_block_timings(
        function: &FunctionValue,
        cfg: &InkwellCFG,
        platform: &PlatformModel,
    ) -> AHashMap<usize, u64> {
        let mut timings = AHashMap::new();

        for block in &cfg.blocks {
            let cycles = Self::calculate_block_timing(&block.block, platform);
            timings.insert(block.id, cycles);
        }

        timings
    }

    /// Calculate timing for a single basic block
    fn calculate_block_timing(block: &BasicBlock, platform: &PlatformModel) -> u64 {
        let mut total_cycles = 0u64;

        // Iterate through instructions
        let mut instr_iter = block.get_first_instruction();
        while let Some(instr) = instr_iter {
            let cycles = Self::instruction_cost(&instr.get_opcode(), platform);
            total_cycles += cycles;
            instr_iter = instr.get_next_instruction();
        }

        total_cycles
    }

    /// Get instruction cost based on opcode and platform
    fn instruction_cost(opcode: &InstructionOpcode, platform: &PlatformModel) -> u64 {
        use crate::analysis::timing::{AccessType, AtomicOp, InstructionClass};
        use InstructionOpcode::*;

        let class = match opcode {
            // Arithmetic operations
            Add | Sub => InstructionClass::Add,
            Mul => InstructionClass::Mul,
            UDiv | SDiv => InstructionClass::Div,
            URem | SRem => InstructionClass::Rem,

            // Floating point
            FAdd => InstructionClass::FAdd,
            FSub => InstructionClass::FSub,
            FMul => InstructionClass::FMul,
            FDiv | FRem => InstructionClass::FDiv,

            // Logic
            And => InstructionClass::And,
            Or => InstructionClass::Or,
            Xor => InstructionClass::Xor,
            Shl => InstructionClass::Shl,
            LShr | AShr => InstructionClass::Shr,

            // Memory operations
            Load => InstructionClass::Load(AccessType::Ram),
            Store => InstructionClass::Store(AccessType::Ram),
            Alloca => InstructionClass::Store(AccessType::Stack),

            // Comparison - treat as Add
            ICmp | FCmp => InstructionClass::Add,

            // Branches
            Br | Switch | IndirectBr => InstructionClass::Branch,

            // Function calls
            Call | Invoke => InstructionClass::Call,

            // Returns
            Return => InstructionClass::Ret,

            // Type conversions - treat as Add
            Trunc | ZExt | SExt | FPToUI | FPToSI | UIToFP | SIToFP | FPTrunc | FPExt
            | PtrToInt | IntToPtr | BitCast | AddrSpaceCast => InstructionClass::Add,

            // Vector operations - treat as Add
            ExtractElement | InsertElement | ShuffleVector => InstructionClass::Add,
            ExtractValue | InsertValue => InstructionClass::Add,

            // Aggregate operations - treat as Add
            GetElementPtr => InstructionClass::Add,

            // Select - treat as Add
            Select => InstructionClass::Add,

            // PHI nodes (no runtime cost)
            Phi => return 0,

            // Atomic operations
            AtomicRMW | AtomicCmpXchg | Fence => {
                return platform
                    .get_timing(&InstructionClass::Atomic(AtomicOp::Add))
                    .worst_case as u64;
            }

            // Landing pad / exception handling
            LandingPad | Resume | CleanupRet | CatchRet | CatchSwitch | CatchPad | CleanupPad => {
                return 10; // Exception handling is expensive
            }

            // Unreachable
            Unreachable => return 0,

            // User operations (inline asm, etc.)
            UserOp1 | UserOp2 => return 5,

            // VA operations - treat as Add
            VAArg => InstructionClass::Add,

            // Freeze (LLVM 10+)
            Freeze => return 0,

            // Default for unknown instructions
            _ => {
                eprintln!(
                    "Warning: Unknown instruction opcode {:?}, using default cost",
                    opcode
                );
                InstructionClass::Other
            }
        };

        platform.get_timing(&class).worst_case as u64
    }

    /// Calculate timing with cache effects
    pub fn calculate_with_cache(
        function: &FunctionValue,
        cfg: &InkwellCFG,
        platform: &PlatformModel,
    ) -> AHashMap<usize, u64> {
        let mut timings = Self::calculate_block_timings(function, cfg, platform);

        // Apply cache miss penalties
        // Simple model: assume cold cache at function entry, warm cache for loops
        let cache_miss_penalty = 10; // Conservative estimate in cycles

        for block in &cfg.blocks {
            // Add cache miss penalty for first block (cold cache)
            if block.id == cfg.entry_block {
                if let Some(timing) = timings.get_mut(&block.id) {
                    *timing += cache_miss_penalty;
                }
            }

            // Add cache miss penalty for blocks with many memory operations
            let memory_ops = Self::count_memory_operations(&block.block);
            if memory_ops > 5 {
                if let Some(timing) = timings.get_mut(&block.id) {
                    // Conservative: assume some cache misses for memory-intensive blocks
                    *timing += (memory_ops / 5) * cache_miss_penalty;
                }
            }
        }

        timings
    }

    /// Count memory operations in a basic block
    fn count_memory_operations(block: &BasicBlock) -> u64 {
        let mut count = 0;

        let mut instr_iter = block.get_first_instruction();
        while let Some(instr) = instr_iter {
            match instr.get_opcode() {
                InstructionOpcode::Load | InstructionOpcode::Store | InstructionOpcode::Alloca => {
                    count += 1;
                }
                _ => {}
            }
            instr_iter = instr.get_next_instruction();
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::CortexM4Model;

    #[test]
    fn test_inkwell_timing_calculator_exists() {
        // Basic compilation test
        assert!(true);
    }

    #[test]
    fn test_instruction_cost_arithmetic() {
        let platform = CortexM4Model::new();

        // Test arithmetic operations
        let add_cost =
            InkwellTimingCalculator::instruction_cost(&InstructionOpcode::Add, &platform);
        assert!(add_cost > 0, "Add instruction should have non-zero cost");

        let mul_cost =
            InkwellTimingCalculator::instruction_cost(&InstructionOpcode::Mul, &platform);
        assert!(
            mul_cost >= add_cost,
            "Multiply should be at least as expensive as add"
        );

        let div_cost =
            InkwellTimingCalculator::instruction_cost(&InstructionOpcode::UDiv, &platform);
        assert!(
            div_cost >= mul_cost,
            "Division should be at least as expensive as multiply"
        );
    }

    #[test]
    fn test_instruction_cost_memory() {
        let platform = CortexM4Model::new();

        // Test memory operations
        let load_cost =
            InkwellTimingCalculator::instruction_cost(&InstructionOpcode::Load, &platform);
        assert!(load_cost > 0, "Load instruction should have non-zero cost");

        let store_cost =
            InkwellTimingCalculator::instruction_cost(&InstructionOpcode::Store, &platform);
        assert!(
            store_cost > 0,
            "Store instruction should have non-zero cost"
        );
    }

    #[test]
    fn test_instruction_cost_control_flow() {
        let platform = CortexM4Model::new();

        // Test control flow operations
        let branch_cost =
            InkwellTimingCalculator::instruction_cost(&InstructionOpcode::Br, &platform);
        assert!(
            branch_cost > 0,
            "Branch instruction should have non-zero cost"
        );

        let call_cost =
            InkwellTimingCalculator::instruction_cost(&InstructionOpcode::Call, &platform);
        assert!(call_cost > 0, "Call instruction should have non-zero cost");

        let ret_cost =
            InkwellTimingCalculator::instruction_cost(&InstructionOpcode::Return, &platform);
        assert!(ret_cost > 0, "Return instruction should have non-zero cost");
    }

    #[test]
    fn test_instruction_cost_phi_node() {
        let platform = CortexM4Model::new();

        // PHI nodes should have zero cost (no runtime overhead)
        let phi_cost =
            InkwellTimingCalculator::instruction_cost(&InstructionOpcode::Phi, &platform);
        assert_eq!(phi_cost, 0, "PHI node should have zero cost");
    }

    #[test]
    fn test_instruction_cost_unreachable() {
        let platform = CortexM4Model::new();

        // Unreachable should have zero cost
        let unreachable_cost =
            InkwellTimingCalculator::instruction_cost(&InstructionOpcode::Unreachable, &platform);
        assert_eq!(unreachable_cost, 0, "Unreachable should have zero cost");
    }
}
