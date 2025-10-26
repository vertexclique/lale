use super::state::Address;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Pipeline state tracking instructions in each stage
#[derive(Debug, Clone)]
pub struct PipelineState {
    /// Pipeline stages (ordered from fetch to writeback)
    pub stages: Vec<PipelineStage>,
}

impl PipelineState {
    /// Create new empty pipeline with given depth
    pub fn new(depth: usize) -> Self {
        let stage_types = match depth {
            3 => vec![StageType::Fetch, StageType::Decode, StageType::Execute],
            5 => vec![
                StageType::Fetch,
                StageType::Decode,
                StageType::Execute,
                StageType::Memory,
                StageType::WriteBack,
            ],
            6 => vec![
                StageType::Fetch,
                StageType::Decode,
                StageType::Execute,
                StageType::Memory,
                StageType::WriteBack1,
                StageType::WriteBack2,
            ],
            _ => panic!("Unsupported pipeline depth: {}", depth),
        };

        let stages = stage_types
            .into_iter()
            .map(|stage_type| PipelineStage {
                instruction: None,
                stage_type,
                stalled: false,
            })
            .collect();

        Self { stages }
    }

    /// Compute hash for state comparison
    pub fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();

        for stage in &self.stages {
            stage.hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Check if two pipeline states are compatible for joining
    pub fn is_compatible(&self, other: &Self) -> bool {
        if self.stages.len() != other.stages.len() {
            return false;
        }

        // Check if stage types match
        for (s1, s2) in self.stages.iter().zip(other.stages.iter()) {
            if s1.stage_type != s2.stage_type {
                return false;
            }
        }

        true
    }

    /// Join two compatible pipeline states
    pub fn join(&self, other: &Self) -> Self {
        assert!(self.is_compatible(other), "Incompatible pipeline states");

        let stages = self
            .stages
            .iter()
            .zip(other.stages.iter())
            .map(|(s1, s2)| s1.join(s2))
            .collect();

        Self { stages }
    }

    /// Check if pipeline is empty
    pub fn is_empty(&self) -> bool {
        self.stages.iter().all(|s| s.instruction.is_none())
    }

    /// Get instruction in specific stage
    pub fn get_stage(&self, stage_type: StageType) -> Option<&PipelineStage> {
        self.stages.iter().find(|s| s.stage_type == stage_type)
    }

    /// Get mutable reference to specific stage
    pub fn get_stage_mut(&mut self, stage_type: StageType) -> Option<&mut PipelineStage> {
        self.stages.iter_mut().find(|s| s.stage_type == stage_type)
    }
}

/// Single pipeline stage
#[derive(Debug, Clone)]
pub struct PipelineStage {
    /// Instruction currently in this stage (if any)
    pub instruction: Option<InstructionSlot>,

    /// Type of this stage
    pub stage_type: StageType,

    /// Whether this stage is stalled
    pub stalled: bool,
}

impl PipelineStage {
    /// Join two pipeline stages
    pub fn join(&self, other: &Self) -> Self {
        assert_eq!(self.stage_type, other.stage_type);

        // Conservative join: if either has instruction, result has instruction
        // If either is stalled, result is stalled
        Self {
            instruction: self
                .instruction
                .as_ref()
                .or(other.instruction.as_ref())
                .cloned(),
            stage_type: self.stage_type,
            stalled: self.stalled || other.stalled,
        }
    }
}

impl Hash for PipelineStage {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.stage_type.hash(state);
        self.stalled.hash(state);
        if let Some(instr) = &self.instruction {
            instr.hash(state);
        }
    }
}

/// Type of pipeline stage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StageType {
    Fetch,
    Decode,
    Execute,
    Memory,
    WriteBack,
    WriteBack1,
    WriteBack2,
}

/// Instruction slot in pipeline
#[derive(Debug, Clone, Hash)]
pub struct InstructionSlot {
    /// Address of this instruction
    pub address: Address,

    /// Opcode/operation type
    pub opcode: Opcode,

    /// Register dependencies
    pub dependencies: Vec<RegisterDep>,

    /// Memory access information (if applicable)
    pub memory_access: Option<MemoryAccess>,
}

/// Opcode classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Opcode {
    ALU,
    Multiply,
    Divide,
    Load,
    Store,
    Branch,
    FloatingPoint,
    Other,
}

/// Register dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegisterDep {
    pub register: u8,
    pub dep_type: DepType,
}

/// Dependency type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DepType {
    Read,
    Write,
}

/// Memory access information
#[derive(Debug, Clone, Hash)]
pub struct MemoryAccess {
    /// Abstract address (may be range)
    pub address: AbstractAddress,

    /// Access type
    pub access_type: MemoryAccessType,

    /// Size in bytes
    pub size: usize,
}

/// Abstract address representation
#[derive(Debug, Clone, Hash)]
pub enum AbstractAddress {
    /// Concrete address
    Concrete(u64),

    /// Address range [min, max]
    Range { min: u64, max: u64 },

    /// Unknown address
    Unknown,
}

/// Memory access type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryAccessType {
    Load,
    Store,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_creation() {
        let pipeline = PipelineState::new(5);
        assert_eq!(pipeline.stages.len(), 5);
        assert!(pipeline.is_empty());
    }

    #[test]
    fn test_pipeline_stages() {
        let pipeline = PipelineState::new(5);

        assert!(pipeline.get_stage(StageType::Fetch).is_some());
        assert!(pipeline.get_stage(StageType::Decode).is_some());
        assert!(pipeline.get_stage(StageType::Execute).is_some());
        assert!(pipeline.get_stage(StageType::Memory).is_some());
        assert!(pipeline.get_stage(StageType::WriteBack).is_some());
    }

    #[test]
    fn test_pipeline_compatibility() {
        let p1 = PipelineState::new(5);
        let p2 = PipelineState::new(5);
        let p3 = PipelineState::new(3);

        assert!(p1.is_compatible(&p2));
        assert!(!p1.is_compatible(&p3));
    }

    #[test]
    fn test_pipeline_join() {
        let p1 = PipelineState::new(5);
        let p2 = PipelineState::new(5);

        let joined = p1.join(&p2);
        assert_eq!(joined.stages.len(), 5);
    }

    #[test]
    fn test_pipeline_hash() {
        let p1 = PipelineState::new(5);
        let p2 = PipelineState::new(5);

        // Same empty pipelines should have same hash
        assert_eq!(p1.hash(), p2.hash());
    }
}
