# LALE Extension for Veecle OS Actor WCET Analysis: Comprehensive Implementation Plan

## Executive Summary

This implementation plan details how to extend LALE (LLVM-based WCET Analysis Tool) to support Veecle OS actor-based systems. The approach combines async/await detection at LLVM IR level, actor-specific metadata extraction, per-actor WCET analysis, and system-level schedulability analysis. The plan enables WCET estimation for individual actors, priority/deadline assignment, and integration with industry-standard tools like Inchron chronVIEW.

**Critical Finding**: LALE currently lacks any support for concurrent, asynchronous, or multi-actor execution models. This extension requires significant architectural additions rather than simple modifications.

---

## 1. Technical Foundation: Rust Async/Await in LLVM IR

### 1.1 Understanding the Transformation Pipeline

Rust async functions undergo a multi-stage transformation that LALE must understand:

**Compilation Flow**:
```
Rust async fn → HIR → MIR (state machine) → LLVM IR (discriminant-based switches) → Machine code
```

**Key Insight**: Rust does NOT use LLVM's native coroutine intrinsics. Instead, it implements state machines at the MIR level, which appear in LLVM IR as ordinary control flow with discriminant-based switches.

### 1.2 LLVM IR Patterns for Async Functions

**State Machine Structure in LLVM IR**:

```llvm
; Actor becomes three generated functions:

; 1. Constructor (creates generator state)
define ptr @actor_name() {
    ; Allocates generator struct
    ; Initializes state discriminant to 0 (unresumed)
    ; Stores captured arguments
}

; 2. Poll/Resume function (actual execution)
define internal fastcc i1 @actor_name.resume(
    ptr %generator,
    ptr %context
) {
entry:
    ; Load discriminant field (state number)
    %disc_ptr = getelementptr inbounds %Generator, ptr %generator, i32 0, i32 1
    %state = load i8, ptr %disc_ptr
    
    ; Switch on current state
    switch i8 %state, label %unreachable [
        i8 0, label %start           ; Initial state
        i8 1, label %completed       ; Already done (panic)
        i8 3, label %suspend_point_1 ; Resume after first await
        i8 4, label %suspend_point_2 ; Resume after second await
    ]
    
start:
    ; Execute up to first await point
    ; ...
    store i8 3, ptr %disc_ptr        ; Update to suspended
    ret i1 0                         ; Return Poll::Pending

suspend_point_1:
    ; Continue from first suspension
    ; ...
}

; 3. Destroy function (cleanup)
```

**Generator Type Structure**:
```llvm
%"[static generator@src:line:col]" = type {
    ptr,     ; resume function pointer
    ptr,     ; destroy function pointer  
    i8/i32,  ; state discriminant
    ...      ; captured variables and local state
}
```

### 1.3 Detection Algorithm for Async Functions

**High-Confidence Markers**:

1. **Type name contains**: `[static generator@`, `{closure#`, `{{closure}}`
2. **Discriminant switch pattern**: First basic block loads small integer, immediately switches with values 0, 1, 2, 3+
3. **Function signature**: `(ptr, ptr) -> i1` (generator, context) → bool (Pending/Ready)
4. **State update pattern**: `store i8 N, ptr %state_ptr` followed by `ret i1 0`

**Detection Pseudocode**:
```
function detect_async_actor(llvm_function):
    confidence = 0
    
    # Check type names
    if contains(function.types, "generator@"):
        confidence += 4
    
    # Check entry block pattern
    entry = function.entry_block
    first_insts = entry.instructions[0:5]
    
    for inst in first_insts:
        if inst.opcode == GETELEMENTPTR and inst.field_index in [1, 2]:
            load_inst = inst.next
            if load_inst.opcode == LOAD and sizeof(load_inst.type) <= 4:
                switch_inst = load_inst.next
                if switch_inst.opcode == SWITCH:
                    cases = switch_inst.case_values
                    if 0 in cases and 1 in cases:
                        confidence += 5  # Strong indicator
    
    # Check for panic strings
    for call in function.calls:
        if "async_fn_resumed" in call.name:
            confidence += 3
    
    return confidence >= 8
```

### 1.4 Extracting Execution Segments

**Segment Definition**: Code between consecutive await points forms a "run-to-completion" segment - the natural unit for WCET analysis.

**Segment Identification**:
- Each state in the discriminant switch represents resume after an await
- Transitions between states mark segment boundaries
- Analyze each state's basic blocks as a separate WCET analysis unit

**Actor Structure**:
```
Actor Function:
  Segment 0 (initial) → await point 1
  Segment 1 → await point 2
  Segment 2 → await point 3
  ...
  Segment N → return/loop

Actor WCET = max(all segment WCETs)
```

---

## 2. LALE Current Architecture Analysis

### 2.1 Core Capabilities (What Works)

LALE provides solid foundation for single-threaded WCET analysis:

**Strengths**:
- Cycle-accurate microarchitectural simulation
- Abstract Execution Graph (AEG) construction
- ILP-based path analysis with loop bounds
- Context-sensitive analysis
- Cache/pipeline modeling
- LLVM IR integration

**Analysis Flow**:
```
LLVM Machine IR → CFG Construction → Static Analysis → 
Microarchitectural Simulation → AEG → ILP Formulation → WCET Bound
```

### 2.2 Critical Gaps for Actor Support

**LALE Does NOT Support**:
- ❌ Concurrent execution (assumes sequential single-threaded code)
- ❌ Asynchronous communication/message passing
- ❌ Multiple tasks/actors executing simultaneously
- ❌ Inter-actor dependencies and timing
- ❌ Scheduling policies (no notion of priorities, deadlines)
- ❌ Multi-core shared resource contention
- ❌ Event-driven execution models

**Impact**: Cannot directly analyze actor systems. Requires significant architectural extension.

### 2.3 Extension Points

LALE provides well-defined interfaces for extension:

1. **ContextAwareAnalysisDomain**: Add actor context tracking
2. **MicroArchitecturalState**: Model actor scheduling states
3. **StateGraphEdgeWeightProvider**: Extract per-actor metrics
4. **Analysis Passes**: Create actor-specific analysis passes

---

## 3. Veecle OS Actor Model

### 3.1 Architecture Overview

**Core Concepts**:
- **Actors**: Async Rust functions annotated with `#[veecle_os_runtime::actor]`
- **Communication**: Slot-based Reader/Writer pattern (not traditional queues)
- **Scheduling**: Cooperative (no preemption), futures-based executor
- **Memory**: Zero heap allocation, compile-time static allocation

**Example Actor**:
```rust
#[veecle_os_runtime::actor]
async fn control_actor(
    sensor: Reader<'_, SensorData>,
    mut output: Writer<'_, ControlCmd>
) -> Infallible {
    let mut sensor = sensor.wait_init().await;
    
    loop {
        let data = sensor.wait_for_update().await.read_cloned();
        let result = process(data);  // Computation segment
        output.write(result).await;
    }
}
```

### 3.2 Communication Model

**Slot-Based, Not Queue-Based**:
- Each data type has one static slot (single-producer, multi-consumer)
- `Writer<T>.write().await` publishes to all readers, then yields
- `Reader<T>.wait_for_update().await` blocks until new data
- No buffering, no queuing - latest value only

**Key Property**: Simpler timing analysis than traditional message queues (no queue depth, no blocking delays).

### 3.3 Execution Model

**Cooperative Scheduling**:
- Actors only yield at `.await` points
- Run-to-completion semantics between awaits
- No preemption within segments
- Fair scheduling (all ready actors polled)
- Event-driven wakeup (wakers notify when data available)

### 3.4 Current Limitations

**No Built-in Support For**:
- Explicit priorities or deadlines
- Real-time scheduling policies
- CPU time budgets or quotas
- Timing annotations
- Multi-core execution (currently single-core focused)

---

## 4. Inchron Tools Integration Requirements

### 4.1 Tool Capabilities

**chronVIEW**: Trace visualization and timing verification
**chronSIM**: Model-based real-time system simulation
**chronSUITE**: Complete workflow integration

### 4.2 Primary Integration: BTF Trace Export

**BTF (Best Trace Format)**: CSV-based, industry-standard trace format

**Structure**:
```csv
# Header
#version 2.2.1
#creator LALE-Veecle_v1.0
#creationDate 2025-11-01 10:00:00
#timeScale ns

# Actor definitions
0,Core_0,T,ControlActor,0,"priority=10,deadline=100000000"
0,Core_0,T,SensorActor,0,"priority=5,deadline=200000000"

# Execution trace
0,Core_0,activate,ControlActor,0,
0,Core_0,start,ControlActor,0,
15000000,Core_0,terminate,ControlActor,0,
50000000,Core_0,activate,SensorActor,0,
50000000,Core_0,start,SensorActor,0,
75000000,Core_0,terminate,SensorActor,0,
```

**Required Events**:
- `activate`, `start`, `terminate`: Task lifecycle
- `preempt`, `resume`: Context switches (if preemptive)
- `wait`, `release`: Synchronization points
- Timestamps in nanoseconds

### 4.3 Secondary Integration: AMALTHEA Model

**Format**: `.amxmi` (AMALTHEA XML Model Instance)

**Required Elements**:
- **Software Model**: Tasks with WCET/BCET/periods/deadlines
- **Hardware Model**: Processors, cores, memory
- **Constraints**: Timing requirements, event chains
- **Stimuli**: Activation patterns

**Benefit**: Enables simulation in chronSIM for validation.

---

## 5. Implementation Plan

### 5.1 Phase 1: Async Function Detection and Analysis (Foundation)

**Goal**: Teach LALE to recognize and analyze Rust async functions at LLVM IR level.

#### Step 1.1: Async Function Detector Pass

**Implementation**:
- Create new LLVM analysis pass: `AsyncFunctionDetector`
- Scan all functions in module for async patterns
- Build registry of detected async functions with metadata

**Key Components**:
```cpp
class AsyncFunctionDetector {
public:
    struct AsyncFunctionInfo {
        llvm::Function* function;
        std::vector<BasicBlock*> state_blocks;  // One per discriminant value
        std::map<int, BasicBlock*> state_to_block;  // State number → entry block
        Value* discriminant_ptr;  // Pointer to state field
        bool is_actor;  // True if Veecle OS actor
    };
    
    std::map<Function*, AsyncFunctionInfo> detect_all_async_functions(Module* M);
    
private:
    bool matches_async_pattern(Function* F);
    void extract_state_machine_structure(Function* F, AsyncFunctionInfo& info);
};
```

**Detection Algorithm**:
1. Iterate all functions in module
2. Check for generator type names in referenced types
3. Analyze entry block for discriminant switch pattern
4. Verify switch values match expected state machine pattern (0, 1, 2, 3+)
5. Map each switch case to its target basic block
6. Identify state update instructions

#### Step 1.2: Segment Extraction Pass

**Implementation**:
- Extract execution segments between await points
- Each segment = sequence of basic blocks reachable from one state
- Build intra-segment control flow graph

**Data Structure**:
```cpp
struct ActorSegment {
    int segment_id;  // Corresponds to discriminant state
    BasicBlock* entry_block;
    std::set<BasicBlock*> reachable_blocks;
    std::set<BasicBlock*> exit_blocks;  // Blocks that update state/return
    SegmentType type;  // INITIAL, SUSPENDED_RESUME, COMPLETION
};

struct ActorExecutionModel {
    Function* actor_function;
    std::vector<ActorSegment> segments;
    std::map<int, int> state_to_segment;
    // Transition graph: segment → next possible segments
    std::map<int, std::vector<int>> segment_transitions;
};
```

**Analysis**:
- For each state in discriminant switch:
  - Starting from target block, explore all reachable blocks
  - Stop at blocks that update discriminant or return
  - These form one segment
- Build transition graph between segments

#### Step 1.3: Per-Segment WCET Analysis

**Approach**: Leverage existing LALE infrastructure for each segment independently.

**Implementation**:
```cpp
class SegmentWCETAnalyzer {
public:
    struct SegmentWCET {
        uint64_t wcet_cycles;
        uint64_t bcet_cycles;
        Path worst_case_path;
        std::map<std::string, LocalMetrics> metrics;  // Cache misses, etc.
    };
    
    SegmentWCET analyze_segment(
        ActorSegment& segment,
        MicroArchitecturalModel& hw_model,
        AnalysisContext& context
    );
};
```

**Analysis Steps** (per segment):
1. **Control Flow Analysis**: Build CFG for segment blocks
2. **Loop Bound Analysis**: Extract loop bounds (requires user annotations)
3. **Microarchitectural Simulation**: Run existing LALE cycle-accurate simulation
4. **Path Analysis**: Use ILP to find worst-case path within segment
5. **Result**: WCET for this segment

**Key Modification**: Restrict analysis to segment boundaries (don't cross state updates).

---

### 5.2 Phase 2: Actor Metadata Extraction System

**Goal**: Extract and embed timing requirements, priorities, and actor attributes.

#### Step 2.1: Metadata Specification Format

**Design Choice**: Use embedded LLVM metadata or external configuration files.

**Option A: LLVM Metadata Attributes** (Recommended)
Emit metadata during Rust compilation via build script:

```llvm
!veecle.actor.control_actor = !{
    !"name", !"control_actor",
    !"priority", i32 10,
    !"deadline_ns", i64 100000000,
    !"period_ns", i64 50000000,
    !"activation", !"periodic",
    !"source_location", !"src/actors.rs:42"
}

define internal fastcc i1 @"control_actor{{closure}}"(ptr %gen, ptr %ctx) 
    !veecle.actor !veecle.actor.control_actor {
    ; Function body
}
```

**Option B: External JSON Configuration**
```json
{
  "actors": [
    {
      "name": "control_actor",
      "mangled_name": "control_actor{{closure}}",
      "priority": 10,
      "deadline_ns": 100000000,
      "period_ns": 50000000,
      "activation": "periodic",
      "core_affinity": 0
    }
  ]
}
```

**Recommendation**: Hybrid approach - LLVM metadata for compilation-visible info, JSON for user-specified constraints.

#### Step 2.2: Veecle OS Macro Extension

**Extend `#[veecle_os_runtime::actor]` Macro**:

Add optional timing attributes:
```rust
#[veecle_os_runtime::actor]
#[timing(priority = 10, deadline = "100ms", period = "50ms")]
async fn control_actor(...) -> Infallible {
    // Actor body
}
```

**Macro Responsibilities**:
1. Parse timing attributes
2. Emit LLVM metadata during compilation
3. Generate JSON manifest file for LALE
4. Validate constraints at compile time

**Implementation** (in `veecle-os-runtime-macros`):
```rust
#[proc_macro_attribute]
pub fn actor(attr: TokenStream, item: TokenStream) -> TokenStream {
    let timing_attrs = parse_timing_attributes(attr);
    let actor_fn = parse_actor_function(item);
    
    // Generate metadata emission code
    let metadata_code = generate_llvm_metadata(timing_attrs);
    
    // Generate original actor implementation + metadata
    quote! {
        #metadata_code
        #actor_fn
    }.into()
}
```

**Alternative**: Build script hook for post-compilation metadata injection.

#### Step 2.3: Metadata Parser in LALE

**Implementation**:
```cpp
class ActorMetadataParser {
public:
    struct ActorConstraints {
        std::string actor_name;
        uint32_t priority;
        uint64_t deadline_ns;
        uint64_t period_ns;
        ActivationType activation;  // PERIODIC, SPORADIC, APERIODIC
        uint32_t core_affinity;
    };
    
    std::map<std::string, ActorConstraints> parse_metadata(Module* M);
    ActorConstraints parse_json_config(std::string config_file);
};
```

**Data Flow**:
```
Rust Source + Annotations → Macro Expansion → LLVM IR with Metadata →
LALE Metadata Parser → Actor Constraints Database
```

---

### 5.3 Phase 3: Actor-Level WCET Analysis

**Goal**: Compute per-actor WCET considering all execution paths through segments.

#### Step 3.1: Actor WCET Calculation

**Model**: Actor as a state machine with timed segments.

**WCET Calculation Options**:

**Option 1: Maximum Segment WCET** (Conservative)
```
Actor_WCET = max(WCET(segment_i) for all segments i)
```
- Simple, very conservative
- Ignores actual control flow between segments

**Option 2: Path-Based Analysis** (More Precise)
```
Actor_WCET = WCET(worst_case_path_through_all_segments)
```
- Build complete state machine CFG
- Find longest path through all possible segments
- Consider loop iterations and branches

**Option 3: Response Time Analysis** (Most Precise)
```
Actor_Response_Time = f(WCET_per_segment, scheduling_policy, interference)
```
- For periodic actors: Consider all iterations within period
- Account for other actors' interference

**Recommended**: Start with Option 1, evolve to Option 2.

#### Step 3.2: ILP Formulation for Actor WCET

**Extend LALE's ILP-Based Path Analysis**:

**Variables**:
- `x_s`: Execution count of segment s
- `x_t`: Execution count of transition between segments

**Constraints**:
- Structural: Entry/exit flow balance per segment
- Control flow: Loop bounds within segments
- State machine: Valid state transitions only

**Objective**:
```
Maximize: Σ (WCET(segment_s) × x_s)
```

**Implementation**:
```cpp
class ActorWCETAnalyzer {
public:
    struct ActorWCETResult {
        uint64_t wcet_cycles;
        std::vector<int> worst_case_segment_sequence;
        std::map<int, uint64_t> per_segment_wcet;
    };
    
    ActorWCETResult compute_actor_wcet(
        ActorExecutionModel& actor,
        std::map<int, SegmentWCET>& segment_wcets,
        ActorConstraints& constraints
    );
    
private:
    void build_ilp_formulation(/* ... */);
    void add_state_machine_constraints(/* ... */);
};
```

#### Step 3.3: Loop Bound Annotations

**Challenge**: Actor loops (e.g., `loop { ... await ... }`) need bounds for WCET.

**Solution**: User annotations + static analysis

**Annotation Syntax**:
```rust
#[veecle_os_runtime::actor]
async fn sensor_actor(...) -> Infallible {
    #[veecle_wcet_loop_bound = 10]
    for i in 0..sensor_count {
        // Process sensor i
    }
    
    loop {  // Main loop - unbounded, analyze per-iteration
        sensor.wait_for_update().await;
        // WCET analysis: one iteration only
    }
}
```

**LALE Integration**:
- Parse annotations from LLVM metadata
- Override default loop bound analysis
- Emit warnings for unbounded loops without annotations

---

### 5.4 Phase 4: Multi-Core Schedulability Analysis

**Goal**: Support multi-core Veecle OS deployments with shared resource analysis.

#### Step 4.1: Multi-Core Actor Model

**Current Veecle OS**: Single-core focused, but architecture supports multi-core.

**Multi-Core Extension Requirements**:
1. **Core Assignment**: Static actor-to-core mapping
2. **Shared Resources**: Model shared memory/communication slots
3. **Interference**: Cache/bus contention between cores
4. **Synchronization**: Reader/Writer synchronization overhead

**Data Model**:
```cpp
struct MultiCoreActorSystem {
    std::vector<ProcessorCore> cores;
    std::map<ActorID, CoreID> actor_assignment;
    std::vector<SharedResource> shared_resources;
    std::map<SlotID, std::set<ActorID>> slot_access_map;
};

struct ProcessorCore {
    CoreID id;
    std::vector<ActorID> assigned_actors;
    SchedulingPolicy policy;  // Currently: COOPERATIVE_FIFO
};
```

#### Step 4.2: Interference Analysis

**Shared Resource Contention**:

For each shared slot (communication channel):
1. Identify all accessing actors (readers + writer)
2. Determine maximum interference delay
3. Add to per-actor WCET bounds

**Cache Interference** (if multi-core):
- Model shared L2/L3 cache
- Use existing LALE cache analysis with multi-core extension
- Add CRPD (Cache-Related Preemption Delay) if needed

**Implementation**:
```cpp
class InterferenceAnalyzer {
public:
    struct InterferenceDelay {
        uint64_t max_blocking_time_ns;
        uint64_t cache_interference_ns;
        uint64_t bus_contention_ns;
    };
    
    InterferenceDelay compute_interference(
        ActorID actor,
        MultiCoreActorSystem& system,
        std::map<ActorID, ActorWCETResult>& actor_wcets
    );
};
```

#### Step 4.3: Response Time Analysis

**For Periodic Actors**:

Use Response Time Analysis (RTA) from real-time scheduling theory:

```
R_i = C_i + Σ(⌈R_i / T_j⌉ × C_j)  for all higher-priority actors j
```

Where:
- `R_i`: Response time of actor i
- `C_i`: WCET of actor i
- `T_j`: Period of actor j
- Sum over all higher-priority actors on same core

**Fixed-Point Iteration**:
```cpp
uint64_t compute_response_time(
    ActorID actor,
    std::map<ActorID, ActorWCETResult>& wcets,
    std::map<ActorID, ActorConstraints>& constraints
) {
    uint64_t R = wcets[actor].wcet_cycles;
    uint64_t R_prev;
    
    do {
        R_prev = R;
        R = wcets[actor].wcet_cycles;
        
        // Add interference from higher-priority actors
        for (auto& [other_id, other_constraints] : constraints) {
            if (other_constraints.priority > constraints[actor].priority) {
                uint64_t interference = 
                    std::ceil((double)R_prev / other_constraints.period_ns) 
                    * wcets[other_id].wcet_cycles;
                R += interference;
            }
        }
    } while (R != R_prev && R <= constraints[actor].deadline_ns);
    
    return R;
}
```

**Schedulability Test**:
```
System is schedulable if: R_i ≤ D_i for all actors i
```

#### Step 4.4: Cooperative Scheduling Considerations

**Key Difference**: Veecle OS is cooperative, not preemptive.

**Implications**:
- No preemption overhead (no CRPD from context switches)
- Actors run to completion of segment (until next await)
- Simpler analysis: No unbounded priority inversion
- **Critical**: Segment WCET must be bounded and small

**Modified RTA** for cooperative scheduling:
```
R_i = C_i + Σ(blocking_segments_j)
```

Where blocking is only from:
1. Lower-priority actors already executing when i becomes ready
2. Shared resource access (slot writes)

**Simpler than preemptive**: No cascading preemptions.

---

### 5.5 Phase 5: System Integration and Output Generation

**Goal**: Integrate all components and produce required outputs for visualization and verification.

#### Step 5.1: LALE Architecture Extensions

**New Components to Add**:

```
lale/
├── lib/
│   ├── AsyncAnalysis/          # NEW
│   │   ├── AsyncDetector.cpp
│   │   ├── SegmentExtractor.cpp
│   │   └── ActorWCETAnalyzer.cpp
│   ├── ActorModel/              # NEW
│   │   ├── ActorMetadata.cpp
│   │   ├── CommunicationGraph.cpp
│   │   └── MultiCoreModel.cpp
│   ├── Schedulability/          # NEW
│   │   ├── ResponseTimeAnalysis.cpp
│   │   ├── InterferenceAnalysis.cpp
│   │   └── SchedulabilityTest.cpp
│   ├── Export/                  # NEW
│   │   ├── BTFExporter.cpp
│   │   ├── AMALTHEAExporter.cpp
│   │   └── ReportGenerator.cpp
│   ├── ControlFlowAnalysis/     # EXTEND
│   ├── MicroarchitecturalAnalysis/  # EXTEND
│   └── PathAnalysis/            # EXTEND
└── tools/
    └── veecle-wcet-analyzer/    # NEW CLI tool
```

**Modified Analysis Pipeline**:
```
LLVM IR Input
    ↓
Async Function Detection  ← NEW
    ↓
Segment Extraction  ← NEW
    ↓
Actor Metadata Parsing  ← NEW
    ↓
Per-Segment CFG Analysis (existing)
    ↓
Per-Segment Microarchitectural Analysis (existing)
    ↓
Per-Segment WCET (ILP, existing)
    ↓
Actor-Level WCET Composition  ← NEW
    ↓
Multi-Actor Schedulability Analysis  ← NEW
    ↓
Report & Trace Export  ← NEW
```

#### Step 5.2: BTF Trace Generation

**Implementation**:
```cpp
class BTFTraceExporter {
public:
    void export_trace(
        std::string output_file,
        MultiCoreActorSystem& system,
        std::map<ActorID, ActorWCETResult>& wcets,
        std::map<ActorID, uint64_t>& response_times
    );
    
private:
    void write_header(std::ofstream& out);
    void write_actor_definitions(/* ... */);
    void write_execution_trace(/* ... */);
};
```

**Output Format**:
```csv
#version 2.2.1
#creator LALE-Veecle_v1.0
#creationDate 2025-11-01 10:00:00
#timeScale ns

# Core definitions
0,Core_0,C,Core_0,0,

# Actor definitions (from metadata)
0,Core_0,T,ControlActor,0,"priority=10,deadline=100000000,wcet=15000000"
0,Core_0,T,SensorActor,0,"priority=5,deadline=200000000,wcet=25000000"

# Simulated execution trace (worst-case schedule)
0,Core_0,activate,ControlActor,0,
0,Core_0,start,ControlActor,0,
15000000,Core_0,terminate,ControlActor,0,
15000000,Core_0,activate,SensorActor,0,
15000000,Core_0,start,SensorActor,0,
40000000,Core_0,terminate,SensorActor,0,
50000000,Core_0,activate,ControlActor,0,
# ... continues for hyperperiod
```

**Trace Generation Strategy**:
- **Option 1**: Generate worst-case schedule trace
  - Use scheduling simulator with WCET values
  - Show worst-case interleaving
- **Option 2**: Symbolic trace with WCET bounds
  - Show actor activations with timing ranges
- **Option 3**: Import from actual execution traces
  - Annotate with WCET analysis results

#### Step 5.3: AMALTHEA Model Export

**Implementation**:
```cpp
class AMALTHEAExporter {
public:
    void export_model(
        std::string output_file,
        MultiCoreActorSystem& system,
        std::map<ActorID, ActorWCETResult>& wcets,
        std::map<ActorID, ActorConstraints>& constraints
    );
    
private:
    void generate_software_model(/* ... */);
    void generate_hardware_model(/* ... */);
    void generate_constraints_model(/* ... */);
    void generate_stimuli_model(/* ... */);
};
```

**XML Structure** (`.amxmi` file):
```xml
<?xml version="1.0" encoding="UTF-8"?>
<am:Amalthea xmi:version="2.0" 
             xmlns:xmi="http://www.omg.org/XMI"
             xmlns:am="http://app4mc.eclipse.org/amalthea/0.9.0">
  
  <swModel>
    <tasks name="ControlActor">
      <activityGraph>
        <items xsi:type="am:Ticks" default="15000000" extended="15000000"/>
      </activityGraph>
    </tasks>
    <!-- More actors -->
  </swModel>
  
  <hwModel>
    <system name="VeecleSystem">
      <ecus name="ECU_0">
        <microcontrollers name="MCU_0">
          <cores name="Core_0" frequency="1000000000"/>
        </microcontrollers>
      </ecus>
    </system>
  </hwModel>
  
  <constraintsModel>
    <timingConstraints name="ControlActorDeadline">
      <target xsi:type="am:Task" href="#//@swModel/@tasks.0"/>
      <upperLimit value="100000000" unit="ns"/>
    </timingConstraints>
  </constraintsModel>
  
  <stimuliModel>
    <periodicStimuli name="ControlActorStimulus" recurrence="50000000">
      <task href="#//@swModel/@tasks.0"/>
    </periodicStimuli>
  </stimuliModel>
  
</am:Amalthea>
```

#### Step 5.4: Report Generation

**HTML Report Structure**:

```html
<!DOCTYPE html>
<html>
<head><title>Veecle OS WCET Analysis Report</title></head>
<body>
  <h1>System Overview</h1>
  <table>
    <tr><th>Actor</th><th>WCET</th><th>Deadline</th><th>Response Time</th><th>Schedulable?</th></tr>
    <tr><td>ControlActor</td><td>15ms</td><td>100ms</td><td>18ms</td><td>✓ Yes</td></tr>
    <!-- ... -->
  </table>
  
  <h1>Per-Actor Details</h1>
  <h2>ControlActor</h2>
  <h3>Segment WCETs</h3>
  <ul>
    <li>Segment 0 (initial): 5ms</li>
    <li>Segment 1 (after sensor read): 10ms</li>
    <li>Segment 2 (after output write): 2ms</li>
  </ul>
  <h3>Worst-Case Path</h3>
  <pre>State 0 → State 3 → State 4 → State 0 (loop)</pre>
  
  <!-- Graphs, visualizations -->
</body>
</html>
```

**JSON Report** (machine-readable):
```json
{
  "system": {
    "name": "VeecleOSSystem",
    "cores": ["Core_0"],
    "actors": ["ControlActor", "SensorActor", "ActuatorActor"]
  },
  "actors": [
    {
      "name": "ControlActor",
      "priority": 10,
      "deadline_ns": 100000000,
      "wcet_ns": 15000000,
      "bcet_ns": 8000000,
      "response_time_ns": 18000000,
      "schedulable": true,
      "segments": [
        {"id": 0, "wcet_ns": 5000000},
        {"id": 1, "wcet_ns": 10000000},
        {"id": 2, "wcet_ns": 2000000}
      ]
    }
  ],
  "schedulability": {
    "system_schedulable": true,
    "utilization": 0.68,
    "hyperperiod_ns": 200000000
  }
}
```

---

### 5.6 Phase 6: Build System and Toolchain Integration

**Goal**: Seamless integration with Veecle OS build process.

#### Step 6.1: Cargo Integration

**Cargo Subcommand**: `cargo veecle-wcet`

```bash
# Analyze current project
cargo veecle-wcet analyze

# Generate reports
cargo veecle-wcet report --format html

# Export to chronVIEW
cargo veecle-wcet export --format btf --output trace.btf

# Check schedulability (fail build if not schedulable)
cargo veecle-wcet check --fail-on-violation
```

**Implementation**: Rust tool in `veecle-wcet-tool` crate:
```rust
// veecle-wcet-tool/src/main.rs
fn main() {
    let args = parse_args();
    
    // 1. Build with LLVM IR emission
    compile_with_llvm_ir();
    
    // 2. Run LALE analysis
    let lale_results = run_lale_analysis(&args);
    
    // 3. Generate outputs
    match args.command {
        Command::Analyze => generate_json_report(lale_results),
        Command::Report { format } => generate_report(lale_results, format),
        Command::Export { format, output } => export_trace(lale_results, format, output),
        Command::Check => check_schedulability(lale_results),
    }
}
```

#### Step 6.2: Build Script Integration

**In Veecle OS projects** (`build.rs`):
```rust
fn main() {
    // Enable LLVM IR emission
    println!("cargo:rustflags=--emit=llvm-ir");
    
    // Generate metadata JSON from actor annotations
    veecle_wcet::generate_actor_metadata();
    
    // Optionally run analysis during build
    if env::var("VEECLE_WCET_CHECK").is_ok() {
        veecle_wcet::run_analysis_and_check();
    }
}
```

#### Step 6.3: CI/CD Integration

**GitHub Actions Workflow**:
```yaml
name: WCET Analysis

on: [push, pull_request]

jobs:
  wcet-analysis:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      
      - name: Install LALE
        run: |
          curl -L https://lale-releases/lale-veecle-v1.0.tar.gz | tar xz
          export PATH=$PATH:$PWD/lale/bin
      
      - name: Build with WCET analysis
        run: cargo veecle-wcet analyze
      
      - name: Check schedulability
        run: cargo veecle-wcet check --fail-on-violation
      
      - name: Generate report
        run: cargo veecle-wcet report --format html --output wcet-report.html
      
      - name: Upload report
        uses: actions/upload-artifact@v2
        with:
          name: wcet-report
          path: wcet-report.html
```

---

## 6. Required Inputs Summary

### 6.1 From Veecle OS Compilation

**Automatically Generated**:
- ✅ LLVM IR (machine-level) of compiled actors
- ✅ Actor names and function signatures
- ✅ Communication graph (Reader/Writer dependencies)
- ✅ Static memory layout

**Via Macro Extensions**:
- Actor metadata (priorities, deadlines, periods) in LLVM metadata
- Loop bounds annotations
- Segment-level timing hints

### 6.2 From User Configuration

**Required Configuration** (`veecle-wcet-config.json`):
```json
{
  "hardware": {
    "processor": "ARM_Cortex_M7",
    "frequency_hz": 216000000,
    "cores": 1,
    "cache": {
      "l1_icache": {"size_kb": 16, "associativity": 4, "line_size": 32},
      "l1_dcache": {"size_kb": 16, "associativity": 4, "line_size": 32}
    },
    "memory": {
      "flash_latency_cycles": 3,
      "ram_latency_cycles": 1
    }
  },
  
  "actors": {
    "ControlActor": {
      "priority": 10,
      "deadline_ms": 100,
      "period_ms": 50,
      "core_affinity": 0
    },
    "SensorActor": {
      "priority": 5,
      "deadline_ms": 200,
      "period_ms": 100
    }
  },
  
  "analysis": {
    "optimization_level": "O2",
    "loop_bound_max_default": 100,
    "enable_cache_analysis": true,
    "enable_pipeline_analysis": true
  }
}
```

**Optional Annotations** (in Rust source):
```rust
#[veecle_os_runtime::actor]
#[timing(priority = 10, deadline = "100ms", period = "50ms")]
async fn control_actor(...) -> Infallible {
    #[veecle_wcet_loop_bound = 10]
    for sensor in sensors {
        // Process each sensor
    }
}
```

### 6.3 From External Sources

**Hardware Specifications**:
- Processor microarchitecture model (pipeline, cache parameters)
- Memory subsystem characteristics
- Bus/interconnect specifications (for multi-core)

**Timing Constraints**:
- System-level timing requirements
- Event chain specifications
- End-to-end latency requirements

---

## 7. Required Outputs Summary

### 7.1 Per-Actor WCET Metrics

**Essential Outputs**:
```json
{
  "actor_name": "ControlActor",
  "wcet_cycles": 32400000,
  "wcet_ns": 150000000,
  "bcet_cycles": 17280000,
  "bcet_ns": 80000000,
  "segments": [
    {"id": 0, "wcet_cycles": 10800000, "wcet_ns": 50000000},
    {"id": 1, "wcet_cycles": 21600000, "wcet_ns": 100000000}
  ],
  "worst_case_path": "State 0 → State 3 → State 4",
  "cache_misses_worst_case": 42,
  "stack_usage_bytes": 2048
}
```

### 7.2 Schedulability Metrics

**System-Level Results**:
```json
{
  "schedulable": true,
  "total_utilization": 0.72,
  "actors": [
    {
      "name": "ControlActor",
      "response_time_ns": 180000000,
      "deadline_ns": 100000000,
      "slack_ns": -80000000,  # Negative = exceeds deadline!
      "schedulable": false
    }
  ],
  "violations": [
    {"actor": "ControlActor", "type": "deadline_miss", "margin_ns": -80000000}
  ]
}
```

### 7.3 Visualization Outputs

**BTF Trace** (for chronVIEW):
- Actor execution traces with worst-case timing
- State transitions and await points
- Resource usage (cache, memory)

**AMALTHEA Model** (for chronSIM):
- System architecture model
- Task definitions with WCET/BCET
- Timing constraints
- Activation patterns

**HTML Report**:
- System overview with summary metrics
- Per-actor detailed analysis
- Gantt charts of worst-case schedule
- Violations and warnings

**JSON Report** (machine-readable):
- All metrics in structured format
- For integration with other tools
- CI/CD pipeline consumption

---

## 8. Integration Points with Veecle OS

### 8.1 Macro-Level Integration

**Extend `veecle-os-runtime-macros`**:

Add new procedural macro for timing annotations:
```rust
#[veecle_timing]
#[priority = 10]
#[deadline = "100ms"]
#[period = "50ms"]
#[veecle_os_runtime::actor]
async fn control_actor(...) -> Infallible {
    // Actor implementation
}
```

**Macro Responsibilities**:
1. Parse timing attributes
2. Validate constraints at compile time
3. Emit LLVM metadata
4. Generate actor metadata JSON
5. Preserve original actor functionality

### 8.2 Runtime Integration (Optional)

**For Runtime Monitoring** (optional feature):

```rust
// In veecle-os-runtime
#[cfg(feature = "wcet-monitoring")]
pub struct WCETMonitor {
    start_time: Instant,
    segment_times: Vec<Duration>,
}

impl WCETMonitor {
    pub fn start_segment(&mut self, segment_id: usize) { /* ... */ }
    pub fn end_segment(&mut self, segment_id: usize) { /* ... */ }
    pub fn check_budget(&self, segment_id: usize) -> bool { /* ... */ }
}
```

**Usage in Generated Code**:
```rust
#[veecle_os_runtime::actor]
async fn control_actor(...) -> Infallible {
    #[cfg(feature = "wcet-monitoring")]
    let mut monitor = WCETMonitor::new();
    
    loop {
        #[cfg(feature = "wcet-monitoring")]
        monitor.start_segment(0);
        
        // Actor segment 0
        sensor.wait_for_update().await;
        
        #[cfg(feature = "wcet-monitoring")]
        monitor.end_segment(0);
    }
}
```

### 8.3 Build System Integration

**Cargo Feature Flags**:
```toml
[features]
default = []
wcet-analysis = ["veecle-wcet-tool"]
wcet-monitoring = ["veecle-os-runtime/wcet-monitoring"]
```

**Build Process**:
```bash
# Normal build
cargo build --release

# Build with WCET analysis
cargo build --release --features wcet-analysis

# Build with runtime monitoring
cargo build --release --features wcet-monitoring
```

---

## 9. Changes Needed to LALE

### 9.1 Core Architecture Changes

**New Analysis Phase: Actor Analysis**
Insert between existing phases:
```
Current: CFG → Microarchitectural → Path Analysis
New:     CFG → Async Detection → Segment Extraction → 
         Microarchitectural (per segment) → Actor WCET → 
         Schedulability → Path Analysis
```

**New Interfaces**:

```cpp
// lib/AsyncAnalysis/AsyncAnalysisInterfaces.h

class ActorAnalysisDomain : public ContextAwareAnalysisDomain {
public:
    virtual ActorID getActorID() const = 0;
    virtual int getCurrentSegment() const = 0;
    virtual void setSegment(int segment) = 0;
};

class ActorSchedulingPolicy {
public:
    virtual uint64_t computeInterference(
        ActorID actor,
        std::vector<ActorID> higher_priority_actors,
        std::map<ActorID, uint64_t> wcets
    ) = 0;
};
```

### 9.2 Modified Components

**ControlFlowAnalysis**:
- Add segment boundary detection
- Respect state machine transitions
- Handle cross-segment flow

**MicroArchitecturalAnalysis**:
- Support per-segment analysis
- Reset cache state at segment boundaries (conservative)
- Track state machine register state

**PathAnalysis**:
- Extend ILP formulation for state machines
- Add state transition constraints
- Support multi-actor scenarios

### 9.3 New Components to Implement

**Component List**:
1. `AsyncFunctionDetector`: Identify Rust async functions
2. `ActorSegmentExtractor`: Extract execution segments
3. `ActorMetadataParser`: Load actor constraints
4. `ActorWCETAnalyzer`: Per-actor WCET calculation
5. `CommunicationGraphBuilder`: Build actor dependencies
6. `InterferenceAnalyzer`: Multi-actor interference
7. `ResponseTimeAnalyzer`: RTA for schedulability
8. `BTFExporter`: Generate BTF traces
9. `AMALTHEAExporter`: Generate AMALTHEA models
10. `ReportGenerator`: HTML/JSON reports

**Estimated Code Size**: ~10,000-15,000 lines of C++ (core analysis) + ~3,000 lines of Rust (tooling)

---

## 10. Data Structures and Metadata

### 10.1 Core Data Structures

**Actor Representation**:
```cpp
struct Actor {
    ActorID id;
    std::string name;
    llvm::Function* async_function;
    
    // Metadata
    ActorConstraints constraints;
    
    // Analysis results
    std::vector<ActorSegment> segments;
    ActorExecutionModel execution_model;
    ActorWCETResult wcet_result;
    
    // Dependencies
    std::set<SlotID> read_slots;
    std::set<SlotID> write_slots;
};
```

**Communication Graph**:
```cpp
struct CommunicationGraph {
    std::map<SlotID, Slot> slots;
    std::map<ActorID, Actor> actors;
    
    // Adjacency: actor → actors it can wake up
    std::map<ActorID, std::set<ActorID>> wakeup_graph;
    
    // Reverse: actor → actors that can wake it up
    std::map<ActorID, std::set<ActorID>> triggered_by_graph;
};

struct Slot {
    SlotID id;
    std::string type_name;
    ActorID writer;  // Single writer
    std::set<ActorID> readers;  // Multiple readers
};
```

**System Model**:
```cpp
struct VeecleOSSystem {
    std::string system_name;
    
    // Hardware
    std::vector<ProcessorCore> cores;
    HardwareModel hw_model;
    
    // Software
    std::map<ActorID, Actor> actors;
    CommunicationGraph comm_graph;
    
    // Mapping
    std::map<ActorID, CoreID> actor_assignment;
    
    // Analysis results
    std::map<ActorID, uint64_t> response_times;
    bool is_schedulable;
    double utilization;
};
```

### 10.2 Metadata Formats

**LLVM Metadata** (embedded in IR):
```llvm
!veecle.system = !{!"VeecleOSSystem", !veecle.actors}

!veecle.actors = !{
    !veecle.actor.control_actor,
    !veecle.actor.sensor_actor
}

!veecle.actor.control_actor = !{
    !"name", !"ControlActor",
    !"mangled_name", !"control_actor{{closure}}",
    !"priority", i32 10,
    !"deadline_ns", i64 100000000,
    !"period_ns", i64 50000000,
    !"reads", !veecle.slots.sensor_data,
    !"writes", !veecle.slots.control_cmd
}
```

**JSON Configuration**:
```json
{
  "veecle_system": {
    "name": "AutomotiveECU",
    "actors": [
      {
        "name": "ControlActor",
        "source_file": "src/actors/control.rs",
        "priority": 10,
        "deadline_ms": 100,
        "period_ms": 50,
        "reads": ["SensorData"],
        "writes": ["ControlCommand"]
      }
    ],
    "slots": [
      {
        "name": "SensorData",
        "type": "SensorReading",
        "size_bytes": 64
      }
    ]
  }
}
```

---

## 11. Limitations and Challenges

### 11.1 Technical Challenges

**Async Detection Reliability**:
- ⚠️ Pattern-based detection may have false positives/negatives
- ⚠️ Optimization can obscure state machine structure
- **Mitigation**: Prefer MIR-level analysis, use debug builds for analysis

**State Space Explosion**:
- ⚠️ Multi-actor systems have exponential state combinations
- ⚠️ Deep async call chains create complex state machines
- **Mitigation**: Compositional analysis, assume-guarantee reasoning

**Loop Bounds**:
- ⚠️ Dynamic loops require manual annotations
- ⚠️ Unbounded loops (common in actors) need per-iteration analysis
- **Mitigation**: User annotations, conservative bounds, warnings

**Optimization Effects**:
- ⚠️ LLVM optimizations can inline, eliminate, or transform async code
- ⚠️ Analyzed IR may differ from final binary
- **Mitigation**: Analyze at low optimization levels, verify with measurements

### 11.2 Veecle OS Specific Challenges

**No Built-in Priorities**:
- ⚠️ Veecle OS currently has no priority concept
- ⚠️ All actors treated equally in scheduling
- **Mitigation**: Extend runtime or analyze with priority annotations

**Cooperative Scheduling Assumptions**:
- ⚠️ Analysis assumes actors yield at await points
- ⚠️ Long-running segments can block entire system
- **Mitigation**: Enforce segment WCET budgets, emit warnings

**Dynamic Behavior**:
- ⚠️ Future Veecle OS versions may support dynamic actor creation
- ⚠️ Current analysis assumes static actor set
- **Mitigation**: Restrict analysis to static subset, document assumptions

### 11.3 LALE Infrastructure Limitations

**Single-Core Focus**:
- ⚠️ LALE designed for single-threaded programs
- ⚠️ Multi-core requires significant extension
- **Mitigation**: Start with single-core, extend incrementally

**Scalability**:
- ⚠️ LALE may not scale to large industrial systems
- ⚠️ Academic prototype performance characteristics
- **Mitigation**: Focus on critical actors, compositional analysis

**Hardware Model Limitations**:
- ⚠️ LALE uses simplified microarchitecture models
- ⚠️ No commercial processor support
- **Mitigation**: Validate with measurements, use conservative bounds

### 11.4 Integration Challenges

**LLVM Version Compatibility**:
- ⚠️ LALE requires custom LLVM patches
- ⚠️ May not be compatible with latest Rust toolchain
- **Mitigation**: Pin Rust version, maintain LLVM fork

**Build Complexity**:
- ⚠️ Adding WCET analysis increases build time
- ⚠️ Requires additional toolchain components
- **Mitigation**: Make analysis optional, cache results

**User Burden**:
- ⚠️ Requires timing annotations and configuration
- ⚠️ Learning curve for developers
- **Mitigation**: Provide defaults, clear documentation, examples

---

## 12. Recommended Implementation Roadmap

### Phase 1: Foundation (3-4 months)
**Deliverables**:
- ✅ Async function detection working
- ✅ Segment extraction for simple actors
- ✅ Per-segment WCET for single actor
- ✅ Basic JSON report output

**Milestone**: Analyze single Veecle OS actor, output WCET.

### Phase 2: Metadata & Multi-Actor (2-3 months)
**Deliverables**:
- ✅ Metadata parsing (LLVM + JSON)
- ✅ Actor-level WCET composition
- ✅ Communication graph builder
- ✅ Multi-actor analysis infrastructure

**Milestone**: Analyze multiple actors with dependencies.

### Phase 3: Schedulability (2-3 months)
**Deliverables**:
- ✅ Response time analysis
- ✅ Schedulability test
- ✅ Priority/deadline support
- ✅ BTF trace export

**Milestone**: Generate schedulability report, export to chronVIEW.

### Phase 4: Tooling & Integration (2-3 months)
**Deliverables**:
- ✅ Cargo subcommand (`cargo veecle-wcet`)
- ✅ Build script integration
- ✅ HTML report generation
- ✅ CI/CD templates

**Milestone**: Seamless integration with Veecle OS development.

### Phase 5: Advanced Features (3-4 months)
**Deliverables**:
- ✅ Multi-core support
- ✅ Interference analysis
- ✅ AMALTHEA export
- ✅ Runtime monitoring (optional)

**Milestone**: Full-featured WCET analysis toolkit.

**Total Estimated Time**: 12-17 months for complete implementation

---

## 13. Alternative Approaches

### 13.1 Hybrid Measurement-Based Approach

**Instead of pure static analysis**:
- Instrument actors with cycle counters
- Run test scenarios on target hardware
- Statistical analysis of measurements
- Use LALE for path analysis only

**Pros**: More accurate for complex code
**Cons**: Less formal, coverage concerns

### 13.2 MIR-Level Analysis

**Analyze at Rust MIR instead of LLVM IR**:
- State machine structure more explicit
- Closer to source semantics
- Better async visibility

**Pros**: Easier detection, clearer structure
**Cons**: Requires Rust compiler plugin, less representative of final code

### 13.3 Temporal Logic Verification

**Use model checker (UPPAAL, PRISM)**:
- Model actors as timed automata
- Verify timing properties formally
- Generate counter-examples

**Pros**: Strong formal guarantees
**Cons**: State explosion, requires different toolchain

---

## 14. Success Criteria

### Customer Requirements Achievement:

✅ **See WCETs of actors**
- Per-actor WCET in JSON/HTML reports
- Segment-level breakdown available
- Worst-case path visualization

✅ **Assign priorities and deadlines to actors**
- Via annotations or configuration
- Validated at compile time
- Enforced in schedulability analysis

✅ **Perform schedulability analysis**
- Response time analysis
- Deadline violation detection
- System utilization metrics

✅ **Integrate with tools like Inchron chronVIEW**
- BTF trace export
- AMALTHEA model export
- Compatible with industry standards

---

## 15. Conclusion

This implementation plan provides a comprehensive roadmap for extending LALE to support Veecle OS actor-based systems with WCET analysis. The approach is technically feasible but requires significant engineering effort (12-17 months).

**Key Success Factors**:
1. **Incremental development**: Start with single-actor analysis, build up to system-level
2. **Close Veecle OS integration**: Leverage macros and build system
3. **Standards compliance**: BTF and AMALTHEA for industry interoperability
4. **Practical focus**: Prioritize usable tooling over theoretical perfection

**Next Steps**:
1. Validate async detection on sample Veecle OS code
2. Prototype segment extraction for one actor
3. Extend `#[actor]` macro with timing attributes
4. Implement per-segment WCET using existing LALE
5. Iterate and expand to multi-actor scenarios

This plan enables formal timing verification for Veecle OS, supporting safety-critical automotive and industrial applications requiring DO-178C or ISO 26262 certification.