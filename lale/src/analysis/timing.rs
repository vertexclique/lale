//! Instruction timing - Minimal stub for compatibility
//!
//! This provides basic types for timing analysis.
//! For new code, use InkwellTimingCalculator instead.

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
