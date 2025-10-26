use crate::analysis::timing::{classify_instruction, Cycles, InstructionClass};
use crate::ir::CFG;
use crate::platform::PlatformModel;
use ahash::AHashMap;
use llvm_ir::Function;
use petgraph::graph::NodeIndex;

/// Calculate timing for basic blocks in a CFG
pub struct TimingCalculator;

impl TimingCalculator {
    /// Calculate timing for all basic blocks in a function
    pub fn calculate_block_timings(
        function: &Function,
        cfg: &CFG,
        platform: &PlatformModel,
    ) -> AHashMap<NodeIndex, Cycles> {
        let mut timings = AHashMap::new();

        for bb in &function.basic_blocks {
            // Find corresponding node in CFG
            let node = cfg.label_to_node.get(&bb.name.to_string());

            if let Some(&node_idx) = node {
                let mut total_cycles = Cycles::new(0);

                // Sum timing for all instructions in basic block
                for instr in &bb.instrs {
                    let class = classify_instruction(instr);
                    let instr_cycles = platform.get_timing(&class);

                    // Add to total (worst-case)
                    total_cycles = Cycles {
                        best_case: total_cycles.best_case + instr_cycles.best_case,
                        worst_case: total_cycles.worst_case + instr_cycles.worst_case,
                    };
                }

                // Add terminator cost (branch/return)
                let term_cycles = Self::calculate_terminator_cost(&bb.term, platform);
                total_cycles = Cycles {
                    best_case: total_cycles.best_case + term_cycles.best_case,
                    worst_case: total_cycles.worst_case + term_cycles.worst_case,
                };

                timings.insert(node_idx, total_cycles);
            }
        }

        timings
    }

    /// Calculate cost of terminator instruction
    fn calculate_terminator_cost(
        terminator: &llvm_ir::Terminator,
        platform: &PlatformModel,
    ) -> Cycles {
        use llvm_ir::Terminator::*;

        match terminator {
            Ret(_) => platform.get_timing(&InstructionClass::Ret),
            Br(_) | CondBr(_) => platform.get_timing(&InstructionClass::Branch),
            Switch(_) => platform.get_timing(&InstructionClass::Branch),
            _ => Cycles::new(1), // Conservative default
        }
    }

    /// Convert cycles to microseconds
    pub fn cycles_to_us(cycles: u64, cpu_freq_mhz: u32) -> f64 {
        cycles as f64 / (cpu_freq_mhz as f64)
    }

    /// Convert microseconds to cycles
    pub fn us_to_cycles(us: f64, cpu_freq_mhz: u32) -> u64 {
        (us * cpu_freq_mhz as f64) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::parser::IRParser;
    use crate::platform::CortexM4Model;

    #[test]
    fn test_timing_calculation() {
        let sample_path = "data/armv7e-m/56e3741adeae4068.ll";
        if std::path::Path::new(sample_path).exists() {
            let module = IRParser::parse_file(sample_path).unwrap();
            let platform = CortexM4Model::new();

            if let Some(function) = module.functions.first() {
                let cfg = CFG::from_function(function);
                let timings = TimingCalculator::calculate_block_timings(function, &cfg, &platform);

                assert!(!timings.is_empty(), "Should calculate timings for blocks");

                // Verify all timings are non-zero
                for (_, cycles) in &timings {
                    assert!(cycles.worst_case > 0, "Worst case should be positive");
                }
            }
        }
    }

    #[test]
    fn test_cycle_conversion() {
        let cycles = 168;
        let freq_mhz = 168;

        let us = TimingCalculator::cycles_to_us(cycles, freq_mhz);
        assert!((us - 1.0).abs() < 0.001);

        let back_to_cycles = TimingCalculator::us_to_cycles(us, freq_mhz);
        assert_eq!(back_to_cycles, cycles);
    }
}
