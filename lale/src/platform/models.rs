use crate::analysis::{Cycles, InstructionClass};
use ahash::AHashMap;

/// Platform timing model (placeholder for Phase 3)
#[derive(Clone)]
pub struct PlatformModel {
    pub name: String,
    pub cpu_frequency_mhz: u32,
    pub instruction_timings: AHashMap<InstructionClass, Cycles>,
}

impl PlatformModel {
    /// Get timing for instruction class
    pub fn get_timing(&self, class: &InstructionClass) -> Cycles {
        self.instruction_timings
            .get(class)
            .copied()
            .unwrap_or(Cycles::new(1))
    }
}
