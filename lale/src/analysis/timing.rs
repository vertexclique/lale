use serde::{Deserialize, Serialize};

/// Instruction timing in cycles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cycles {
    pub best_case: u32,
    pub worst_case: u32,
}

impl Cycles {
    pub fn new(cycles: u32) -> Self {
        Self {
            best_case: cycles,
            worst_case: cycles,
        }
    }

    pub fn range(best: u32, worst: u32) -> Self {
        Self {
            best_case: best,
            worst_case: worst,
        }
    }
}

/// Memory access type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessType {
    Ram,
    Flash,
    Peripheral,
    Stack,
}

/// Atomic operation type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AtomicOp {
    Load,
    Store,
    Exchange,
    CompareExchange,
    Add,
}

/// Instruction classification for timing
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum InstructionClass {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Rem,

    // Floating point
    FAdd,
    FSub,
    FMul,
    FDiv,

    // Logic
    And,
    Or,
    Xor,
    Shl,
    Shr,

    // Memory
    Load(AccessType),
    Store(AccessType),

    // Control flow
    Branch,
    Call,
    Ret,

    // Special
    Atomic(AtomicOp),
    Intrinsic(String),

    // Default
    Other,
}

/// Classify LLVM IR instruction
pub fn classify_instruction(instr: &llvm_ir::Instruction) -> InstructionClass {
    use llvm_ir::Instruction::*;

    match instr {
        Add(_) => InstructionClass::Add,
        Sub(_) => InstructionClass::Sub,
        Mul(_) => InstructionClass::Mul,
        UDiv(_) | SDiv(_) => InstructionClass::Div,
        URem(_) | SRem(_) => InstructionClass::Rem,

        FAdd(_) => InstructionClass::FAdd,
        FSub(_) => InstructionClass::FSub,
        FMul(_) => InstructionClass::FMul,
        FDiv(_) => InstructionClass::FDiv,

        And(_) => InstructionClass::And,
        Or(_) => InstructionClass::Or,
        Xor(_) => InstructionClass::Xor,
        Shl(_) => InstructionClass::Shl,
        LShr(_) | AShr(_) => InstructionClass::Shr,

        Load(_) => InstructionClass::Load(AccessType::Ram), // Default to RAM
        Store(_) => InstructionClass::Store(AccessType::Ram),

        Call(call) => {
            // Check if intrinsic
            let func_name = format!("{:?}", call.function);
            if func_name.contains("llvm.") {
                InstructionClass::Intrinsic(func_name)
            } else {
                InstructionClass::Call
            }
        }

        AtomicRMW(rmw) => {
            use llvm_ir::instruction::RMWBinOp::*;
            let op = match rmw.operation {
                Add => AtomicOp::Add,
                Xchg => AtomicOp::Exchange,
                _ => AtomicOp::Add, // Default
            };
            InstructionClass::Atomic(op)
        }

        _ => InstructionClass::Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycles_creation() {
        let c1 = Cycles::new(5);
        assert_eq!(c1.best_case, 5);
        assert_eq!(c1.worst_case, 5);

        let c2 = Cycles::range(1, 10);
        assert_eq!(c2.best_case, 1);
        assert_eq!(c2.worst_case, 10);
    }
}
