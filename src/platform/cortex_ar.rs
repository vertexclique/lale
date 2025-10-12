use crate::analysis::timing::{AccessType, AtomicOp, Cycles, InstructionClass};
use crate::platform::PlatformModel;
use ahash::AHashMap;

/// ARM Cortex-R4 timing model (ARMv7-R, real-time)
pub struct CortexR4Model;

impl CortexR4Model {
    /// Create Cortex-R4 @ 600MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::range(1, 2));
        timings.insert(InstructionClass::Div, Cycles::range(3, 12));
        timings.insert(InstructionClass::Rem, Cycles::range(3, 12));

        // No FPU (optional VFP)
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

        // Memory access (tightly-coupled memory)
        timings.insert(InstructionClass::Load(AccessType::Ram), Cycles::new(1));
        timings.insert(InstructionClass::Store(AccessType::Ram), Cycles::new(1));
        timings.insert(InstructionClass::Load(AccessType::Flash), Cycles::range(1, 3));
        timings.insert(InstructionClass::Store(AccessType::Flash), Cycles::range(1, 3));

        // Control flow
        timings.insert(InstructionClass::Branch, Cycles::range(1, 2));
        timings.insert(InstructionClass::Call, Cycles::range(2, 4));
        timings.insert(InstructionClass::Ret, Cycles::range(2, 4));

        // Atomics
        timings.insert(InstructionClass::Atomic(AtomicOp::Load), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Store), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Add), Cycles::new(3));

        timings.insert(InstructionClass::Other, Cycles::new(1));

        PlatformModel {
            name: "ARM Cortex-R4".to_string(),
            cpu_frequency_mhz: 600,
            instruction_timings: timings,
        }
    }
}

/// ARM Cortex-R5 timing model (ARMv7-R with optional FPU)
pub struct CortexR5Model;

impl CortexR5Model {
    /// Create Cortex-R5 @ 800MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::new(1));
        timings.insert(InstructionClass::Div, Cycles::range(3, 12));
        timings.insert(InstructionClass::Rem, Cycles::range(3, 12));

        // Floating point (with VFPv3)
        timings.insert(InstructionClass::FAdd, Cycles::range(1, 3));
        timings.insert(InstructionClass::FSub, Cycles::range(1, 3));
        timings.insert(InstructionClass::FMul, Cycles::range(1, 3));
        timings.insert(InstructionClass::FDiv, Cycles::range(10, 15));

        // Logic
        timings.insert(InstructionClass::And, Cycles::new(1));
        timings.insert(InstructionClass::Or, Cycles::new(1));
        timings.insert(InstructionClass::Xor, Cycles::new(1));
        timings.insert(InstructionClass::Shl, Cycles::new(1));
        timings.insert(InstructionClass::Shr, Cycles::new(1));

        // Memory access (with cache)
        timings.insert(InstructionClass::Load(AccessType::Ram), Cycles::range(1, 2));
        timings.insert(InstructionClass::Store(AccessType::Ram), Cycles::range(1, 2));
        timings.insert(InstructionClass::Load(AccessType::Flash), Cycles::range(1, 4));
        timings.insert(InstructionClass::Store(AccessType::Flash), Cycles::range(1, 4));

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
            name: "ARM Cortex-R5".to_string(),
            cpu_frequency_mhz: 800,
            instruction_timings: timings,
        }
    }
}

/// ARM Cortex-A7 timing model (ARMv7-A, application processor)
pub struct CortexA7Model;

impl CortexA7Model {
    /// Create Cortex-A7 @ 1200MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic (in-order, dual-issue)
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::range(1, 2));
        timings.insert(InstructionClass::Div, Cycles::range(3, 15));
        timings.insert(InstructionClass::Rem, Cycles::range(3, 15));

        // Floating point (NEON)
        timings.insert(InstructionClass::FAdd, Cycles::range(2, 4));
        timings.insert(InstructionClass::FSub, Cycles::range(2, 4));
        timings.insert(InstructionClass::FMul, Cycles::range(2, 4));
        timings.insert(InstructionClass::FDiv, Cycles::range(8, 15));

        // Logic
        timings.insert(InstructionClass::And, Cycles::new(1));
        timings.insert(InstructionClass::Or, Cycles::new(1));
        timings.insert(InstructionClass::Xor, Cycles::new(1));
        timings.insert(InstructionClass::Shl, Cycles::new(1));
        timings.insert(InstructionClass::Shr, Cycles::new(1));

        // Memory access (with L1/L2 cache)
        timings.insert(InstructionClass::Load(AccessType::Ram), Cycles::range(1, 5));
        timings.insert(InstructionClass::Store(AccessType::Ram), Cycles::range(1, 5));
        timings.insert(InstructionClass::Load(AccessType::Flash), Cycles::range(1, 10));
        timings.insert(InstructionClass::Store(AccessType::Flash), Cycles::range(1, 10));

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
            name: "ARM Cortex-A7".to_string(),
            cpu_frequency_mhz: 1200,
            instruction_timings: timings,
        }
    }
}

/// ARM Cortex-A53 timing model (ARMv8-A, 64-bit)
pub struct CortexA53Model;

impl CortexA53Model {
    /// Create Cortex-A53 @ 1400MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic (in-order, dual-issue)
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::range(1, 3));
        timings.insert(InstructionClass::Div, Cycles::range(4, 20));
        timings.insert(InstructionClass::Rem, Cycles::range(4, 20));

        // Floating point (NEON/SIMD)
        timings.insert(InstructionClass::FAdd, Cycles::range(2, 4));
        timings.insert(InstructionClass::FSub, Cycles::range(2, 4));
        timings.insert(InstructionClass::FMul, Cycles::range(2, 4));
        timings.insert(InstructionClass::FDiv, Cycles::range(8, 18));

        // Logic
        timings.insert(InstructionClass::And, Cycles::new(1));
        timings.insert(InstructionClass::Or, Cycles::new(1));
        timings.insert(InstructionClass::Xor, Cycles::new(1));
        timings.insert(InstructionClass::Shl, Cycles::new(1));
        timings.insert(InstructionClass::Shr, Cycles::new(1));

        // Memory access (with L1/L2 cache)
        timings.insert(InstructionClass::Load(AccessType::Ram), Cycles::range(1, 6));
        timings.insert(InstructionClass::Store(AccessType::Ram), Cycles::range(1, 6));
        timings.insert(InstructionClass::Load(AccessType::Flash), Cycles::range(1, 12));
        timings.insert(InstructionClass::Store(AccessType::Flash), Cycles::range(1, 12));

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
            name: "ARM Cortex-A53".to_string(),
            cpu_frequency_mhz: 1400,
            instruction_timings: timings,
        }
    }
}
