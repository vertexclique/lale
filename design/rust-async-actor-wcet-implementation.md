# LALE Rust-Only Async Actor WCET Analysis: Implementation Plan

## Executive Summary

This document provides a complete implementation plan for extending LALE with async actor and multi-core WCET analysis capabilities, implemented entirely in Rust. The implementation leverages LALE's existing infrastructure while adding new modules for async detection, segment extraction, and multi-core schedulability analysis.

**Key Objectives:**
1. Detect Rust async functions in LLVM IR via pattern matching
2. Extract execution segments between await points
3. Calculate per-segment WCET using existing LALE infrastructure
4. Compose actor-level WCET from segment WCETs
5. Perform multi-core schedulability analysis with RMA/EDF

**Timeline:** 15 weeks (3.5 months)

**Language:** Pure Rust (no C++ modifications)

---

## 1. Architecture Overview

### 1.1 Module Structure

```
lale/src/
├── lib.rs                          # Public API exports (to be extended)
├── async_analysis/                 # NEW: Async actor analysis
│   ├── mod.rs                      # Module exports
│   ├── detector.rs                 # Async function detection
│   ├── segment.rs                  # Segment extraction
│   ├── actor.rs                    # Actor model and composition
│   └── wcet.rs                     # Per-segment WCET analysis
├── multicore/                      # NEW: Multi-core support
│   ├── mod.rs                      # Module exports
│   ├── partitioning.rs             # Actor-to-core assignment
│   ├── interference.rs             # Shared resource analysis
│   └── schedulability.rs           # Multi-core RTA
├── ir/                             # EXISTING: LLVM IR parsing
├── analysis/                       # EXISTING: WCET analysis
├── scheduling/                     # EXISTING: Scheduling (extend)
├── platform/                       # EXISTING: Platform models
└── wcet/                           # EXISTING: WCET calculator
```

### 1.2 Data Flow

```
LLVM IR Input
    ↓
AsyncDetector::detect() → AsyncFunctionInfo
    ↓
SegmentExtractor::extract_segments() → Vec<ActorSegment>
    ↓
SegmentWCETAnalyzer::analyze_segments() → HashMap<SegmentID, WCET>
    ↓
Actor::compute_actor_wcet() → Actor with WCET
    ↓
MultiCoreScheduler::analyze() → SchedulabilityResult
    ↓
Output (JSON/BTF)
```

---

## 2. Public API Design

### 2.1 Exports from `lale/src/lib.rs`

```rust
// Existing exports
pub mod ir;
pub mod analysis;
pub mod scheduling;
pub mod platform;
pub mod wcet;
pub mod output;

// NEW: Async analysis exports
pub mod async_analysis;
pub use async_analysis::{
    // Core detection
    AsyncDetector,
    AsyncFunctionInfo,
    StateBlock,
    
    // Segment extraction
    SegmentExtractor,
    ActorSegment,
    
    // Actor model
    Actor,
    ActorSystem,
    ActorConfig,
    
    // WCET analysis
    SegmentWCETAnalyzer,
    SegmentWCET,
    ActorWCETResult,
};

// NEW: Multi-core support exports
pub mod multicore;
pub use multicore::{
    // Core assignment
    CoreAssignment,
    PartitioningStrategy,
    
    // Schedulability
    MultiCoreScheduler,
    MultiCoreResult,
    CoreSchedulabilityResult,
    
    // Interference
    InterferenceAnalyzer,
    InterferenceModel,
    SharedResourceAccess,
};

// NEW: High-level analysis API
pub mod actor_analysis;
pub use actor_analysis::{
    ActorAnalyzer,
    ActorAnalysisConfig,
    ActorAnalysisResult,
};
```

### 2.2 High-Level API Usage

```rust
use lale::{
    ActorAnalyzer,
    ActorAnalysisConfig,
    platform::CortexM7Model,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure analysis
    let config = ActorAnalysisConfig {
        platform: CortexM7Model::new(),
        num_cores: 2,
        scheduling_policy: SchedulingPolicy::RMA,
        enable_cache_analysis: true,
    };
    
    // Create analyzer
    let analyzer = ActorAnalyzer::new(config);
    
    // Analyze LLVM IR files
    let result = analyzer.analyze_directory("target/release/deps/")?;
    
    // Check schedulability
    if result.is_schedulable() {
        println!("System is schedulable!");
        println!("Utilization: {:.2}%", result.utilization() * 100.0);
    } else {
        eprintln!("System is NOT schedulable!");
        for violation in result.violations() {
            eprintln!("  Actor '{}' misses deadline by {} us",
                violation.actor_name, violation.slack_us);
        }
    }
    
    // Export results
    result.export_json("analysis_report.json")?;
    result.export_btf("trace.btf")?;
    
    Ok(())
}
```

---

## 3. Module Implementation Details

### 3.1 Async Detection Module

**File:** `lale/src/async_analysis/detector.rs`

```rust
use llvm_ir::{Function, Instruction, BasicBlock, Terminator};
use ahash::AHashSet;
use serde::{Serialize, Deserialize};

/// Async function detector using LLVM IR pattern matching
pub struct AsyncDetector;

/// Information about detected async function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncFunctionInfo {
    /// Function name
    pub function_name: String,
    
    /// Whether function is async
    pub is_async: bool,
    
    /// Confidence score (0-10)
    pub confidence_score: u8,
    
    /// Pointer to state discriminant field
    pub state_discriminant_ptr: Option<String>,
    
    /// Detected state blocks
    pub state_blocks: Vec<StateBlock>,
    
    /// Detection method used
    pub detection_method: DetectionMethod,
}

/// State block in async state machine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateBlock {
    /// State ID (discriminant value)
    pub state_id: u32,
    
    /// Entry basic block name
    pub entry_block: String,
    
    /// All reachable blocks in this state
    pub reachable_blocks: Vec<String>,
}

/// Detection method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionMethod {
    /// Detected via generator type names
    GeneratorType,
    
    /// Detected via discriminant switch pattern
    DiscriminantSwitch,
    
    /// Detected via function signature
    AsyncSignature,
    
    /// Multiple detection methods
    Combined(Vec<DetectionMethod>),
}

impl AsyncDetector {
    /// Detect if function is Rust async
    pub fn detect(function: &Function) -> AsyncFunctionInfo {
        let mut confidence = 0u8;
        let mut methods = Vec::new();
        let func_name = function.name.to_string();
        
        // Pattern 1: Generator type detection
        if Self::has_generator_type(function) {
            confidence += 4;
            methods.push(DetectionMethod::GeneratorType);
        }
        
        // Pattern 2: Discriminant switch detection (strongest indicator)
        if let Some((disc_ptr, states)) = Self::find_discriminant_switch(function) {
            confidence += 5;
            methods.push(DetectionMethod::DiscriminantSwitch);
            
            return AsyncFunctionInfo {
                function_name: func_name,
                is_async: true,
                confidence_score: confidence,
                state_discriminant_ptr: Some(disc_ptr),
                state_blocks: states,
                detection_method: if methods.len() > 1 {
                    DetectionMethod::Combined(methods)
                } else {
                    DetectionMethod::DiscriminantSwitch
                },
            };
        }
        
        // Pattern 3: Async signature
        if Self::has_async_signature(function) {
            confidence += 2;
            methods.push(DetectionMethod::AsyncSignature);
        }
        
        AsyncFunctionInfo {
            function_name: func_name,
            is_async: confidence >= 6,
            confidence_score: confidence,
            state_discriminant_ptr: None,
            state_blocks: vec![],
            detection_method: if methods.len() > 1 {
                DetectionMethod::Combined(methods)
            } else if !methods.is_empty() {
                methods[0].clone()
            } else {
                DetectionMethod::AsyncSignature
            },
        }
    }
    
    /// Detect all async functions in module
    pub fn detect_all(module: &llvm_ir::Module) -> Vec<AsyncFunctionInfo> {
        module.functions
            .iter()
            .map(|f| Self::detect(f))
            .filter(|info| info.is_async)
            .collect()
    }
    
    /// Check if function references generator types
    fn has_generator_type(function: &Function) -> bool {
        let func_str = format!("{:?}", function);
        func_str.contains("generator@") || 
        func_str.contains("{{closure}}") ||
        func_str.contains("async_fn") ||
        func_str.contains("{async_fn_env")
    }
    
    /// Find discriminant switch pattern in entry block
    fn find_discriminant_switch(
        function: &Function
    ) -> Option<(String, Vec<StateBlock>)> {
        let entry_bb = function.basic_blocks.first()?;
        
        // Look for load → switch pattern
        for (idx, instr) in entry_bb.instrs.iter().enumerate() {
            if let Instruction::Load(load) = instr {
                // Check if load is of small integer (state discriminant)
                let load_type = format!("{:?}", load.dest);
                if load_type.contains("i8") || load_type.contains("i32") {
                    // Check terminator for switch
                    if let Terminator::Switch(switch) = &entry_bb.term {
                        let states = Self::extract_state_blocks(switch, function);
                        
                        // Valid state machine has at least 2 states
                        if states.len() >= 2 {
                            let disc_ptr = format!("{:?}", load.address);
                            return Some((disc_ptr, states));
                        }
                    }
                }
            }
        }
        None
    }
    
    /// Extract state blocks from switch instruction
    fn extract_state_blocks(
        switch: &llvm_ir::terminator::Switch,
        function: &Function
    ) -> Vec<StateBlock> {
        let mut states = Vec::new();
        
        // Extract each switch case as a state
        for (idx, (value, dest)) in switch.dests.iter().enumerate() {
            let state_id = match value {
                llvm_ir::Constant::Int { value, .. } => *value as u32,
                _ => idx as u32,
            };
            
            let entry_block = dest.to_string();
            
            states.push(StateBlock {
                state_id,
                entry_block: entry_block.clone(),
                reachable_blocks: vec![entry_block],
            });
        }
        
        // Sort by state ID
        states.sort_by_key(|s| s.state_id);
        states
    }
    
    /// Check for async function signature pattern
    fn has_async_signature(function: &Function) -> bool {
        // Async functions typically have signature:
        // (ptr, ptr) -> i1  (generator, context) -> bool
        let params = &function.parameters;
        let has_two_ptr_params = params.len() == 2;
        let returns_bool = function.return_type.to_string().contains("i1");
        
        has_two_ptr_params && returns_bool
    }
}
```

### 3.2 Segment Extraction Module

**File:** `lale/src/async_analysis/segment.rs`

```rust
use llvm_ir::{Function, BasicBlock, Instruction, Terminator};
use crate::async_analysis::detector::{AsyncFunctionInfo, StateBlock};
use ahash::{AHashMap, AHashSet};
use serde::{Serialize, Deserialize};

/// Actor execution segment (code between await points)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSegment {
    /// Segment ID (corresponds to state)
    pub segment_id: u32,
    
    /// Entry basic block
    pub entry_block: String,
    
    /// All basic blocks in segment
    pub blocks: Vec<String>,
    
    /// Exit blocks (update state or return)
    pub exit_blocks: Vec<String>,
    
    /// Next possible segments
    pub next_segments: Vec<u32>,
    
    /// Segment type
    pub segment_type: SegmentType,
}

/// Type of segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SegmentType {
    /// Initial segment (state 0)
    Initial,
    
    /// Resume after await
    Suspended,
    
    /// Final segment (returns)
    Completion,
}

/// Segment extractor
pub struct SegmentExtractor;

impl SegmentExtractor {
    /// Extract execution segments from async function
    pub fn extract_segments(
        function: &Function,
        async_info: &AsyncFunctionInfo,
    ) -> Vec<ActorSegment> {
        if !async_info.is_async || async_info.state_blocks.is_empty() {
            return vec![];
        }
        
        let mut segments = Vec::new();
        
        for (idx, state_block) in async_info.state_blocks.iter().enumerate() {
            let segment_type = match state_block.state_id {
                0 => SegmentType::Initial,
                1 => SegmentType::Completion,
                _ => SegmentType::Suspended,
            };
            
            let segment = Self::build_segment(
                function,
                state_block,
                &async_info.state_discriminant_ptr,
                segment_type,
            );
            
            segments.push(segment);
        }
        
        segments
    }
    
    /// Build segment by exploring reachable blocks
    fn build_segment(
        function: &Function,
        state_block: &StateBlock,
        disc_ptr: &Option<String>,
        segment_type: SegmentType,
    ) -> ActorSegment {
        let mut visited = AHashSet::new();
        let mut blocks = Vec::new();
        let mut exit_blocks = Vec::new();
        let mut next_segments = Vec::new();
        
        // BFS from entry block
        let mut queue = vec![state_block.entry_block.clone()];
        
        while let Some(bb_name) = queue.pop() {
            if visited.contains(&bb_name) {
                continue;
            }
            visited.insert(bb_name.clone());
            blocks.push(bb_name.clone());
            
            // Find basic block in function
            let bb = match Self::find_block(function, &bb_name) {
                Some(b) => b,
                None => continue,
            };
            
            // Check if this block updates state (segment boundary)
            if Self::updates_state(bb, disc_ptr) {
                exit_blocks.push(bb_name.clone());
                
                // Extract next state value
                if let Some(next_state) = Self::extract_next_state(bb) {
                    next_segments.push(next_state);
                }
                continue;
            }
            
            // Check if this block returns (segment end)
            if Self::is_return_block(bb) {
                exit_blocks.push(bb_name);
                continue;
            }
            
            // Add successors to queue
            for succ in Self::get_successors(bb) {
                if !visited.contains(&succ) {
                    queue.push(succ);
                }
            }
        }
        
        ActorSegment {
            segment_id: state_block.state_id,
            entry_block: state_block.entry_block.clone(),
            blocks,
            exit_blocks,
            next_segments,
            segment_type,
        }
    }
    
    /// Find basic block by name
    fn find_block<'a>(function: &'a Function, name: &str) -> Option<&'a BasicBlock> {
        function.basic_blocks.iter()
            .find(|bb| bb.name.as_ref().map(|n| n.as_str()) == Some(name))
    }
    
    /// Check if block updates state discriminant
    fn updates_state(bb: &BasicBlock, disc_ptr: &Option<String>) -> bool {
        if let Some(ptr) = disc_ptr {
            for instr in &bb.instrs {
                if let Instruction::Store(store) = instr {
                    let dest = format!("{:?}", store.address);
                    if dest.contains(ptr) {
                        return true;
                    }
                }
            }
        }
        false
    }
    
    /// Extract next state value from store instruction
    fn extract_next_state(bb: &BasicBlock) -> Option<u32> {
        for instr in &bb.instrs {
            if let Instruction::Store(store) = instr {
                if let llvm_ir::Operand::ConstantOperand(c) = &store.value {
                    if let llvm_ir::Constant::Int { value, .. } = c.as_ref() {
                        return Some(*value as u32);
                    }
                }
            }
        }
        None
    }
    
    /// Check if block is a return block
    fn is_return_block(bb: &BasicBlock) -> bool {
        matches!(bb.term, Terminator::Ret(_))
    }
    
    /// Get successor blocks
    fn get_successors(bb: &BasicBlock) -> Vec<String> {
        match &bb.term {
            Terminator::Br(br) => vec![br.dest.to_string()],
            Terminator::CondBr(cbr) => vec![
                cbr.true_dest.to_string(),
                cbr.false_dest.to_string(),
            ],
            Terminator::Switch(sw) => sw.dests.iter()
                .map(|(_, dest)| dest.to_string())
                .collect(),
            _ => vec![],
        }
    }
}
```

### 3.3 Actor Model Module

**File:** `lale/src/async_analysis/actor.rs`

```rust
use serde::{Serialize, Deserialize};
use ahash::AHashMap;
use crate::async_analysis::segment::ActorSegment;
use crate::scheduling::Task;

/// Actor in Veecle OS system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    /// Actor name
    pub name: String,
    
    /// LLVM function name
    pub function: String,
    
    /// Priority (higher = more important)
    pub priority: u8,
    
    /// Deadline in microseconds
    pub deadline_us: f64,
    
    /// Period in microseconds (None = aperiodic)
    pub period_us: Option<f64>,
    
    /// Core affinity (None = any core)
    pub core_affinity: Option<usize>,
    
    /// Execution segments
    pub segments: Vec<ActorSegment>,
    
    /// Per-segment WCET in cycles
    pub segment_wcets: AHashMap<u32, u64>,
    
    /// Actor-level WCET in cycles
    pub actor_wcet_cycles: u64,
    
    /// Actor-level WCET in microseconds
    pub actor_wcet_us: f64,
}

impl Actor {
    /// Create new actor
    pub fn new(
        name: String,
        function: String,
        priority: u8,
        deadline_us: f64,
        period_us: Option<f64>,
        core_affinity: Option<usize>,
    ) -> Self {
        Self {
            name,
            function,
            priority,
            deadline_us,
            period_us,
            core_affinity,
            segments: vec![],
            segment_wcets: AHashMap::new(),
            actor_wcet_cycles: 0,
            actor_wcet_us: 0.0,
        }
    }
    
    /// Compute actor-level WCET from segment WCETs
    pub fn compute_actor_wcet(&mut self, cpu_freq_mhz: u32) {
        // Strategy: Maximum segment WCET (conservative)
        self.actor_wcet_cycles = self.segment_wcets.values()
            .copied()
            .max()
            .unwrap_or(0);
        
        self.actor_wcet_us = self.actor_wcet_cycles as f64 / cpu_freq_mhz as f64;
    }
    
    /// Convert to schedulable task
    pub fn to_task(&self) -> Task {
        Task {
            name: self.name.clone(),
            function: self.function.clone(),
            wcet_cycles: self.actor_wcet_cycles,
            wcet_us: self.actor_wcet_us,
            period_us: self.period_us,
            deadline_us: Some(self.deadline_us),
            priority: Some(self.priority),
            preemptible: false, // Cooperative scheduling
            dependencies: vec![],
        }
    }
    
    /// Get utilization (WCET / Period)
    pub fn utilization(&self) -> f64 {
        if let Some(period) = self.period_us {
            self.actor_wcet_us / period
        } else {
            0.0
        }
    }
}

/// Actor system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorSystem {
    /// System name
    pub name: String,
    
    /// All actors
    pub actors: Vec<Actor>,
    
    /// Platform name
    pub platform: String,
    
    /// Number of cores
    pub num_cores: usize,
    
    /// CPU frequency in MHz
    pub cpu_freq_mhz: u32,
}

impl ActorSystem {
    /// Create new actor system
    pub fn new(
        name: String,
        platform: String,
        num_cores: usize,
        cpu_freq_mhz: u32,
    ) -> Self {
        Self {
            name,
            actors: vec![],
            platform,
            num_cores,
            cpu_freq_mhz,
        }
    }
    
    /// Add actor to system
    pub fn add_actor(&mut self, actor: Actor) {
        self.actors.push(actor);
    }
    
    /// Get total system utilization
    pub fn total_utilization(&self) -> f64 {
        self.actors.iter()
            .map(|a| a.utilization())
            .sum()
    }
    
    /// Get actors assigned to specific core
    pub fn actors_on_core(&self, core_id: usize) -> Vec<&Actor> {
        self.actors.iter()
            .filter(|a| a.core_affinity == Some(core_id))
            .collect()
    }
}

/// Actor configuration from external file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorConfig {
    pub name: String,
    pub function: String,
    pub priority: u8,
    pub deadline_ms: f64,
    pub period_ms: Option<f64>,
    pub core_affinity: Option<usize>,
}

impl ActorConfig {
    /// Convert to Actor (without WCET data)
    pub fn to_actor(&self) -> Actor {
        Actor::new(
            self.name.clone(),
            self.function.clone(),
            self.priority,
            self.deadline_ms * 1000.0, // ms to us
            self.period_ms.map(|p| p * 1000.0),
            self.core_affinity,
        )
    }
}
```

### 3.4 Multi-Core Schedulability Module

**File:** `lale/src/multicore/schedulability.rs`

```rust
use crate::async_analysis::Actor;
use crate::scheduling::{RMAScheduler, EDFScheduler, SchedulabilityResult};
use serde::{Serialize, Deserialize};

/// Multi-core scheduler
pub struct MultiCoreScheduler {
    pub num_cores: usize,
    pub policy: SchedulingPolicy,
}

/// Scheduling policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SchedulingPolicy {
    RMA,  // Rate Monotonic Analysis
    EDF,  // Earliest Deadline First
}

/// Multi-core schedulability result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiCoreResult {
    /// Per-core results
    pub per_core: Vec<CoreSchedulabilityResult>,
    
    /// Overall schedulability
    pub overall_schedulable: bool,
    
    /// Total system utilization
    pub total_utilization: f64,
    
    /// Per-core utilization
    pub core_utilizations: Vec<f64>,
}

/// Per-core schedulability result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreSchedulabilityResult {
    pub core_id: usize,
    pub schedulable: bool,
    pub utilization: f64,
    pub actors: Vec<String>,
    pub violations: Vec<DeadlineViolation>,
}

/// Deadline violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadlineViolation {
    pub actor_name: String,
    pub response_time_us: f64,
    pub deadline_us: f64,
    pub slack_us: f64,
}

impl MultiCoreScheduler {
    /// Create new multi-core scheduler
    pub fn new(num_cores: usize, policy: SchedulingPolicy) -> Self {
        Self { num_cores, policy }
    }
    
    /// Analyze schedulability for actor system
    pub fn analyze(&self, actors: &[Actor]) -> MultiCoreResult {
        // Partition actors by core affinity
        let partitions = self.partition_actors(actors);
        
        // Analyze each core independently
        let mut per_core = Vec::new();
        let mut overall_schedulable = true;
        let mut core_utilizations = Vec::new();
        
        for core_id in 0..self.num_cores {
            let core_actors = partitions.get(&core_id).cloned().unwrap_or_default();
            
            let result = self.analyze_core(core_id, &core_actors);
            
            overall_schedulable &= result.schedulable;
            core_utilizations.push(result.utilization);
            per_core.push(result);
        }
        
        let total_utilization = actors.iter()
            .map(|a| a.utilization())
            .sum();
        
        MultiCoreResult {
            per_core,
            overall_schedulable,
            total_utilization,
            core_utilizations,
        }
    }
    
    /// Partition actors by core affinity
    fn partition_actors(&self, actors: &[Actor]) -> ahash::AHashMap<usize, Vec<&Actor>> {
        let mut partitions = ahash::AHashMap::new();
        
        for actor in actors {
            let core = actor.core_affinity.unwrap_or(0);
            partitions.entry(core).or_insert_with(Vec::new).push(actor);
        }
        
        partitions
    }
    
    /// Analyze single core
    fn analyze_core(&self, core_id: usize, actors: &[&Actor]) -> CoreSchedulabilityResult {
        if actors.is_empty() {
            return CoreSchedulabilityResult {
                core_id,
                schedulable: true,
                utilization: 0.0,
                actors: vec![],
                violations: vec![],
            };
        }
        
        // Convert actors to tasks
        let tasks: Vec<_> = actors.iter().map(|a| a.to_task()).collect();
        
        // Perform schedulability analysis
        let result = match self.policy {
            SchedulingPolicy::RMA => RMAScheduler::analyze(&tasks),
            SchedulingPolicy::EDF => EDFScheduler::analyze(&tasks),
        };
        
        // Extract violations
        let mut violations = Vec::new();
        for (actor, task_result) in actors.iter().zip(result.task_results.iter()) {
            if !task_result.schedulable {
                violations.push(DeadlineViolation {
                    actor_name: actor.name.clone(),
                    response_time_us: task_result.response_time_us,
                    deadline_us: actor.deadline_us,
                    slack_us: actor.deadline_us - task_result.response_time_us,
                });
            }
        }
        
        let utilization = actors.iter().map(|a| a.utilization()).sum();
        
        CoreSchedulabilityResult {
            core_id,
            schedulable: result.schedulable,
            utilization,
            actors: actors.iter().map(|a| a.name.clone()).collect(),
            violations,
        }
    }
}
```

---

## 4. Public API Methods to Export

### 4.1 Core Async Analysis API

```rust
// From async_analysis module
pub struct AsyncDetector;
impl AsyncDetector {
    /// Detect if single function is async
    pub fn detect(function: &llvm_ir::Function) -> AsyncFunctionInfo;
    
    /// Detect all async functions in module
    pub fn detect_all(module: &llvm_ir::Module) -> Vec<AsyncFunctionInfo>;
}

pub struct SegmentExtractor;
impl SegmentExtractor {
    /// Extract execution segments from async function
    pub fn extract_segments(
        function: &llvm_ir::Function,
        async_info: &AsyncFunctionInfo,
    ) -> Vec<ActorSegment>;
}

pub struct SegmentWCETAnalyzer;
impl SegmentWCETAnalyzer {
    /// Create analyzer with platform model
    pub fn new(platform: PlatformModel) -> Self;
    
    /// Analyze WCET for all segments
    pub fn analyze_segments(
        &self,
        function: &llvm_ir::Function,
        segments: &[ActorSegment],
    ) -> ahash::AHashMap<u32, SegmentWCET>;
}
```

### 4.2 Actor Model API

```rust
pub struct Actor {
    // ... fields ...
}

impl Actor {
    /// Create new actor
    pub fn new(
        name: String,
        function: String,
        priority: u8,
        deadline_us: f64,
        period_us: Option<f64>,
        core_affinity: Option<usize>,
    ) -> Self;
    
    /// Compute actor-level WCET from segments
    pub fn compute_actor_wcet(&mut self, cpu_freq_mhz: u32);
    
    /// Convert to schedulable task
    pub fn to_task(&self) -> Task;
    
    /// Get utilization
    pub fn utilization(&self) -> f64;
}

pub struct ActorSystem {
    // ... fields ...
}

impl ActorSystem {
    /// Create new actor system
    pub fn new(
        name: String,
        platform: String,
        num_cores: usize,
        cpu_freq_mhz: u32,
    ) -> Self;
    
    /// Add actor to system
    pub fn add_actor(&mut self, actor: Actor);
    
    /// Get total system utilization
    pub fn total_utilization(&self) -> f64;
    
    /// Get actors on specific core
    pub fn actors_on_core(&self, core_id: usize) -> Vec<&Actor>;
}
```

### 4.3 Multi-Core Schedulability API

```rust
pub struct MultiCoreScheduler {
    pub num_cores: usize,
    pub policy: SchedulingPolicy,
}

impl MultiCoreScheduler {
    /// Create new multi-core scheduler
    pub fn new(num_cores: usize, policy: SchedulingPolicy) -> Self;
    
    /// Analyze schedulability for actor system
    pub fn analyze(&self, actors: &[Actor]) -> MultiCoreResult;
}

pub struct MultiCoreResult {
    pub per_core: Vec<CoreSchedulabilityResult>,
    pub overall_schedulable: bool,
    pub total_utilization: f64,
    pub core_utilizations: Vec<f64>,
}

impl MultiCoreResult {
    /// Check if system is schedulable
    pub fn is_schedulable(&self) -> bool;
    
    /// Get all deadline violations
    pub fn violations(&self) -> Vec<&DeadlineViolation>;
    
    /// Export to JSON
    pub fn export_json(&self, path: &str) -> Result<()>;
}
```

### 4.4 High-Level Unified API

```rust
pub struct ActorAnalyzer {
    config: ActorAnalysisConfig,
}

impl ActorAnalyzer {
    /// Create new analyzer with configuration
    pub fn new(config: ActorAnalysisConfig) -> Self;
    
    /// Analyze single LLVM IR file
    pub fn analyze_file(&self, path: &str) -> Result<ActorAnalysisResult>;
    
    /// Analyze directory of LLVM IR files
    pub fn analyze_directory(&self, dir: &str) -> Result<ActorAnalysisResult>;
    
    /// Analyze with actor configuration
    pub fn analyze_with_config(
        &self,
        ir_files: &[String],
        actor_configs: &[ActorConfig],
    ) -> Result<ActorAnalysisResult>;
}

pub struct ActorAnalysisConfig {
    pub platform: PlatformModel,
    pub num_cores: usize,
    pub scheduling_policy: SchedulingPolicy,
    pub enable_cache_analysis: bool,
    pub enable_pipeline_analysis: bool,
}

pub struct ActorAnalysisResult {
    pub actors: Vec<Actor>,
    pub schedulability: MultiCoreResult,
    pub system: ActorSystem,
}

impl ActorAnalysisResult {
    /// Check if system is schedulable
    pub fn is_schedulable(&self) -> bool;
    
    /// Get total utilization
    pub fn utilization(&self) -> f64;
    
    /// Get all violations
    pub fn violations(&self) -> Vec<&DeadlineViolation>;
    
    /// Export to JSON
    pub fn export_json(&self, path: &str) -> Result<()>;
    
    /// Export to BTF trace format
    pub fn export_btf(&self, path: &str) -> Result<()>;
    
    /// Generate HTML report
    pub fn export_html(&self, path: &str) -> Result<()>;
}
```

---

## 5. Implementation Phases

### Phase 1: Async Detection (Weeks 1-4)

**Deliverables:**
- `lale/src/async_analysis/mod.rs`
- `lale/src/async_analysis/detector.rs`
- Unit tests for detection patterns
- Integration test with sample async LLVM IR

**Tasks:**
1. Implement `AsyncDetector::detect()` with pattern matching
2. Implement `AsyncDetector::detect_all()` for module scanning
3. Add detection confidence scoring
4. Test with various Rust async patterns
5. Document detection heuristics

**Success Criteria:**
- Detect 95%+ of Rust async functions
- No false positives on sync functions
- Extract state machine structure correctly

### Phase 2: Segment Extraction (Weeks 5-7)

**Deliverables:**
- `lale/src/async_analysis/segment.rs`
- Segment boundary detection
- State transition graph construction
- Unit tests for segment extraction

**Tasks:**
1. Implement BFS-based segment exploration
2. Detect state update instructions
3. Build segment transition graph
4. Handle complex control flow
5. Test with nested async/await

**Success Criteria:**
- Correctly identify all segment boundaries
- Extract complete basic block sets per segment
- Build accurate transition graph

### Phase 3: Per-Segment WCET (Weeks 8-10)

**Deliverables:**
- `lale/src/async_analysis/wcet.rs`
- Integration with existing WCET calculator
- Per-segment timing analysis
- Segment WCET composition

**Tasks:**
1. Implement `SegmentWCETAnalyzer`
2. Build segment-restricted CFGs
3. Integrate with existing IPET solver
4. Handle segment-specific loop bounds
5. Validate timing results

**Success Criteria:**
- Accurate WCET per segment
- Leverage existing LALE infrastructure
- Handle cache/pipeline effects

### Phase 4: Actor Composition (Weeks 11-12)

**Deliverables:**
- `lale/src/async_analysis/actor.rs`
- Actor model implementation
- WCET composition strategies
- Actor configuration loading

**Tasks:**
1. Implement `Actor` struct and methods
2. Implement `ActorSystem` management
3. Add WCET composition (max segment)
4. Load actor configs from JSON/TOML
5. Convert actors to tasks

**Success Criteria:**
- Complete actor model
- Accurate actor-level WCET
- Configuration file support

### Phase 5: Multi-Core Schedulability (Weeks 13-15)

**Deliverables:**
- `lale/src/multicore/mod.rs`
- `lale/src/multicore/schedulability.rs`
- Multi-core RMA/EDF analysis
- Partitioned scheduling

**Tasks:**
1. Implement `MultiCoreScheduler`
2. Add core partitioning logic
3. Extend RMA/EDF for multi-core
4. Implement response time analysis
5. Generate schedulability reports

**Success Criteria:**
- Correct multi-core analysis
- Per-core schedulability results
- Violation detection and reporting

---

## 6. Testing Strategy

### 6.1 Unit Tests

**Per Module:**

```rust
// tests/async_detection_test.rs
#[test]
fn test_detect_simple_async() {
    let ir = include_str!("fixtures/simple_async.ll");
    let module = Module::from_ir(ir).unwrap();
    let func = &module.functions[0];
    
    let info = AsyncDetector::detect(func);
    assert!(info.is_async);
    assert!(info.confidence_score >= 8);
    assert_eq!(info.state_blocks.len(), 3);
}

#[test]
fn test_no_false_positives() {
    let ir = include_str!("fixtures/sync_function.ll");
    let module = Module::from_ir(ir).unwrap();
    let func = &module.functions[0];
    
    let info = AsyncDetector::detect(func);
    assert!(!info.is_async);
}

// tests/segment_extraction_test.rs
#[test]
fn test_extract_segments() {
    let module = load_test_module("async_with_awaits.ll");
    let func = &module.functions[0];
    let async_info = AsyncDetector::detect(func);
    
    let segments = SegmentExtractor::extract_segments(func, &async_info);
    assert_eq!(segments.len(), 3);
    assert_eq!(segments[0].segment_type, SegmentType::Initial);
}

// tests/multicore_test.rs
#[test]
fn test_multicore_schedulability() {
    let actors = create_test_actors();
    let scheduler = MultiCoreScheduler::new(2, SchedulingPolicy::RMA);
    
    let result = scheduler.analyze(&actors);
    assert!(result.overall_schedulable);
    assert_eq!(result.per_core.len(), 2);
}
```

### 6.2 Integration Tests

**End-to-End Workflow:**

```rust
// tests/integration_test.rs
#[test]
fn test_full_actor_analysis() {
    // 1. Parse LLVM IR
    let module = IRParser::parse_file("tests/fixtures/veecle_actors.ll").unwrap();
    
    // 2. Detect async functions
    let async_funcs = AsyncDetector::detect_all(&module);
    assert!(!async_funcs.is_empty());
    
    // 3. Extract segments
    let func = &module.functions[0];
    let segments = SegmentExtractor::extract_segments(func, &async_funcs[0]);
    
    // 4. Analyze WCET
    let platform = CortexM7Model::new();
    let analyzer = SegmentWCETAnalyzer::new(platform);
    let wcets = analyzer.analyze_segments(func, &segments);
    
    // 5. Create actor
    let mut actor = Actor::new(
        "test_actor".to_string(),
        func.name.to_string(),
        10,
        100000.0,
        Some(50000.0),
        Some(0),
    );
    actor.segments = segments;
    actor.segment_wcets = wcets.into_iter()
        .map(|(id, w)| (id, w.wcet_cycles))
        .collect();
    actor.compute_actor_wcet(216);
    
    // 6. Check schedulability
    let scheduler = MultiCoreScheduler::new(1, SchedulingPolicy::RMA);
    let result = scheduler.analyze(&[actor]);
    
    assert!(result.overall_schedulable);
}
```

### 6.3 Validation Tests

**Against Known Benchmarks:**

```rust
#[test]
fn test_taclebench_compatibility() {
    // Use TACLEBench programs compiled to async
    // Validate WCET results against known bounds
}

#[test]
fn test_veecle_os_examples() {
    // Test with actual Veecle OS actor examples
    // Verify detection and analysis correctness
}
```

---

## 7. Documentation Requirements

### 7.1 API Documentation

**Rustdoc for all public APIs:**

```rust
/// Detects Rust async functions in LLVM IR using pattern matching.
///
/// # Detection Methods
///
/// 1. **Generator Type Detection**: Looks for `[static generator@` in type names
/// 2. **Discriminant Switch**: Identifies state machine switch patterns
/// 3. **Async Signature**: Checks for `(ptr, ptr) -> i1` signature
///
/// # Examples
///
/// ```rust
/// use lale::{AsyncDetector, IRParser};
///
/// let module = IRParser::parse_file("actor.ll")?;
/// let func = &module.functions[0];
/// let info = AsyncDetector::detect(func);
///
/// if info.is_async {
///     println!("Detected async function with {} states", info.state_blocks.len());
/// }
/// ```
pub struct AsyncDetector;
```

### 7.2 User Guide

**Create:** `docs/async-actor-analysis.md`

Topics:
- How async detection works
- Segment extraction explained
- Actor configuration format
- Multi-core analysis guide
- Troubleshooting common issues

### 7.3 Examples

**Create:** `examples/veecle_actor_analysis.rs`

```rust
//! Example: Analyzing Veecle OS actors for WCET and schedulability
//!
//! This example demonstrates the complete workflow for analyzing
//! async actors in a Veecle OS system.

use lale::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load actor configurations
    let actor_configs = load_actor_configs("actors.toml")?;
    
    // Configure analysis
    let config = ActorAnalysisConfig {
        platform: platform::CortexM7Model::new(),
        num_cores: 2,
        scheduling_policy: SchedulingPolicy::RMA,
        enable_cache_analysis: true,
        enable_pipeline_analysis: true,
    };
    
    // Create analyzer
    let analyzer = ActorAnalyzer::new(config);
    
    // Analyze LLVM IR files
    let ir_files = vec![
        "target/release/deps/sensor_actor.ll",
        "target/release/deps/control_actor.ll",
    ];
    
    let result = analyzer.analyze_with_config(&ir_files, &actor_configs)?;
    
    // Print results
    println!("=== Actor Analysis Results ===\n");
    
    for actor in &result.actors {
        println!("Actor: {}", actor.name);
        println!("  WCET: {:.2} us", actor.actor_wcet_us);
        println!("  Deadline: {:.2} us", actor.deadline_us);
        println!("  Utilization: {:.2}%", actor.utilization() * 100.0);
        println!("  Segments: {}", actor.segments.len());
        println!();
    }
    
    println!("=== Schedulability ===\n");
    println!("Overall: {}", if result.is_schedulable() { "✓ PASS" } else { "✗ FAIL" });
    println!("Total Utilization: {:.2}%", result.utilization() * 100.0);
    
    for core_result in &result.schedulability.per_core {
        println!("\nCore {}: {}", 
            core_result.core_id,
            if core_result.schedulable { "✓" } else { "✗" }
        );
        println!("  Utilization: {:.2}%", core_result.utilization * 100.0);
        println!("  Actors: {}", core_result.actors.join(", "));
        
        if !core_result.violations.is_empty() {
            println!("  Violations:");
            for v in &core_result.violations {
                println!("    - {}: exceeds deadline by {:.2} us",
                    v.actor_name, -v.slack_us);
            }
        }
    }
    
    // Export results
    result.export_json("analysis_report.json")?;
    result.export_btf("trace.btf")?;
    result.export_html("report.html")?;
    
    println!("\n✓ Results exported");
    
    Ok(())
}
```

---

## 8. Configuration File Formats

### 8.1 Actor Configuration via Veecle Metamodel (TOML)

LALE integrates with Veecle's metamodel format. Actor timing constraints are specified as extensions to the standard Veecle metamodel.

**File:** `system.toml` (Veecle metamodel with LALE extensions)

```toml
[metadata]
name = "automotive_control_system"
version = "1.0.0"
description = "Real-time automotive control system with WCET analysis"
veecle_os_path = "../veecle-os"

# Dataflow interfaces
[interfaces.dataflow.SensorData]
input = { raw = "SensorReading" }
output = { filtered = "FilteredData" }
description = "Sensor data processing interface"

[interfaces.dataflow.ControlCommand]
input = { data = "FilteredData" }
output = { command = "ActuatorCommand" }
description = "Control command generation interface"

# Service with actors
[services.ControlService]
implements = ["dataflow.SensorData", "dataflow.ControlCommand"]
description = "Main control service"

# Actor instances with LALE timing annotations
[services.ControlService.actors.sensor_actor]
path = "sensor_processing::SensorProcessor"

# LALE timing constraints (extension)
[services.ControlService.actors.sensor_actor.lale]
priority = 10
deadline_ms = 100.0
period_ms = 50.0
core_affinity = 0

[services.ControlService.actors.control_actor]
path = "control_logic::ControlLogic"

[services.ControlService.actors.control_actor.lale]
priority = 20
deadline_ms = 50.0
period_ms = 50.0
core_affinity = 0

[services.ControlService.actors.actuator_actor]
path = "actuator_driver::ActuatorDriver"

[services.ControlService.actors.actuator_actor.lale]
priority = 15
deadline_ms = 75.0
period_ms = 100.0
core_affinity = 1

# Deployment with platform specification
[deployment.embedded_controller]
services = ["ControlService"]
platform = { os = "freertos", arch = "arm-cortex-m7", mcu = "stm32f767zi" }
description = "Embedded real-time controller"

# LALE platform configuration (extension)
[deployment.embedded_controller.lale]
cpu_freq_mhz = 216
num_cores = 1
scheduling_policy = "RMA"
enable_cache_analysis = true
```

### 8.2 Alternative: Standalone LALE Configuration

For systems without full Veecle metamodel, LALE supports a simplified format:

**File:** `lale_actors.toml`

```toml
[system]
name = "control_system"
platform = "cortex-m7"
cpu_freq_mhz = 216
num_cores = 1

[[actors]]
name = "sensor_actor"
path = "sensor_processing::SensorProcessor"
priority = 10
deadline_ms = 100.0
period_ms = 50.0
core_affinity = 0

[[actors]]
name = "control_actor"
path = "control_logic::ControlLogic"
priority = 20
deadline_ms = 50.0
period_ms = 50.0
core_affinity = 0

[[actors]]
name = "actuator_actor"
path = "actuator_driver::ActuatorDriver"
priority = 15
deadline_ms = 75.0
period_ms = 100.0
core_affinity = 1
```

### 8.3 Analysis Configuration (TOML)

**File:** `lale_config.toml`

```toml
[platform]
name = "cortex-m7"
cpu_freq_mhz = 216
num_cores = 2

[platform.cache]
i_cache_kb = 16
d_cache_kb = 16
line_size = 32

[analysis]
scheduling_policy = "RMA"  # or "EDF"
enable_cache_analysis = true
enable_pipeline_analysis = true
enable_aeg = false  # Advanced: Abstract Execution Graph

[output]
export_json = true
export_btf = true
export_html = true
output_dir = "analysis_results"
```

---

## 9. Performance Considerations

### 9.1 Optimization Strategies

**Caching:**
- Cache parsed LLVM IR modules
- Cache segment extraction results
- Cache WCET calculations per segment

**Parallelization:**
- Analyze multiple actors in parallel
- Per-core analysis can run concurrently
- Segment WCET analysis parallelizable

**Memory Management:**
- Use `Arc` for shared data structures
- Avoid unnecessary clones
- Stream large IR files

### 9.2 Scalability Targets

- **Small systems:** 10-20 actors, < 1 second analysis
- **Medium systems:** 50-100 actors, < 10 seconds
- **Large systems:** 200+ actors, < 60 seconds

---

## 10. Future Enhancements

### 10.1 Advanced Features (Post-MVP)

1. **Abstract Execution Graph (AEG)**
   - Cycle-accurate microarchitectural simulation
   - Cache state tracking (must/may analysis)
   - Pipeline state modeling

2. **Automatic Loop Bound Inference**
   - Scalar evolution analysis
   - Pattern matching for common loops
   - User annotation support

3. **Communication Analysis**
   - Reader/Writer dependency tracking
   - Channel timing overhead
   - End-to-end latency analysis

4. **Dynamic Priority Assignment**
   - Optimal priority assignment algorithms
   - Deadline monotonic analysis
   - Audsley's algorithm

5. **BTF Trace Generation**
   - Worst-case schedule simulation
   - Event trace export
   - chronVIEW integration

### 10.2 Tool Integration

1. **Cargo Integration**
   - `cargo lale` subcommand
   - Build script hooks
   - CI/CD integration

2. **IDE Support**
   - VS Code extension
   - Inline WCET annotations
   - Real-time schedulability feedback

3. **Veecle OS Integration**
   - Macro-level metadata emission
   - Runtime monitoring hooks
   - Budget enforcement

---

## 11. Success Criteria

### 11.1 Functional Requirements

✅ **Async Detection:**
- Detect 95%+ of Rust async functions
- < 1% false positive rate
- Extract complete state machine structure

✅ **Segment Extraction:**
- Identify all segment boundaries correctly
- Build accurate transition graphs
- Handle complex control flow

✅ **WCET Analysis:**
- Per-segment WCET within 20% of measurement
- Leverage existing LALE infrastructure
- Support multiple platforms

✅ **Actor Composition:**
- Accurate actor-level WCET
- Configuration file support
- Task conversion

✅ **Multi-Core Schedulability:**
- Correct RMA/EDF analysis
- Per-core results
- Violation detection

### 11.2 Non-Functional Requirements

✅ **Performance:**
- Analyze 50 actors in < 10 seconds
- Memory usage < 1GB for typical systems

✅ **Usability:**
- Single command-line invocation
- Clear error messages
- Comprehensive documentation

✅ **Maintainability:**
- Pure Rust implementation
- Modular architecture
- 80%+ test coverage

---

## 12. Conclusion

This implementation plan provides a complete roadmap for adding async actor and multi-core WCET analysis to LALE using pure Rust. The design leverages existing LALE infrastructure while adding new capabilities for Veecle OS actor systems.

**Key Advantages:**
- Pure Rust (no C++ required)
- Modular and extensible
- Leverages existing WCET analysis
- Multi-core support from day one
- Industry-standard output formats

**Timeline:** 15 weeks for complete implementation

**Next Steps:**
1. Review and approve this plan
2. Set up development environment
3. Begin Phase 1: Async Detection
4. Iterate with regular testing and validation
