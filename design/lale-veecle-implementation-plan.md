# LALE Implementation Plan: Veecle OS Async Actor Support & Schedulability Analysis

## Executive Summary

**Objective**: Extend LALE (LLVM-based WCET Analysis and Scheduling) to support Veecle OS's async actor runtime, enabling customers to:
1. Analyze WCET of Rust async actors
2. Assign priorities and deadlines to actors
3. Perform multi-core schedulability analysis
4. Export results to Inchron ChronVIEW/ChronSIM for visualization

**Key Challenge**: Rust async functions compile to state machines in LLVM IR. LALE must understand these state machines to correctly model actor execution and suspension points.

---

## Part 1: Understanding Rust Async in LLVM IR

### 1.1 How Rust Compiles Async Functions

Rust's `async fn` desugars into:

```rust
// Source
async fn actor_task() -> u32 {
    let x = some_async_op().await;
    let y = another_async_op().await;
    x + y
}

// Becomes internally:
enum ActorTaskGenerator {
    State0 { /* initial */ },
    State1 { x: ..., awaiting_future: ... },
    State2 { x: ..., y: ..., awaiting_future: ... },
    State3 { x: ..., y: ... },  // final
}

impl Future for ActorTaskGenerator {
    fn poll(&mut self, cx: &mut Context) -> Poll<u32> {
        match self {
            State0 => { /* start some_async_op, transition to State1 */ }
            State1 => { /* poll future, if ready get x, start another_async_op, go to State2 */ }
            State2 => { /* poll future, if ready get y, go to State3 */ }
            State3 => { /* return Poll::Ready(x + y) */ }
        }
    }
}
```

### 1.2 LLVM IR Representation

At LLVM IR level, async functions appear as:

**Key LLVM IR Patterns to Detect:**

```llvm
; 1. Generator/Future struct with discriminant (state enum)
%generator_type = type { i32, ... }  ; i32 is discriminant/state

; 2. Poll function (the state machine)
define i8 @"poll_closure"(%generator_type* %self, %Context* %cx) {
entry:
  ; Load current state
  %state = load i32, i32* %self
  
  ; State dispatch via switch
  switch i32 %state, label %unreachable [
    i32 0, label %state0
    i32 1, label %state1
    i32 2, label %state2
    i32 3, label %state3
  ]
  
state0:
  ; First await point
  ; ... setup future ...
  store i32 1, i32* %self  ; transition to state 1
  ret i8 0  ; return Poll::Pending
  
state1:
  ; Poll first future
  ; if ready: continue to state2
  ; if pending: ret i8 0
  
  ; ... etc
}

; 3. Coroutine intrinsics (older LLVM versions)
declare token @llvm.coro.id(...)
declare i1 @llvm.coro.suspend(...)
declare void @llvm.coro.resume(...)
```

**Critical Observation**: 
- Async actors are NOT standard functions - they're state machines that suspend/resume
- Each `.await` point is a suspension point (yield)
- Between suspension points is a "run-to-completion" segment
- WCET must be calculated PER STATE, not per entire function

### 1.3 Identifying Async Functions in LLVM IR

**Heuristics:**

1. **Function signature patterns:**
   - Contains `Future` or `Generator` in mangled name
   - Returns `Poll<T>` enum (i8 or similar with 0=Pending, 1=Ready)
   - Takes `&mut Context` parameter

2. **Structural patterns:**
   - Entry block with `switch` on discriminant
   - Multiple state blocks
   - Stores to discriminant field (state transitions)
   - Early returns (yields)

3. **Metadata/Attributes:**
   - Check for `#[async]` or coroutine-related attributes
   - LLVM debug info may contain "async" or "generator" tags

---

## Part 2: Veecle OS Actor Model

### 2.1 Veecle OS Runtime Architecture

```rust
// Veecle OS Actor Pattern
#[veecle_os_runtime::actor]
async fn sensor_actor(
    mut output: Writer<'_, SensorData>,
    input: Reader<'_, Config>
) -> Infallible {
    loop {
        // Wait for input (suspension point)
        let config = input.wait_for_update().await;
        
        // Process (CPU-bound work)
        let data = process_sensor(config);
        
        // Write output (suspension point)
        output.write(data).await;
    }
}
```

**Key Properties:**
- Actors are `async fn` functions
- Communication via `Reader<T>` / `Writer<T>` (channels)
- Actors run on static executor (no heap allocation)
- Zero runtime overhead actor model
- Deterministic scheduling possible

### 2.2 Static Executor Model

Veecle OS uses a **static executor**:
- Fixed set of actors known at compile time
- No dynamic spawning
- Actors scheduled by polling futures
- Suspension points are explicit (`.await`)

**Schedulability Requirements:**
- Each actor needs WCET per scheduling segment
- Deadlines can be attached to actors
- Priorities determine scheduling order
- Multi-core: actors can be pinned to cores

---

## Part 3: LALE Current State Analysis

### 3.1 What LALE Has

✅ **Existing Capabilities:**
1. LLVM IR parsing (`ir::IRParser`)
2. CFG construction per function
3. Basic WCET calculation (IPET solver)
4. Simple platform models (Cortex-M, RISC-V)
5. Basic scheduling (RMA, EDF) for simple tasks
6. JSON/Graphviz output

### 3.2 What LALE Lacks for Veecle OS

❌ **Missing for Async Actors:**
1. **Async function detection** - no recognition of state machines
2. **State-based WCET** - calculates per-function, not per-state
3. **Suspension point identification** - no `.await` detection
4. **Multi-segment timing** - no run-to-completion segments
5. **Deadline metadata** - no way to attach deadlines to actors
6. **Actor dependency tracking** - no Reader/Writer analysis
7. **Multi-core scheduling** - single-core only
8. **ChronVIEW export** - no BTF trace format output

❌ **Missing for Precision (from llvmta-missing-components.md):**
9. **Abstract Execution Graph (AEG)** - no microarchitectural state tracking
10. **Pipeline state modeling** - no cycle-accurate simulation
11. **Cache state analysis** - only basic cache timing
12. **State joining** - no abstract state merging

---

## Part 4: Implementation Plan

### Phase 1: Async Function Detection (2-3 weeks)

**Goal**: Identify async functions and their state machines in LLVM IR.

#### 1.1 Create Async Pattern Detector

**File**: `lale/src/async_analysis.rs`

```rust
pub struct AsyncFunctionInfo {
    pub function_name: String,
    pub states: Vec<AsyncState>,
    pub initial_state: usize,
    pub suspension_points: Vec<SuspensionPoint>,
}

pub struct AsyncState {
    pub state_id: u32,
    pub discriminant: u32,
    pub basic_blocks: Vec<String>,
    pub next_states: Vec<StateTransition>,
}

pub struct SuspensionPoint {
    pub state_before: u32,
    pub state_after: u32,
    pub location: String,  // basic block
    pub await_type: AwaitType,
}

pub enum AwaitType {
    FuturePoll,
    ChannelRead,
    ChannelWrite,
    Timer,
    Unknown,
}

pub struct AsyncDetector;

impl AsyncDetector {
    /// Detect if function is async by analyzing IR patterns
    pub fn is_async_function(func: &llvm_ir::Function) -> bool {
        // 1. Check function name for Generator/Future/async markers
        if func.name.contains("Generator") 
            || func.name.contains("Future")
            || func.name.contains("async") {
            return true;
        }
        
        // 2. Check for state machine pattern
        Self::has_state_machine_pattern(func)
    }
    
    fn has_state_machine_pattern(func: &llvm_ir::Function) -> bool {
        // Look for entry block with switch on discriminant
        let entry_bb = &func.basic_blocks[0];
        
        for instr in &entry_bb.instrs {
            if let llvm_ir::Instruction::Switch(switch_instr) = instr {
                // Check if switching on small integer (state enum)
                if Self::is_state_discriminant(&switch_instr.operand) {
                    return true;
                }
            }
        }
        false
    }
    
    /// Extract state machine structure
    pub fn extract_state_machine(
        func: &llvm_ir::Function
    ) -> Result<AsyncFunctionInfo, String> {
        // 1. Find discriminant switch
        // 2. Map each case to a state
        // 3. Identify suspension points (returns with Pending)
        // 4. Build state transition graph
        
        todo!("Implement state machine extraction")
    }
}
```

#### 1.2 Integration Point

Modify `lale/src/wcet.rs`:

```rust
impl WCETAnalyzer {
    pub fn analyze_function(&self, function: &llvm_ir::Function) -> Result<u64, String> {
        // NEW: Check if async
        if AsyncDetector::is_async_function(function) {
            return self.analyze_async_function(function);
        }
        
        // Existing synchronous path
        let cfg = CFG::from_function(function);
        // ... rest of existing code
    }
    
    fn analyze_async_function(&self, function: &llvm_ir::Function) -> Result<u64, String> {
        let state_machine = AsyncDetector::extract_state_machine(function)?;
        
        // Calculate WCET per state
        let mut state_wcets = HashMap::new();
        for state in &state_machine.states {
            let wcet = self.analyze_state_segment(function, state)?;
            state_wcets.insert(state.state_id, wcet);
        }
        
        // Return max WCET of any state (worst-case scheduling segment)
        Ok(*state_wcets.values().max().unwrap_or(&0))
    }
    
    fn analyze_state_segment(
        &self, 
        function: &llvm_ir::Function,
        state: &AsyncState
    ) -> Result<u64, String> {
        // Build CFG for this state's basic blocks only
        let state_cfg = CFG::from_basic_blocks(&state.basic_blocks, function);
        
        // Calculate WCET using existing IPET solver
        let timings = TimingCalculator::calculate_block_timings(
            function, 
            &state_cfg, 
            &self.platform
        );
        IPETSolver::solve_wcet(&state_cfg, &timings, &[])
    }
}
```

**Deliverable**: 
- `async_analysis.rs` module
- Async detection integrated into main analysis pipeline
- Unit tests with synthetic async LLVM IR

---

### Phase 2: Deadline & Actor Metadata (1-2 weeks)

**Goal**: Extend LALE to support deadline and priority annotations for actors.

#### 2.1 Actor Configuration Format

**File**: `config/actors.toml`

```toml
[[actor]]
name = "sensor_actor"
function = "sensor_actor::{{closure}}"  # mangled name
priority = 10
deadline_us = 1000
period_us = 1000
core_affinity = 0  # pin to core 0

[[actor]]
name = "control_actor"
function = "control_actor::{{closure}}"
priority = 20
deadline_us = 500
period_us = 500
core_affinity = 0

# Dependencies (data flow)
[[dependency]]
from = "sensor_actor"
to = "control_actor"
channel = "sensor_data"
```

#### 2.2 Actor Model Extension

**File**: `lale/src/actor.rs`

```rust
use crate::async_analysis::AsyncFunctionInfo;
use crate::scheduling::Task;

pub struct Actor {
    pub name: String,
    pub function: String,
    pub async_info: AsyncFunctionInfo,
    pub priority: u8,
    pub deadline_us: f64,
    pub period_us: Option<f64>,
    pub core_affinity: Option<usize>,
    
    // Per-state WCET
    pub state_wcets: HashMap<u32, u64>,  // state_id -> cycles
    pub max_wcet_cycles: u64,
    pub max_wcet_us: f64,
}

impl Actor {
    /// Convert actor to schedulable task
    pub fn to_task(&self) -> Task {
        Task {
            name: self.name.clone(),
            function: self.function.clone(),
            wcet_cycles: self.max_wcet_cycles,
            wcet_us: self.max_wcet_us,
            period_us: self.period_us,
            deadline_us: Some(self.deadline_us),
            priority: Some(self.priority),
            preemptible: true,
            dependencies: vec![],
        }
    }
}

pub struct ActorSystem {
    pub actors: Vec<Actor>,
    pub dependencies: Vec<ActorDependency>,
}

pub struct ActorDependency {
    pub from_actor: String,
    pub to_actor: String,
    pub channel_type: ChannelType,
}

pub enum ChannelType {
    Reader,
    Writer,
}
```

**Deliverable**:
- Actor configuration format
- Actor metadata structures
- Integration with existing Task model

---

### Phase 3: Multi-Core Schedulability Analysis (2-3 weeks)

**Goal**: Extend scheduler to support multi-core platforms and actor pinning.

#### 3.1 Multi-Core Platform Model

**File**: `lale/src/platform.rs` (extend existing)

```rust
#[derive(Clone)]
pub struct PlatformModel {
    pub name: String,
    pub cpu_frequency_mhz: u32,
    pub cache: CacheModel,
    pub pipeline: PipelineModel,
    
    // NEW: Multi-core support
    pub num_cores: usize,
    pub cores: Vec<CoreModel>,
    pub interconnect: Option<InterconnectModel>,
}

pub struct CoreModel {
    pub core_id: usize,
    pub frequency_mhz: u32,
    pub private_cache: Option<CacheModel>,
}

pub struct InterconnectModel {
    pub topology: InterconnectTopology,
    pub latency_cycles: u32,
    pub bandwidth_gb_s: f64,
}

pub enum InterconnectTopology {
    Shared Bus,
    Crossbar,
    NoC,
}
```

#### 3.2 Partitioned Scheduling

**File**: `lale/src/scheduling/multicore.rs`

```rust
pub struct PartitionedScheduler {
    pub policy: SchedulingPolicy,
}

impl PartitionedScheduler {
    /// Perform schedulability analysis with core affinity
    pub fn analyze_schedulability(
        &self,
        actors: &[Actor],
        platform: &PlatformModel,
    ) -> MultiCoreSchedulabilityResult {
        // 1. Partition actors by core_affinity
        let partitions = self.partition_actors(actors);
        
        // 2. Analyze each core independently
        let mut per_core_results = Vec::new();
        for (core_id, core_actors) in partitions {
            let tasks: Vec<Task> = core_actors.iter()
                .map(|a| a.to_task())
                .collect();
            
            let result = match self.policy {
                SchedulingPolicy::RMA => RMAScheduler::analyze(&tasks),
                SchedulingPolicy::EDF => EDFScheduler::analyze(&tasks),
            };
            
            per_core_results.push((core_id, result));
        }
        
        // 3. Check for shared resource interference
        let interference = self.analyze_interconnect_interference(
            actors, 
            platform
        );
        
        MultiCoreSchedulabilityResult {
            per_core: per_core_results,
            interconnect_overhead: interference,
            overall_schedulable: per_core_results.iter()
                .all(|(_, r)| r.schedulable),
        }
    }
    
    fn partition_actors(&self, actors: &[Actor]) -> HashMap<usize, Vec<&Actor>> {
        let mut partitions: HashMap<usize, Vec<&Actor>> = HashMap::new();
        
        for actor in actors {
            let core = actor.core_affinity.unwrap_or(0);
            partitions.entry(core).or_default().push(actor);
        }
        
        partitions
    }
    
    fn analyze_interconnect_interference(
        &self,
        actors: &[Actor],
        platform: &PlatformModel,
    ) -> f64 {
        // Simple model: assume worst-case bus contention
        // More sophisticated: model NoC arbitration, TDMA, etc.
        
        if platform.num_cores == 1 {
            return 0.0;  // no interference on single core
        }
        
        // Count cross-core dependencies
        let cross_core_deps = self.count_cross_core_deps(actors);
        
        // Model bus overhead
        let interconnect = platform.interconnect.as_ref()
            .expect("Multi-core platform must have interconnect model");
        
        // Overhead per message (cycles)
        let overhead_per_msg = interconnect.latency_cycles as f64;
        
        cross_core_deps as f64 * overhead_per_msg
    }
}

pub struct MultiCoreSchedulabilityResult {
    pub per_core: Vec<(usize, SchedulabilityResult)>,
    pub interconnect_overhead: f64,
    pub overall_schedulable: bool,
}
```

**Deliverable**:
- Multi-core platform models
- Partitioned scheduling analysis
- Interconnect overhead modeling

---

### Phase 4: ChronVIEW/BTF Export (2 weeks)

**Goal**: Export WCET and scheduling results to industry-standard formats for visualization.

#### 4.1 BTF Trace Format

BTF (Best Trace Format) is a CSV-based format used by chronVIEW.

**Format:**
```
#version 2.1.3
#creator LALE
#creationDate 2025-11-01 12:00:00

timestamp,source,source_instance,event_type,event,target,target_instance,data

0,ECU,0,set_frequency,1000000000,,,  # 1GHz CPU
0,Core,0,T,actor1,actor1,0,         # Task definition
100,Core,0,start,actor1,actor1,0,
150,Core,0,terminate,actor1,actor1,0,
150,Core,0,start,actor2,actor2,0,
200,Core,0,terminate,actor2,actor2,0,
```

#### 4.2 BTF Exporter

**File**: `lale/src/output/btf.rs`

```rust
pub struct BTFExporter;

impl BTFExporter {
    pub fn export_schedule(
        actors: &[Actor],
        schedule: &StaticSchedule,
        platform: &PlatformModel,
        output_path: &Path,
    ) -> Result<(), String> {
        let mut btf = String::new();
        
        // Header
        btf.push_str("#version 2.1.3\n");
        btf.push_str("#creator LALE\n");
        btf.push_str(&format!("#creationDate {}\n", chrono::Utc::now()));
        btf.push_str("\n");
        
        // Column headers
        btf.push_str("timestamp,source,source_instance,event_type,event,target,target_instance,data\n");
        
        // Platform info
        btf.push_str(&format!(
            "0,ECU,0,set_frequency,{},,,\n", 
            platform.cpu_frequency_mhz * 1_000_000
        ));
        
        for core_id in 0..platform.num_cores {
            btf.push_str(&format!("0,Core,{},T,,,,\n", core_id));
        }
        
        // Task definitions
        for actor in actors {
            let core = actor.core_affinity.unwrap_or(0);
            btf.push_str(&format!(
                "0,Core,{},T,{},{},0,\n",
                core, actor.name, actor.name
            ));
        }
        
        // Schedule events
        for event in &schedule.events {
            let core = actors[event.task_id].core_affinity.unwrap_or(0);
            let actor = &actors[event.task_id];
            
            match event.event_type {
                EventType::Start => {
                    btf.push_str(&format!(
                        "{},Core,{},start,{},{},0,\n",
                        event.time_us as u64,
                        core,
                        actor.name,
                        actor.name
                    ));
                }
                EventType::Complete => {
                    btf.push_str(&format!(
                        "{},Core,{},terminate,{},{},0,\n",
                        event.time_us as u64,
                        core,
                        actor.name,
                        actor.name
                    ));
                }
                EventType::Preempt => {
                    btf.push_str(&format!(
                        "{},Core,{},preempt,{},{},0,\n",
                        event.time_us as u64,
                        core,
                        actor.name,
                        actor.name
                    ));
                }
                EventType::Resume => {
                    btf.push_str(&format!(
                        "{},Core,{},resume,{},{},0,\n",
                        event.time_us as u64,
                        core,
                        actor.name,
                        actor.name
                    ));
                }
            }
        }
        
        std::fs::write(output_path, btf)
            .map_err(|e| format!("Failed to write BTF: {}", e))
    }
}
```

#### 4.3 Integration with Main Tool

**File**: `lale/src/main.rs` (extend)

```rust
// Add new CLI option
#[derive(Parser)]
struct Args {
    // ... existing fields
    
    #[arg(long)]
    actors: Option<PathBuf>,  // actors.toml
    
    #[arg(long)]
    export_btf: Option<PathBuf>,  // output.btf
}

fn main() -> Result<()> {
    // ... existing code
    
    // NEW: Actor analysis
    if let Some(actors_path) = args.actors {
        let actor_config = ActorConfig::load(&actors_path)?;
        
        // Analyze each actor
        let mut actors = Vec::new();
        for actor_def in actor_config.actors {
            let module = /* find module with actor function */;
            let func = /* find function */;
            
            let async_info = AsyncDetector::extract_state_machine(func)?;
            let state_wcets = /* analyze each state */;
            
            let actor = Actor {
                name: actor_def.name,
                function: actor_def.function,
                async_info,
                priority: actor_def.priority,
                deadline_us: actor_def.deadline_us,
                period_us: actor_def.period_us,
                core_affinity: actor_def.core_affinity,
                state_wcets,
                max_wcet_cycles: /* max of state_wcets */,
                max_wcet_us: /* convert to us */,
            };
            actors.push(actor);
        }
        
        // Multi-core schedulability
        let scheduler = PartitionedScheduler::new(policy);
        let result = scheduler.analyze_schedulability(&actors, &platform);
        
        // Generate schedule
        if result.overall_schedulable {
            let schedule = scheduler.generate_schedule(&actors, &platform)?;
            
            // Export to BTF
            if let Some(btf_path) = args.export_btf {
                BTFExporter::export_schedule(&actors, &schedule, &platform, &btf_path)?;
                println!("BTF trace exported to: {}", btf_path.display());
            }
        } else {
            eprintln!("System is NOT schedulable!");
            std::process::exit(1);
        }
    }
    
    Ok(())
}
```

**Deliverable**:
- BTF trace exporter
- chronVIEW-compatible output
- Integration with scheduling pipeline

---

### Phase 5: Advanced Features (Optional, 3-4 weeks)

These are the missing LLVMTA components from the project files, providing higher precision.

#### 5.1 Abstract Execution Graph (AEG)

Instead of basic CFG with fixed timings, build AEG with microarchitectural states.

**File**: `lale/src/aeg/mod.rs`

```rust
pub struct AEG {
    pub graph: DiGraph<MicroArchState, AEGEdge>,
    pub initial_state: NodeIndex,
    pub final_states: Vec<NodeIndex>,
}

pub struct MicroArchState {
    pub pc: Address,
    pub pipeline: PipelineState,
    pub cache: AbstractCacheState,
}

pub struct PipelineState {
    pub stages: Vec<Option<InstructionInfo>>,
}

pub struct AbstractCacheState {
    pub i_cache: CacheSetState,
    pub d_cache: CacheSetState,
}

// Must/May analysis for cache
pub enum CacheSetState {
    Must(AHashSet<MemoryBlock>),     // guaranteed hits
    May(AHashSet<MemoryBlock>),      // possible hits
}

impl AEG {
    pub fn from_basic_block(
        bb: &BasicBlock,
        initial_state: MicroArchState,
        platform: &PlatformModel,
    ) -> Self {
        // Cycle-by-cycle simulation
        let mut graph = DiGraph::new();
        let entry = graph.add_node(initial_state);
        
        let mut worklist = vec![entry];
        let mut visited = HashSet::new();
        
        while let Some(node_idx) = worklist.pop() {
            if visited.contains(&node_idx) {
                continue;
            }
            visited.insert(node_idx);
            
            let state = &graph[node_idx];
            
            // Simulate one cycle
            let next_states = Self::simulate_cycle(state, platform);
            
            for (next_state, cycles) in next_states {
                let next_idx = graph.add_node(next_state);
                graph.add_edge(node_idx, next_idx, AEGEdge { cycles });
                worklist.push(next_idx);
            }
        }
        
        AEG {
            graph,
            initial_state: entry,
            final_states: /* find terminal states */,
        }
    }
    
    fn simulate_cycle(
        state: &MicroArchState,
        platform: &PlatformModel,
    ) -> Vec<(MicroArchState, u32)> {
        // Advance pipeline
        let mut next_state = state.clone();
        
        // Fetch stage
        if next_state.pipeline.stages[0].is_none() {
            let instr = /* fetch from PC */;
            next_state.pipeline.stages[0] = Some(instr);
        }
        
        // Advance each stage
        for i in (1..platform.pipeline.depth).rev() {
            next_state.pipeline.stages[i] = next_state.pipeline.stages[i-1].take();
        }
        
        // Check for cache hit/miss non-determinism
        if let Some(mem_access) = Self::get_memory_access(&next_state) {
            // Split into hit and miss cases
            let hit_state = Self::apply_cache_hit(next_state.clone());
            let miss_state = Self::apply_cache_miss(next_state.clone());
            
            vec![
                (hit_state, 1),
                (miss_state, platform.cache.miss_penalty),
            ]
        } else {
            vec![(next_state, 1)]
        }
    }
}
```

**Benefit**: Much more precise WCET by modeling timing correlations between instructions.

#### 5.2 Loop Bound Analysis

**File**: `lale/src/analysis/loop_bounds.rs`

```rust
pub struct LoopBoundAnalyzer;

impl LoopBoundAnalyzer {
    /// Try to infer loop bounds from LLVM IR using scalar evolution
    pub fn infer_loop_bounds(func: &llvm_ir::Function) -> HashMap<String, u64> {
        let mut bounds = HashMap::new();
        
        // Look for loop patterns
        for bb in &func.basic_blocks {
            if let Some(loop_info) = Self::detect_loop(bb, func) {
                if let Some(bound) = Self::analyze_loop_bound(&loop_info) {
                    bounds.insert(loop_info.header_label, bound);
                }
            }
        }
        
        bounds
    }
    
    fn analyze_loop_bound(loop_info: &LoopInfo) -> Option<u64> {
        // Pattern matching on common loop patterns:
        // for i in 0..N
        // while condition based on bounded counter
        // etc.
        
        todo!("Implement loop bound inference")
    }
}
```

---

## Part 5: Implementation Timeline & Milestones

### Timeline (10-14 weeks total)

| Phase | Duration | Deliverable | Priority |
|-------|----------|-------------|----------|
| **Phase 1**: Async Detection | 2-3 weeks | Async function identification, state machine extraction | **CRITICAL** |
| **Phase 2**: Actor Metadata | 1-2 weeks | Deadline/priority configuration, actor model | **CRITICAL** |
| **Phase 3**: Multi-Core Scheduling | 2-3 weeks | Partitioned scheduling, core affinity | **HIGH** |
| **Phase 4**: BTF Export | 2 weeks | chronVIEW integration | **HIGH** |
| **Phase 5**: AEG (optional) | 3-4 weeks | Precise timing with microarch states | **MEDIUM** |

### Milestones

**M1** (End of Phase 1): LALE can detect and analyze basic async actors
- Demo: WCET per state for simple async function

**M2** (End of Phase 2): Actors can have deadlines and priorities
- Demo: Configuration file with actor definitions

**M3** (End of Phase 3): Multi-core schedulability analysis works
- Demo: Schedulability report for 2-core system with 5 actors

**M4** (End of Phase 4): Customer can visualize results in chronVIEW
- Demo: Import BTF trace into chronVIEW, show Gantt chart

**M5** (End of Phase 5, optional): High-precision WCET with AEG
- Demo: Side-by-side comparison of basic vs. AEG analysis

---

## Part 6: Testing Strategy

### 6.1 Unit Tests

**For Each Module:**

```rust
// tests/async_detection.rs
#[test]
fn test_detect_simple_async() {
    let ir = r#"
    define i8 @async_function(...) {
    entry:
      %state = load i32, i32* %0
      switch i32 %state, label %unreachable [
        i32 0, label %state0
        i32 1, label %state1
      ]
    state0:
      store i32 1, i32* %0
      ret i8 0
    state1:
      ret i8 1
    unreachable:
      unreachable
    }
    "#;
    
    let module = parse_ir_string(ir).unwrap();
    let func = &module.functions[0];
    
    assert!(AsyncDetector::is_async_function(func));
    
    let state_machine = AsyncDetector::extract_state_machine(func).unwrap();
    assert_eq!(state_machine.states.len(), 2);
}
```

### 6.2 Integration Tests

**Test with Real Veecle OS Code:**

1. Compile simple Veecle OS actor to LLVM IR
2. Run LALE analysis
3. Verify WCET results
4. Check BTF output format

```bash
# Generate LLVM IR from Veecle OS example
cd veecle-os-examples/ping-pong
cargo rustc --release -- --emit=llvm-ir

# Analyze with LALE
lale analyze \
  --dir target/release/deps/ \
  --actors config/actors.toml \
  --platform cortex-m4 \
  --export-btf output.btf

# Import to chronVIEW (manual step)
chronVIEW import output.btf
```

### 6.3 Validation Against LLVMTA

Compare LALE results with LLVMTA on same benchmarks (TACLEBench, Mälardalen).

---

## Part 7: Customer Workflow

### End-to-End Usage

```bash
# 1. Customer develops Veecle OS actors
# (in their Veecle OS project)

# 2. Generate LLVM IR
cargo rustc --release --target <target> -- --emit=llvm-ir

# 3. Configure actors
cat > actors.toml <<EOF
[[actor]]
name = "sensor_fusion"
function = "sensor_fusion_actor::{{closure}}"
priority = 10
deadline_us = 1000
period_us = 1000
core_affinity = 0

[[actor]]
name = "motor_control"
function = "motor_control_actor::{{closure}}"
priority = 20
deadline_us = 500
period_us = 1000
core_affinity = 1
EOF

# 4. Run LALE analysis
lale analyze \
  --dir target/release/deps/ \
  --actors actors.toml \
  --platform cortex-r5 \  # dual-core
  --policy edf \
  --export-btf schedule.btf \
  --export-json report.json

# 5. View results
cat report.json
# {
#   "actors": [
#     {
#       "name": "sensor_fusion",
#       "wcet_us": 850.0,
#       "deadline_us": 1000.0,
#       "schedulable": true
#     },
#     ...
#   ],
#   "overall_schedulable": true,
#   "utilization": {
#     "core_0": 0.72,
#     "core_1": 0.45
#   }
# }

# 6. Visualize in chronVIEW
# File > Import > BTF Trace > schedule.btf
# View Gantt chart, timing metrics, deadlines
```

---

## Part 8: Required Inputs Summary

### For LALE to Work

**Inputs from Customer:**

1. **LLVM IR files** (`.ll`)
   - Generated from Rust with `--emit=llvm-ir`
   - All actor functions must be present

2. **Actor configuration** (`actors.toml`)
   - Actor names and function mappings
   - Priorities, deadlines, periods
   - Core affinity (for multi-core)
   - Dependencies (optional, for better analysis)

3. **Platform specification** (built-in or custom)
   - Target CPU (Cortex-M4, Cortex-R5, etc.)
   - Number of cores
   - Clock frequency
   - Cache configuration
   - Pipeline model (if available)

4. **Loop bounds** (if cannot be inferred)
   - Manual annotations for complex loops
   - Format: CSV or inline annotations

**Optional Inputs:**

5. **Memory layout** (for precise cache analysis)
   - Stack/heap regions
   - SRAM/Flash addresses
   - Linker script information

6. **Veecle OS runtime configuration**
   - Executor settings
   - Channel buffer sizes
   - Priority levels

---

## Part 9: Rust Async LLVM IR Deep Dive

### Detailed Example

**Source:**
```rust
async fn example_actor(mut writer: Writer<'_, Data>) -> u32 {
    let x = first_operation().await;
    let y = second_operation(x).await;
    writer.write(Data { value: y }).await;
    x + y
}
```

**LLVM IR (simplified):**

```llvm
; Generator struct for state machine
%Generator = type { i32, i32, i32, %Future1, %Future2, %Writer }
;                   ^state ^x   ^y   ^pending futures     ^writer

define i8 @"poll_example_actor"(%Generator* %self, %Context* %cx) {
entry:
  %state_ptr = getelementptr %Generator, %Generator* %self, i32 0, i32 0
  %state = load i32, i32* %state_ptr
  
  switch i32 %state, label %unreachable [
    i32 0, label %state0_initial
    i32 1, label %state1_await_first
    i32 2, label %state2_await_second
    i32 3, label %state3_await_write
    i32 4, label %state4_done
  ]

state0_initial:
  ; Start first_operation
  call void @start_first_operation(%Future1* ...)
  store i32 1, i32* %state_ptr  ; transition to state 1
  ret i8 0  ; Poll::Pending

state1_await_first:
  ; Poll first_operation
  %poll1 = call i8 @poll_future1(%Future1* ..., %Context* %cx)
  %is_ready1 = icmp eq i8 %poll1, 1
  br i1 %is_ready1, label %first_ready, label %first_pending

first_ready:
  ; Get result, store in x
  %x = call i32 @future1_output(%Future1* ...)
  %x_ptr = getelementptr %Generator, %Generator* %self, i32 0, i32 1
  store i32 %x, i32* %x_ptr
  
  ; Start second_operation
  call void @start_second_operation(%Future2* ..., i32 %x)
  store i32 2, i32* %state_ptr  ; transition to state 2
  ret i8 0  ; Poll::Pending

first_pending:
  ret i8 0  ; stay in state 1, Poll::Pending

state2_await_second:
  ; Similar to state1, poll second_operation
  ; ...
  ; On ready: store y, start write, go to state 3

state3_await_write:
  ; Poll writer.write()
  ; On ready: go to state 4

state4_done:
  ; Load x, y, compute x + y
  %x_final = load i32, i32* %x_ptr
  %y_final = load i32, i32* %y_ptr
  %result = add i32 %x_final, %y_final
  
  ; Store result for return
  %result_ptr = getelementptr %Generator, %Generator* %self, i32 0, i32 5
  store i32 %result, i32* %result_ptr
  
  ret i8 1  ; Poll::Ready

unreachable:
  unreachable
}
```

**Key Observations for LALE:**

1. **State is first field** of generator struct (offset 0)
2. **Switch dispatch** determines current state
3. **Store to state field** = state transition
4. **Return 0** = `Poll::Pending` (suspend)
5. **Return 1** = `Poll::Ready` (complete)
6. **Each state is a basic block** or set of blocks
7. **WCET must be calculated per state**, not total function

---

## Part 10: Integration with Inchron Tools

### ChronVIEW Usage

**What Customer Sees:**

1. **Gantt Chart**: Visual timeline of actor execution
   - X-axis: Time (microseconds)
   - Y-axis: Cores / Actors
   - Bars show when each actor runs
   - Colors indicate state (running, suspended, ready)

2. **Timing Metrics**:
   - WCET per actor
   - Response time
   - Deadline misses (if any)
   - CPU utilization per core

3. **Event Chains**:
   - Data flow: sensor → processing → actuator
   - End-to-end latency
   - Jitter analysis

4. **Statistics**:
   - Histogram of execution times
   - Distribution of inter-arrival times
   - Load balancing across cores

### ChronSIM Integration (Future)

LALE could also export models to chronSIM for:
- "What-if" analysis (change frequencies, add actors)
- Corner case simulation
- Stochastic analysis (probability distributions)

**Export Format**: AMALTHEA model or Python API

---

## Part 11: Risk Mitigation

### Technical Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| LLVM IR varies by Rust version | HIGH | Test with multiple Rust versions (1.70+), document supported versions |
| Async pattern detection false negatives | HIGH | Provide manual annotation fallback, extensive testing |
| WCET overestimation in async context | MEDIUM | Compare with measurement-based timing, refine models |
| Multi-core interference hard to model | MEDIUM | Start with conservative partitioned scheduling, document limitations |
| BTF format incompatibility | LOW | Use official BTF spec 2.1.3, validate with chronVIEW test suite |

### Project Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Scope creep (adding LLVMTA features) | MEDIUM | Prioritize async support, make AEG optional |
| Insufficient test coverage | HIGH | Require 80%+ coverage, CI/CD with real Veecle OS examples |
| Poor performance on large systems | MEDIUM | Profile and optimize hot paths, consider caching |
| Customer expectation mismatch | HIGH | Deliver working demos early (M1-M3), weekly updates |

---

## Part 12: Success Criteria

### Minimum Viable Product (MVP)

At end of Phase 4, LALE must:

✅ **Functional Requirements:**
1. Detect Veecle OS async actors in LLVM IR
2. Calculate WCET per actor (max over all states)
3. Accept deadline/priority configuration
4. Perform multi-core schedulability analysis
5. Export BTF trace for chronVIEW

✅ **Quality Requirements:**
6. Accuracy: WCET within 20% of measurement (for simple actors)
7. Performance: Analyze 50 actors in < 5 minutes
8. Usability: Single command-line invocation
9. Compatibility: Works with Rust 1.70+ and Veecle OS latest

### Stretch Goals

If time permits:
- Full AEG implementation (Phase 5)
- Automatic loop bound inference
- Cache conflict analysis
- ChronSIM model export
- GUI for configuration

---

## Appendix A: Code Structure

```
lale/
├── src/
│   ├── main.rs                 # CLI entry point
│   ├── lib.rs                  # Public API
│   │
│   ├── async_analysis.rs       # NEW: Async detection & state extraction
│   ├── actor.rs                # NEW: Actor model & system
│   │
│   ├── analysis/
│   │   ├── mod.rs
│   │   ├── loop_analyzer.rs    # Existing
│   │   ├── timing.rs           # Existing
│   │   └── ipet.rs             # Existing: ILP solver
│   │
│   ├── ir/
│   │   ├── mod.rs
│   │   ├── parser.rs           # Existing: LLVM IR parsing
│   │   ├── cfg.rs              # Existing: Control Flow Graph
│   │   └── call_graph.rs       # Existing
│   │
│   ├── scheduling/
│   │   ├── mod.rs
│   │   ├── rma.rs              # Existing: Rate Monotonic
│   │   ├── edf.rs              # Existing: Earliest Deadline First
│   │   ├── multicore.rs        # NEW: Multi-core partitioned scheduling
│   │   └── static_sched.rs     # Existing: Schedule generation
│   │
│   ├── aeg/                    # NEW: Abstract Execution Graph (Phase 5)
│   │   ├── mod.rs
│   │   ├── builder.rs          # AEG construction
│   │   ├── state.rs            # Microarchitectural state
│   │   └── simulator.rs        # Cycle-level simulation
│   │
│   ├── platform/
│   │   ├── mod.rs
│   │   ├── cortex_m.rs         # Existing: Cortex-M models
│   │   ├── cortex_r.rs         # Existing: Cortex-R models
│   │   ├── riscv.rs            # Existing: RISC-V models
│   │   └── cache.rs            # Existing: Cache models
│   │
│   ├── output/
│   │   ├── mod.rs
│   │   ├── json.rs             # Existing
│   │   ├── graphviz.rs         # Existing
│   │   ├── btf.rs              # NEW: BTF trace export
│   │   └── report.rs           # Existing
│   │
│   └── config.rs               # Extend for actor config
│
├── config/
│   ├── platforms/              # Existing
│   └── examples/
│       └── veecle-actors.toml  # NEW: Example actor config
│
├── tests/
│   ├── async_detection.rs      # NEW
│   ├── actor_analysis.rs       # NEW
│   ├── multicore_sched.rs      # NEW
│   └── btf_export.rs           # NEW
│
└── examples/
    └── veecle-pingpong/        # NEW: Real Veecle OS example
```

---

## Appendix B: Reference Documents

### Must Read

1. **LLVMTA Paper** (project file): Architecture and AEG concepts
2. **Rust Async Book**: https://rust-lang.github.io/async-book/
3. **LLVM Coroutines**: https://llvm.org/docs/Coroutines.html
4. **BTF Spec 2.1.3**: https://www.eclipse.org/app4mc/documentation/
5. **Veecle OS Docs**: https://docs.rs/veecle-os-runtime/

### Helpful

- "Lowering async/await in Rust": https://wiki.cont.run/lowering-async-await-in-rust/
- ChronVIEW User Manual: https://www.inchron.com/support/
- Real-Time Systems (Liu): Schedulability theory
- "WCET Analysis of Object Code" (Rapita): Trace-based methods

---

## Appendix C: Quick Start Commands

### Generate LLVM IR from Veecle OS

```bash
# In Veecle OS project
cd veecle-os-examples/ping-pong

# Generate LLVM IR for all dependencies
RUSTFLAGS="--emit=llvm-ir" cargo build --release --target thumbv7em-none-eabihf

# IR files will be in:
# target/thumbv7em-none-eabihf/release/deps/*.ll
```

### Run LALE Analysis

```bash
# Simple analysis
lale --dir target/release/deps/ --platform cortex-m4

# Full actor analysis with BTF export
lale \
  --dir target/release/deps/ \
  --actors config/actors.toml \
  --platform cortex-r5 \
  --num-cores 2 \
  --policy edf \
  --export-btf output.btf \
  --export-json report.json \
  --verbose
```

### Validate BTF Output

```bash
# Check BTF format
head -n 20 output.btf

# Expected:
# #version 2.1.3
# #creator LALE
# ...
# timestamp,source,source_instance,event_type,event,target,...
```

---

## Summary

This implementation plan provides a complete roadmap for extending LALE to support Veecle OS's async actor runtime with multi-core schedulability analysis and industry-standard trace export.

**Key Takeaways:**

1. **Rust async → LLVM IR**: Async functions become state machines with switch-based dispatch
2. **State-based WCET**: Calculate timing per state, not per entire function
3. **Actor = Task**: Map async actors to schedulable tasks with deadlines
4. **Multi-core**: Partitioned scheduling with core affinity
5. **BTF Export**: Industry-standard format for chronVIEW integration

**Implementation Priority**: Phases 1-4 are critical (8-10 weeks), Phase 5 is optional but provides much higher precision.

Customer can immediately benefit from schedulability analysis and visualization, even without full AEG implementation.
