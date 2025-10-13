use crate::analysis::timing::{AccessType, AtomicOp, Cycles, InstructionClass};
use crate::platform::PlatformModel;
use ahash::AHashMap;

/// ARM Cortex-M0/M0+/M1 timing model (ARMv6-M)
pub struct CortexM0Model;

impl CortexM0Model {
    /// Create Cortex-M0 @ 48MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic (1 cycle, no hardware multiply/divide)
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::new(32)); // Software multiply
        timings.insert(InstructionClass::Div, Cycles::new(40)); // Software divide
        timings.insert(InstructionClass::Rem, Cycles::new(40));

        // No FPU
        timings.insert(InstructionClass::FAdd, Cycles::new(100));
        timings.insert(InstructionClass::FSub, Cycles::new(100));
        timings.insert(InstructionClass::FMul, Cycles::new(100));
        timings.insert(InstructionClass::FDiv, Cycles::new(150));

        // Logic (1 cycle)
        timings.insert(InstructionClass::And, Cycles::new(1));
        timings.insert(InstructionClass::Or, Cycles::new(1));
        timings.insert(InstructionClass::Xor, Cycles::new(1));
        timings.insert(InstructionClass::Shl, Cycles::new(1));
        timings.insert(InstructionClass::Shr, Cycles::new(1));

        // Memory access (no cache)
        timings.insert(InstructionClass::Load(AccessType::Ram), Cycles::new(2));
        timings.insert(InstructionClass::Store(AccessType::Ram), Cycles::new(2));
        timings.insert(InstructionClass::Load(AccessType::Flash), Cycles::new(2));
        timings.insert(InstructionClass::Store(AccessType::Flash), Cycles::new(2));

        // Control flow
        timings.insert(InstructionClass::Branch, Cycles::range(1, 3));
        timings.insert(InstructionClass::Call, Cycles::range(3, 4));
        timings.insert(InstructionClass::Ret, Cycles::range(3, 4));

        // Atomics (limited support)
        timings.insert(InstructionClass::Atomic(AtomicOp::Load), Cycles::new(3));
        timings.insert(InstructionClass::Atomic(AtomicOp::Store), Cycles::new(3));
        timings.insert(InstructionClass::Atomic(AtomicOp::Add), Cycles::new(5));

        timings.insert(InstructionClass::Other, Cycles::new(1));

        PlatformModel {
            name: "ARM Cortex-M0".to_string(),
            cpu_frequency_mhz: 48,
            instruction_timings: timings,
        }
    }
}

/// ARM Cortex-M3 timing model (ARMv7-M)
pub struct CortexM3Model;

impl CortexM3Model {
    /// Create Cortex-M3 @ 72MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::new(1)); // Hardware multiply
        timings.insert(InstructionClass::Div, Cycles::range(2, 12)); // Hardware divide
        timings.insert(InstructionClass::Rem, Cycles::range(2, 12));

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
        timings.insert(InstructionClass::Store(AccessType::Ram), Cycles::range(1, 2));
        timings.insert(InstructionClass::Load(AccessType::Flash), Cycles::range(2, 3));
        timings.insert(InstructionClass::Store(AccessType::Flash), Cycles::range(2, 3));

        // Control flow
        timings.insert(InstructionClass::Branch, Cycles::range(1, 3));
        timings.insert(InstructionClass::Call, Cycles::range(3, 5));
        timings.insert(InstructionClass::Ret, Cycles::range(3, 5));

        // Atomics
        timings.insert(InstructionClass::Atomic(AtomicOp::Load), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Store), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Add), Cycles::new(3));

        timings.insert(InstructionClass::Other, Cycles::new(1));

        PlatformModel {
            name: "ARM Cortex-M3".to_string(),
            cpu_frequency_mhz: 72,
            instruction_timings: timings,
        }
    }
}

/// ARM Cortex-M4 timing model (ARMv7E-M)
pub struct CortexM4Model;

impl CortexM4Model {
    /// Create Cortex-M4 @ 168MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic (1 cycle)
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::range(1, 2));
        timings.insert(InstructionClass::Div, Cycles::new(12));
        timings.insert(InstructionClass::Rem, Cycles::new(12));

        // Floating point (with FPU)
        timings.insert(InstructionClass::FAdd, Cycles::new(1));
        timings.insert(InstructionClass::FSub, Cycles::new(1));
        timings.insert(InstructionClass::FMul, Cycles::new(1));
        timings.insert(InstructionClass::FDiv, Cycles::new(15));

        // Logic (1 cycle)
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
            Cycles::range(3, 5),
        );
        timings.insert(
            InstructionClass::Store(AccessType::Flash),
            Cycles::range(3, 5),
        );

        // Control flow
        timings.insert(InstructionClass::Branch, Cycles::range(1, 3));
        timings.insert(InstructionClass::Call, Cycles::range(3, 5));
        timings.insert(InstructionClass::Ret, Cycles::range(3, 5));

        // Atomics
        timings.insert(InstructionClass::Atomic(AtomicOp::Load), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Store), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Add), Cycles::new(3));

        // Default
        timings.insert(InstructionClass::Other, Cycles::new(1));

        PlatformModel {
            name: "ARM Cortex-M4".to_string(),
            cpu_frequency_mhz: 168,
            instruction_timings: timings,
        }
    }
}

/// ARM Cortex-M7 timing model (ARMv7E-M)
pub struct CortexM7Model;

impl CortexM7Model {
    /// Create Cortex-M7 @ 400MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic (1 cycle, dual-issue capable)
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::new(1));
        timings.insert(InstructionClass::Div, Cycles::range(3, 12));
        timings.insert(InstructionClass::Rem, Cycles::range(3, 12));

        // Floating point (with FPU, double precision)
        timings.insert(InstructionClass::FAdd, Cycles::new(1));
        timings.insert(InstructionClass::FSub, Cycles::new(1));
        timings.insert(InstructionClass::FMul, Cycles::new(1));
        timings.insert(InstructionClass::FDiv, Cycles::new(14));

        // Logic
        timings.insert(InstructionClass::And, Cycles::new(1));
        timings.insert(InstructionClass::Or, Cycles::new(1));
        timings.insert(InstructionClass::Xor, Cycles::new(1));
        timings.insert(InstructionClass::Shl, Cycles::new(1));
        timings.insert(InstructionClass::Shr, Cycles::new(1));

        // Memory access (with cache)
        timings.insert(InstructionClass::Load(AccessType::Ram), Cycles::range(1, 3));
        timings.insert(InstructionClass::Store(AccessType::Ram), Cycles::range(1, 3));
        timings.insert(InstructionClass::Load(AccessType::Flash), Cycles::range(1, 5));
        timings.insert(InstructionClass::Store(AccessType::Flash), Cycles::range(1, 5));

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
            name: "ARM Cortex-M7".to_string(),
            cpu_frequency_mhz: 400,
            instruction_timings: timings,
        }
    }
}

/// ARM Cortex-M33 timing model (ARMv8-M)
pub struct CortexM33Model;

impl CortexM33Model {
    /// Create Cortex-M33 @ 120MHz timing model
    pub fn new() -> PlatformModel {
        let mut timings = AHashMap::new();

        // Integer arithmetic
        timings.insert(InstructionClass::Add, Cycles::new(1));
        timings.insert(InstructionClass::Sub, Cycles::new(1));
        timings.insert(InstructionClass::Mul, Cycles::new(1));
        timings.insert(InstructionClass::Div, Cycles::range(2, 12));
        timings.insert(InstructionClass::Rem, Cycles::range(2, 12));

        // Floating point (optional FPU)
        timings.insert(InstructionClass::FAdd, Cycles::new(1));
        timings.insert(InstructionClass::FSub, Cycles::new(1));
        timings.insert(InstructionClass::FMul, Cycles::new(1));
        timings.insert(InstructionClass::FDiv, Cycles::new(15));

        // Logic
        timings.insert(InstructionClass::And, Cycles::new(1));
        timings.insert(InstructionClass::Or, Cycles::new(1));
        timings.insert(InstructionClass::Xor, Cycles::new(1));
        timings.insert(InstructionClass::Shl, Cycles::new(1));
        timings.insert(InstructionClass::Shr, Cycles::new(1));

        // Memory access
        timings.insert(InstructionClass::Load(AccessType::Ram), Cycles::range(1, 2));
        timings.insert(InstructionClass::Store(AccessType::Ram), Cycles::range(1, 2));
        timings.insert(InstructionClass::Load(AccessType::Flash), Cycles::range(2, 4));
        timings.insert(InstructionClass::Store(AccessType::Flash), Cycles::range(2, 4));

        // Control flow
        timings.insert(InstructionClass::Branch, Cycles::range(1, 3));
        timings.insert(InstructionClass::Call, Cycles::range(3, 5));
        timings.insert(InstructionClass::Ret, Cycles::range(3, 5));

        // Atomics (TrustZone support)
        timings.insert(InstructionClass::Atomic(AtomicOp::Load), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Store), Cycles::new(2));
        timings.insert(InstructionClass::Atomic(AtomicOp::Add), Cycles::new(3));

        timings.insert(InstructionClass::Other, Cycles::new(1));

        PlatformModel {
            name: "ARM Cortex-M33".to_string(),
            cpu_frequency_mhz: 120,
            instruction_timings: timings,
        }
    }
}
