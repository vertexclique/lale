use crate::analysis::timing::{AccessType, AtomicOp, Cycles, InstructionClass};
use crate::platform::PlatformModel;
use ahash::AHashMap;

/// RISC-V RV32I timing model (base integer ISA)
pub struct RV32IModel;

impl RV32IModel {
    /// Create RV32I @ 100MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic (1 cycle)
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::new(32)); // Software multiply (no M extension)
        timings.insert(InstructionClass::Div, Cycles::new(40)); // Software divide
        timings.insert(InstructionClass::Rem, Cycles::new(40));

        // No FPU (no F/D extension)
        timings.insert(InstructionClass::FAdd, Cycles::new(100));
        timings.insert(InstructionClass::FSub, Cycles::new(100));
        timings.insert(InstructionClass::FMul, Cycles::new(100));
        timings.insert(InstructionClass::FDiv, Cycles::new(150));

        // Logic
        timings.insert(InstructionClass::And, Cycles::new(1));
        timings.insert(InstructionClass::Or, Cycles::new(1));
        timings.insert(InstructionClass::Xor, Cycles::new(1));
        timings.insert(InstructionClass::Shl, Cycles::new(1));
        timings.insert(InstructionClass::Shr, Cycles::new(1));

        // Memory access
        timings.insert(InstructionClass::Load(AccessType::Ram), Cycles::new(2));
        timings.insert(InstructionClass::Store(AccessType::Ram), Cycles::new(2));
        timings.insert(InstructionClass::Load(AccessType::Flash), Cycles::new(3));
        timings.insert(InstructionClass::Store(AccessType::Flash), Cycles::new(3));

        // Control flow
        timings.insert(InstructionClass::Branch, Cycles::range(1, 3));
        timings.insert(InstructionClass::Call, Cycles::range(2, 4));
        timings.insert(InstructionClass::Ret, Cycles::range(2, 4));

        // Atomics (no A extension)
        timings.insert(InstructionClass::Atomic(AtomicOp::Load), Cycles::new(5));
        timings.insert(InstructionClass::Atomic(AtomicOp::Store), Cycles::new(5));
        timings.insert(InstructionClass::Atomic(AtomicOp::Add), Cycles::new(10));

        timings.insert(InstructionClass::Other, Cycles::new(1));

        PlatformModel {
            name: "RISC-V RV32I".to_string(),
            cpu_frequency_mhz: 100,
            instruction_timings: timings,
        }
    }
}

/// RISC-V RV32IMAC timing model (with M, A, C extensions)
pub struct RV32IMACModel;

impl RV32IMACModel {
    /// Create RV32IMAC @ 320MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::range(1, 3)); // M extension
        timings.insert(InstructionClass::Div, Cycles::range(3, 33)); // M extension
        timings.insert(InstructionClass::Rem, Cycles::range(3, 33));

        // No FPU
        timings.insert(InstructionClass::FAdd, Cycles::new(100));
        timings.insert(InstructionClass::FSub, Cycles::new(100));
        timings.insert(InstructionClass::FMul, Cycles::new(100));
        timings.insert(InstructionClass::FDiv, Cycles::new(150));

        // Logic
        timings.insert(InstructionClass::And, Cycles::new(1));
        timings.insert(InstructionClass::Or, Cycles::new(1));
        timings.insert(InstructionClass::Xor, Cycles::new(1));
        timings.insert(InstructionClass::Shl, Cycles::new(1));
        timings.insert(InstructionClass::Shr, Cycles::new(1));

        // Memory access
        timings.insert(InstructionClass::Load(AccessType::Ram), Cycles::range(1, 2));
        timings.insert(
            InstructionClass::Store(AccessType::Ram),
            Cycles::range(1, 2),
        );
        timings.insert(
            InstructionClass::Load(AccessType::Flash),
            Cycles::range(2, 3),
        );
        timings.insert(
            InstructionClass::Store(AccessType::Flash),
            Cycles::range(2, 3),
        );

        // Control flow (C extension for compressed instructions)
        timings.insert(InstructionClass::Branch, Cycles::range(1, 2));
        timings.insert(InstructionClass::Call, Cycles::range(2, 3));
        timings.insert(InstructionClass::Ret, Cycles::range(2, 3));

        // Atomics (A extension)
        timings.insert(InstructionClass::Atomic(AtomicOp::Load), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Store), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Add), Cycles::new(3));

        timings.insert(InstructionClass::Other, Cycles::new(1));

        PlatformModel {
            name: "RISC-V RV32IMAC".to_string(),
            cpu_frequency_mhz: 320,
            instruction_timings: timings,
        }
    }
}

/// RISC-V RV32GC timing model (full general-purpose ISA with FPU)
pub struct RV32GCModel;

impl RV32GCModel {
    /// Create RV32GC @ 1000MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::range(1, 2));
        timings.insert(InstructionClass::Div, Cycles::range(3, 20));
        timings.insert(InstructionClass::Rem, Cycles::range(3, 20));

        // Floating point (F/D extensions)
        timings.insert(InstructionClass::FAdd, Cycles::range(3, 5));
        timings.insert(InstructionClass::FSub, Cycles::range(3, 5));
        timings.insert(InstructionClass::FMul, Cycles::range(3, 5));
        timings.insert(InstructionClass::FDiv, Cycles::range(10, 20));

        // Logic
        timings.insert(InstructionClass::And, Cycles::new(1));
        timings.insert(InstructionClass::Or, Cycles::new(1));
        timings.insert(InstructionClass::Xor, Cycles::new(1));
        timings.insert(InstructionClass::Shl, Cycles::new(1));
        timings.insert(InstructionClass::Shr, Cycles::new(1));

        // Memory access (with cache)
        timings.insert(InstructionClass::Load(AccessType::Ram), Cycles::range(1, 3));
        timings.insert(
            InstructionClass::Store(AccessType::Ram),
            Cycles::range(1, 3),
        );
        timings.insert(
            InstructionClass::Load(AccessType::Flash),
            Cycles::range(1, 5),
        );
        timings.insert(
            InstructionClass::Store(AccessType::Flash),
            Cycles::range(1, 5),
        );

        // Control flow
        timings.insert(InstructionClass::Branch, Cycles::range(1, 2));
        timings.insert(InstructionClass::Call, Cycles::range(2, 3));
        timings.insert(InstructionClass::Ret, Cycles::range(2, 3));

        // Atomics
        timings.insert(InstructionClass::Atomic(AtomicOp::Load), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Store), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Add), Cycles::new(3));

        timings.insert(InstructionClass::Other, Cycles::new(1));

        PlatformModel {
            name: "RISC-V RV32GC".to_string(),
            cpu_frequency_mhz: 1000,
            instruction_timings: timings,
        }
    }
}

/// RISC-V RV64GC timing model (64-bit general-purpose ISA)
pub struct RV64GCModel;

impl RV64GCModel {
    /// Create RV64GC @ 1500MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic (64-bit)
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::range(1, 3));
        timings.insert(InstructionClass::Div, Cycles::range(5, 35));
        timings.insert(InstructionClass::Rem, Cycles::range(5, 35));

        // Floating point (double precision)
        timings.insert(InstructionClass::FAdd, Cycles::range(3, 5));
        timings.insert(InstructionClass::FSub, Cycles::range(3, 5));
        timings.insert(InstructionClass::FMul, Cycles::range(3, 5));
        timings.insert(InstructionClass::FDiv, Cycles::range(10, 25));

        // Logic
        timings.insert(InstructionClass::And, Cycles::new(1));
        timings.insert(InstructionClass::Or, Cycles::new(1));
        timings.insert(InstructionClass::Xor, Cycles::new(1));
        timings.insert(InstructionClass::Shl, Cycles::new(1));
        timings.insert(InstructionClass::Shr, Cycles::new(1));

        // Memory access (with cache)
        timings.insert(InstructionClass::Load(AccessType::Ram), Cycles::range(1, 4));
        timings.insert(
            InstructionClass::Store(AccessType::Ram),
            Cycles::range(1, 4),
        );
        timings.insert(
            InstructionClass::Load(AccessType::Flash),
            Cycles::range(1, 6),
        );
        timings.insert(
            InstructionClass::Store(AccessType::Flash),
            Cycles::range(1, 6),
        );

        // Control flow
        timings.insert(InstructionClass::Branch, Cycles::range(1, 2));
        timings.insert(InstructionClass::Call, Cycles::range(2, 3));
        timings.insert(InstructionClass::Ret, Cycles::range(2, 3));

        // Atomics
        timings.insert(InstructionClass::Atomic(AtomicOp::Load), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Store), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Add), Cycles::new(3));

        timings.insert(InstructionClass::Other, Cycles::new(1));

        PlatformModel {
            name: "RISC-V RV64GC".to_string(),
            cpu_frequency_mhz: 1500,
            instruction_timings: timings,
        }
    }
}
