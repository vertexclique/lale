use ahash::AHashSet;

/// Data hazard types in pipelined processors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HazardType {
    /// Read After Write - true dependency
    RAW,

    /// Write After Read - anti-dependency
    WAR,

    /// Write After Write - output dependency
    WAW,
}

/// Register identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Register(pub u32);

/// Instruction with register dependencies
#[derive(Debug, Clone)]
pub struct InstructionDependency {
    /// Instruction ID
    pub id: usize,

    /// Registers read by this instruction
    pub reads: Vec<Register>,

    /// Registers written by this instruction
    pub writes: Vec<Register>,

    /// Pipeline stage when instruction starts
    pub stage: usize,
}

/// Detected hazard between two instructions
#[derive(Debug, Clone)]
pub struct Hazard {
    /// Type of hazard
    pub hazard_type: HazardType,

    /// Earlier instruction (producer)
    pub producer: usize,

    /// Later instruction (consumer)
    pub consumer: usize,

    /// Register involved
    pub register: Register,

    /// Number of stall cycles needed
    pub stall_cycles: usize,
}

/// Hazard detection unit
pub struct HazardDetector {
    /// Pipeline depth
    pipeline_depth: usize,

    /// Whether forwarding is enabled
    forwarding_enabled: bool,
}

impl HazardDetector {
    pub fn new(pipeline_depth: usize, forwarding_enabled: bool) -> Self {
        Self {
            pipeline_depth,
            forwarding_enabled,
        }
    }

    /// Detect all hazards in instruction sequence
    pub fn detect_hazards(&self, instructions: &[InstructionDependency]) -> Vec<Hazard> {
        let mut hazards = Vec::new();

        for i in 0..instructions.len() {
            for j in (i + 1)..instructions.len() {
                let earlier = &instructions[i];
                let later = &instructions[j];

                // Check for RAW hazards
                for &write_reg in &earlier.writes {
                    if later.reads.contains(&write_reg) {
                        let stall = self.calculate_raw_stall(earlier, later);
                        if stall > 0 {
                            hazards.push(Hazard {
                                hazard_type: HazardType::RAW,
                                producer: earlier.id,
                                consumer: later.id,
                                register: write_reg,
                                stall_cycles: stall,
                            });
                        }
                    }
                }

                // Check for WAR hazards
                for &read_reg in &earlier.reads {
                    if later.writes.contains(&read_reg) {
                        let stall = self.calculate_war_stall(earlier, later);
                        if stall > 0 {
                            hazards.push(Hazard {
                                hazard_type: HazardType::WAR,
                                producer: earlier.id,
                                consumer: later.id,
                                register: read_reg,
                                stall_cycles: stall,
                            });
                        }
                    }
                }

                // Check for WAW hazards
                for &write_reg in &earlier.writes {
                    if later.writes.contains(&write_reg) {
                        let stall = self.calculate_waw_stall(earlier, later);
                        if stall > 0 {
                            hazards.push(Hazard {
                                hazard_type: HazardType::WAW,
                                producer: earlier.id,
                                consumer: later.id,
                                register: write_reg,
                                stall_cycles: stall,
                            });
                        }
                    }
                }
            }
        }

        hazards
    }

    /// Calculate stall cycles for RAW hazard
    fn calculate_raw_stall(
        &self,
        producer: &InstructionDependency,
        consumer: &InstructionDependency,
    ) -> usize {
        // Distance between instructions
        let distance = consumer.stage.saturating_sub(producer.stage);

        if self.forwarding_enabled {
            // With forwarding, data available after EX stage (stage 2)
            // Need to stall if consumer needs data before it's ready
            if distance < 2 {
                2 - distance
            } else {
                0
            }
        } else {
            // Without forwarding, data available after WB stage (last stage)
            let wb_stage = self.pipeline_depth - 1;
            if distance < wb_stage {
                wb_stage - distance
            } else {
                0
            }
        }
    }

    /// Calculate stall cycles for WAR hazard
    fn calculate_war_stall(
        &self,
        _producer: &InstructionDependency,
        _consumer: &InstructionDependency,
    ) -> usize {
        // WAR hazards typically don't cause stalls in in-order pipelines
        // They're mainly a concern for out-of-order execution
        0
    }

    /// Calculate stall cycles for WAW hazard
    fn calculate_waw_stall(
        &self,
        _producer: &InstructionDependency,
        _consumer: &InstructionDependency,
    ) -> usize {
        // WAW hazards typically don't cause stalls in in-order pipelines
        // They're mainly a concern for out-of-order execution
        0
    }

    /// Get total stall cycles for instruction sequence
    pub fn total_stall_cycles(&self, instructions: &[InstructionDependency]) -> usize {
        let hazards = self.detect_hazards(instructions);
        hazards.iter().map(|h| h.stall_cycles).sum()
    }

    /// Check if two instructions have data dependency
    pub fn has_dependency(
        &self,
        earlier: &InstructionDependency,
        later: &InstructionDependency,
    ) -> bool {
        // RAW dependency
        for &write_reg in &earlier.writes {
            if later.reads.contains(&write_reg) {
                return true;
            }
        }

        // WAR dependency
        for &read_reg in &earlier.reads {
            if later.writes.contains(&read_reg) {
                return true;
            }
        }

        // WAW dependency
        for &write_reg in &earlier.writes {
            if later.writes.contains(&write_reg) {
                return true;
            }
        }

        false
    }
}

/// Dependency graph for instruction scheduling
pub struct DependencyGraph {
    /// Nodes (instructions)
    instructions: Vec<InstructionDependency>,

    /// Edges (dependencies)
    dependencies: Vec<(usize, usize, HazardType)>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            dependencies: Vec::new(),
        }
    }

    /// Add instruction to graph
    pub fn add_instruction(&mut self, instr: InstructionDependency) {
        self.instructions.push(instr);
    }

    /// Build dependency edges
    pub fn build_dependencies(&mut self, detector: &HazardDetector) {
        self.dependencies.clear();

        for i in 0..self.instructions.len() {
            for j in (i + 1)..self.instructions.len() {
                let earlier = &self.instructions[i];
                let later = &self.instructions[j];

                // Check RAW
                for &write_reg in &earlier.writes {
                    if later.reads.contains(&write_reg) {
                        self.dependencies.push((i, j, HazardType::RAW));
                        break;
                    }
                }

                // Check WAR
                for &read_reg in &earlier.reads {
                    if later.writes.contains(&read_reg) {
                        self.dependencies.push((i, j, HazardType::WAR));
                        break;
                    }
                }

                // Check WAW
                for &write_reg in &earlier.writes {
                    if later.writes.contains(&write_reg) {
                        self.dependencies.push((i, j, HazardType::WAW));
                        break;
                    }
                }
            }
        }
    }

    /// Get instructions that can execute in parallel
    pub fn get_independent_instructions(&self) -> Vec<AHashSet<usize>> {
        let mut groups = Vec::new();
        let mut scheduled = AHashSet::new();

        while scheduled.len() < self.instructions.len() {
            let mut current_group = AHashSet::new();

            for i in 0..self.instructions.len() {
                if scheduled.contains(&i) {
                    continue;
                }

                // Check if all dependencies are satisfied
                let mut can_schedule = true;
                for &(from, to, _) in &self.dependencies {
                    if to == i && !scheduled.contains(&from) {
                        can_schedule = false;
                        break;
                    }
                }

                if can_schedule {
                    // Check if independent from current group
                    let mut independent = true;
                    for &other in &current_group {
                        for &(from, to, _) in &self.dependencies {
                            if (from == i && to == other) || (from == other && to == i) {
                                independent = false;
                                break;
                            }
                        }
                        if !independent {
                            break;
                        }
                    }

                    if independent {
                        current_group.insert(i);
                    }
                }
            }

            if current_group.is_empty() {
                break;
            }

            for &i in &current_group {
                scheduled.insert(i);
            }

            groups.push(current_group);
        }

        groups
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_hazard_detection() {
        let detector = HazardDetector::new(5, false);

        let instr1 = InstructionDependency {
            id: 0,
            reads: vec![],
            writes: vec![Register(1)],
            stage: 0,
        };

        let instr2 = InstructionDependency {
            id: 1,
            reads: vec![Register(1)],
            writes: vec![],
            stage: 1,
        };

        let hazards = detector.detect_hazards(&[instr1, instr2]);

        assert_eq!(hazards.len(), 1);
        assert_eq!(hazards[0].hazard_type, HazardType::RAW);
        assert!(hazards[0].stall_cycles > 0);
    }

    #[test]
    fn test_forwarding_reduces_stalls() {
        let without_forwarding = HazardDetector::new(5, false);
        let with_forwarding = HazardDetector::new(5, true);

        let instr1 = InstructionDependency {
            id: 0,
            reads: vec![],
            writes: vec![Register(1)],
            stage: 0,
        };

        let instr2 = InstructionDependency {
            id: 1,
            reads: vec![Register(1)],
            writes: vec![],
            stage: 1,
        };

        let stalls_without =
            without_forwarding.total_stall_cycles(&[instr1.clone(), instr2.clone()]);
        let stalls_with = with_forwarding.total_stall_cycles(&[instr1, instr2]);

        assert!(stalls_with < stalls_without);
    }

    #[test]
    fn test_dependency_graph() {
        let mut graph = DependencyGraph::new();
        let detector = HazardDetector::new(5, true);

        graph.add_instruction(InstructionDependency {
            id: 0,
            reads: vec![],
            writes: vec![Register(1)],
            stage: 0,
        });

        graph.add_instruction(InstructionDependency {
            id: 1,
            reads: vec![Register(1)],
            writes: vec![Register(2)],
            stage: 1,
        });

        graph.add_instruction(InstructionDependency {
            id: 2,
            reads: vec![Register(2)],
            writes: vec![],
            stage: 2,
        });

        graph.build_dependencies(&detector);

        assert!(!graph.dependencies.is_empty());
    }
}
