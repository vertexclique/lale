use super::hazards::{HazardType, InstructionDependency, Register};
use ahash::AHashMap;

/// Forwarding path between pipeline stages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForwardingPath {
    /// Source stage (where data is produced)
    pub from_stage: usize,
    
    /// Destination stage (where data is needed)
    pub to_stage: usize,
    
    /// Register being forwarded
    pub register: Register,
}

/// Forwarding unit that manages data forwarding paths
pub struct ForwardingUnit {
    /// Pipeline depth
    pipeline_depth: usize,
    
    /// Available forwarding paths
    paths: Vec<ForwardingPath>,
    
    /// Forwarding latency (cycles)
    forwarding_latency: usize,
}

impl ForwardingUnit {
    /// Create new forwarding unit
    pub fn new(pipeline_depth: usize) -> Self {
        Self {
            pipeline_depth,
            paths: Vec::new(),
            forwarding_latency: 0, // Typically 0 for same-cycle forwarding
        }
    }
    
    /// Enable standard forwarding paths (EX->EX, MEM->EX, WB->EX)
    pub fn enable_standard_paths(&mut self) {
        // Typical 5-stage pipeline: IF, ID, EX, MEM, WB
        // EX stage = 2, MEM stage = 3, WB stage = 4
        
        if self.pipeline_depth >= 5 {
            // EX->EX forwarding (for back-to-back ALU operations)
            self.add_path(2, 2);
            
            // MEM->EX forwarding (for load-use)
            self.add_path(3, 2);
            
            // WB->EX forwarding (last resort)
            self.add_path(4, 2);
        }
    }
    
    /// Add a forwarding path
    fn add_path(&mut self, from_stage: usize, to_stage: usize) {
        // Paths are register-agnostic, will be matched at runtime
        self.paths.push(ForwardingPath {
            from_stage,
            to_stage,
            register: Register(0), // Placeholder
        });
    }
    
    /// Check if forwarding is possible for a dependency
    pub fn can_forward(&self, producer: &InstructionDependency, consumer: &InstructionDependency, register: Register) -> bool {
        // Check if there's a path from producer's stage to consumer's stage
        for path in &self.paths {
            if path.from_stage >= producer.stage && path.to_stage == consumer.stage {
                return true;
            }
        }
        false
    }
    
    /// Get forwarding latency for a specific path
    pub fn get_forwarding_latency(&self, producer_stage: usize, consumer_stage: usize) -> usize {
        for path in &self.paths {
            if path.from_stage == producer_stage && path.to_stage == consumer_stage {
                return self.forwarding_latency;
            }
        }
        
        // No forwarding path available
        self.pipeline_depth
    }
    
    /// Resolve hazard with forwarding
    pub fn resolve_hazard(&self, producer: &InstructionDependency, consumer: &InstructionDependency, register: Register) -> ForwardingResolution {
        if self.can_forward(producer, consumer, register) {
            ForwardingResolution {
                can_forward: true,
                stall_cycles: self.forwarding_latency,
                path: Some(ForwardingPath {
                    from_stage: producer.stage,
                    to_stage: consumer.stage,
                    register,
                }),
            }
        } else {
            // Must stall until data reaches WB stage
            let stall = (self.pipeline_depth - 1).saturating_sub(consumer.stage - producer.stage);
            ForwardingResolution {
                can_forward: false,
                stall_cycles: stall,
                path: None,
            }
        }
    }
}

/// Result of forwarding resolution
#[derive(Debug, Clone)]
pub struct ForwardingResolution {
    /// Whether forwarding is possible
    pub can_forward: bool,
    
    /// Number of stall cycles needed
    pub stall_cycles: usize,
    
    /// Forwarding path used (if any)
    pub path: Option<ForwardingPath>,
}

/// Forwarding network that tracks active forwards
pub struct ForwardingNetwork {
    /// Active forwarding paths
    active_forwards: AHashMap<Register, Vec<ForwardingPath>>,
    
    /// Forwarding unit
    unit: ForwardingUnit,
}

impl ForwardingNetwork {
    pub fn new(pipeline_depth: usize) -> Self {
        let mut unit = ForwardingUnit::new(pipeline_depth);
        unit.enable_standard_paths();
        
        Self {
            active_forwards: AHashMap::new(),
            unit,
        }
    }
    
    /// Register a forwarding path
    pub fn register_forward(&mut self, path: ForwardingPath) {
        self.active_forwards
            .entry(path.register)
            .or_insert_with(Vec::new)
            .push(path);
    }
    
    /// Check if data is available via forwarding
    pub fn is_data_available(&self, register: Register, at_stage: usize) -> bool {
        if let Some(paths) = self.active_forwards.get(&register) {
            for path in paths {
                if path.to_stage <= at_stage {
                    return true;
                }
            }
        }
        false
    }
    
    /// Clear forwarding paths (e.g., on pipeline flush)
    pub fn clear(&mut self) {
        self.active_forwards.clear();
    }
    
    /// Get forwarding unit
    pub fn unit(&self) -> &ForwardingUnit {
        &self.unit
    }
}

/// Bypass network for data forwarding
#[derive(Debug, Clone)]
pub struct BypassNetwork {
    /// Bypass paths from each stage
    bypasses: Vec<Vec<usize>>, // bypasses[from_stage] = [to_stages]
}

impl BypassNetwork {
    pub fn new(pipeline_depth: usize) -> Self {
        let mut bypasses = vec![Vec::new(); pipeline_depth];
        
        // Standard bypasses for 5-stage pipeline
        if pipeline_depth >= 5 {
            // EX stage can bypass to ID stage
            bypasses[2].push(1);
            
            // MEM stage can bypass to ID stage
            bypasses[3].push(1);
            
            // WB stage can bypass to ID stage
            bypasses[4].push(1);
        }
        
        Self { bypasses }
    }
    
    /// Check if bypass exists
    pub fn has_bypass(&self, from_stage: usize, to_stage: usize) -> bool {
        if from_stage < self.bypasses.len() {
            self.bypasses[from_stage].contains(&to_stage)
        } else {
            false
        }
    }
    
    /// Add custom bypass path
    pub fn add_bypass(&mut self, from_stage: usize, to_stage: usize) {
        if from_stage < self.bypasses.len() {
            if !self.bypasses[from_stage].contains(&to_stage) {
                self.bypasses[from_stage].push(to_stage);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_forwarding_unit_creation() {
        let mut unit = ForwardingUnit::new(5);
        unit.enable_standard_paths();
        
        assert!(!unit.paths.is_empty());
    }
    
    #[test]
    fn test_can_forward() {
        let mut unit = ForwardingUnit::new(5);
        unit.enable_standard_paths();
        
        let producer = InstructionDependency {
            id: 0,
            reads: vec![],
            writes: vec![Register(1)],
            stage: 2, // EX stage
        };
        
        let consumer = InstructionDependency {
            id: 1,
            reads: vec![Register(1)],
            writes: vec![],
            stage: 2, // EX stage
        };
        
        assert!(unit.can_forward(&producer, &consumer, Register(1)));
    }
    
    #[test]
    fn test_forwarding_resolution() {
        let mut unit = ForwardingUnit::new(5);
        unit.enable_standard_paths();
        
        let producer = InstructionDependency {
            id: 0,
            reads: vec![],
            writes: vec![Register(1)],
            stage: 2,
        };
        
        let consumer = InstructionDependency {
            id: 1,
            reads: vec![Register(1)],
            writes: vec![],
            stage: 2,
        };
        
        let resolution = unit.resolve_hazard(&producer, &consumer, Register(1));
        
        assert!(resolution.can_forward);
        assert_eq!(resolution.stall_cycles, 0);
    }
    
    #[test]
    fn test_forwarding_network() {
        let mut network = ForwardingNetwork::new(5);
        
        let path = ForwardingPath {
            from_stage: 2,
            to_stage: 2,
            register: Register(1),
        };
        
        network.register_forward(path);
        
        assert!(network.is_data_available(Register(1), 2));
        assert!(network.is_data_available(Register(1), 3));
    }
    
    #[test]
    fn test_bypass_network() {
        let network = BypassNetwork::new(5);
        
        // Standard bypasses should exist
        assert!(network.has_bypass(2, 1)); // EX->ID
        assert!(network.has_bypass(3, 1)); // MEM->ID
        assert!(network.has_bypass(4, 1)); // WB->ID
    }
}
