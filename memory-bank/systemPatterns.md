# System Patterns

## Architecture Overview

### Workspace Structure
```
lale/                    # Root workspace
├── lale/               # Core library
│   └── src/
│       ├── lib.rs      # Public API
│       ├── main.rs     # CLI binary
│       ├── analysis/   # WCET analysis
│       ├── ir/         # LLVM IR parsing
│       ├── output/     # Report generation
│       ├── platform/   # Platform models
│       ├── scheduling/ # Schedulability
│       └── wcet/       # WCET utilities
└── laleprism/          # GUI application
    ├── src/            # Rust backend
    └── frontend/       # React frontend
```

## Core Analysis Pipeline

### WCET Analysis Flow
```
LLVM IR File (.ll)
    ↓
IRParser::parse_file()
    ↓
Module (llvm_ir::Module)
    ↓
CFG::from_function()
    ↓
Control Flow Graph
    ↓
LoopAnalyzer::analyze_loops()
    ↓
Loop Information + Bounds
    ↓
TimingCalculator::calculate_block_timings()
    ↓
Block Timings (Cycles)
    ↓
IPETSolver::solve_wcet()
    ↓
WCET (cycles)
```

### Key Components

#### 1. IR Module (`lale/src/ir/`)
**Purpose**: Parse and represent LLVM IR

**IRParser**
- `parse_file()` - Parse single .ll file
- `parse_files()` - Batch parse multiple files
- Uses `llvm-ir` crate for parsing

**CFG (Control Flow Graph)**
- `from_function()` - Build CFG from LLVM function
- Represents basic blocks as graph nodes
- Edges represent control flow
- Tracks entry block and exit blocks

**CallGraph**
- `from_module()` - Build call graph from module
- Tracks caller-callee relationships
- Detects recursive calls

#### 2. Analysis Module (`lale/src/analysis/`)
**Purpose**: Core WCET calculation

**LoopAnalyzer**
- Detects loops via back-edge detection
- Extracts loop bounds (constant, induction variable, pattern matching)
- Computes nesting levels
- Conservative default: 100 iterations if unknown

**TimingCalculator**
- Maps LLVM instructions to cycle counts
- Platform-specific timing models
- Handles terminators (branches, returns)
- Conversion utilities (cycles ↔ microseconds)

**IPETSolver** (Critical Component)
- Uses Integer Linear Programming (ILP)
- Formulates WCET as optimization problem
- Constraints:
  - Entry block executes exactly once
  - Flow conservation (incoming = outgoing)
  - Loop bounds (body ≤ max_iterations × header)
- Maximizes: Σ(execution_count × block_timing)
- Uses `good_lp` crate with CBC solver

**Cycles**
- Represents timing with best/worst case
- Handles timing ranges

#### 3. Platform Module (`lale/src/platform/`)
**Purpose**: Hardware-specific timing models

**PlatformModel Structure**
```rust
pub struct PlatformModel {
    pub name: String,
    pub cpu_frequency_mhz: u32,
    pub instruction_timings: InstructionTimings,
    pub cache_config: Option<CacheConfig>,
}
```

**Supported Platforms**
- ARM Cortex-M: M0 (48MHz), M3 (72MHz), M4 (168MHz), M7 (400MHz), M33 (120MHz)
- ARM Cortex-R: R4 (600MHz), R5 (800MHz)
- ARM Cortex-A: A7 (1200MHz), A53 (1400MHz)
- RISC-V: RV32I (100MHz), RV32IMAC (320MHz), RV32GC (1000MHz), RV64GC (1500MHz)

#### 4. Scheduling Module (`lale/src/scheduling/`)
**Purpose**: Real-time schedulability analysis

**Task Structure**
```rust
pub struct Task {
    pub name: String,
    pub function: String,
    pub wcet_cycles: u64,
    pub wcet_us: f64,
    pub period_us: Option<f64>,
    pub deadline_us: Option<f64>,
    pub priority: Option<u8>,
    pub preemptible: bool,
    pub dependencies: Vec<String>,
}
```

**RMAScheduler** (Rate Monotonic Analysis)
- Priority = 1/period (shorter period = higher priority)
- Utilization test: U = Σ(WCET/Period) ≤ n(2^(1/n) - 1)
- Exact test: Response time analysis

**EDFScheduler** (Earliest Deadline First)
- Dynamic priority based on absolute deadline
- Utilization test: U = Σ(WCET/Period) ≤ 1

**StaticScheduleGenerator**
- Generates timeline of task executions
- Hyperperiod calculation: LCM of all periods
- Produces Gantt chart data

#### 5. Output Module (`lale/src/output/`)
**Purpose**: Report generation and visualization

**AnalysisReport Structure**
```rust
pub struct AnalysisReport {
    pub timestamp: String,
    pub platform: String,
    pub cpu_frequency_mhz: u32,
    pub wcet_results: HashMap<String, u64>,
    pub tasks: Vec<Task>,
    pub schedulability: SchedulabilityResult,
    pub schedule: Option<Schedule>,
}
```

**JSONOutput**
- Serializes reports to JSON
- Used for CLI output and GUI communication

**GraphvizOutput**
- Generates DOT format for CFG visualization

**GanttOutput**
- Produces Gantt chart data for schedule visualization

## LALE Prism Architecture

### Backend (Tauri)
**Commands** (`laleprism/src/commands.rs`)
- IPC handlers for frontend communication
- Wraps lale library functionality
- Manages application state

**Analysis** (`laleprism/src/analysis.rs`)
- Adapts lale API for GUI use
- Handles directory scanning
- Provides platform list

**Demangler** (`laleprism/src/demangler.rs`)
- Rust symbol demangling (rustc-demangle)
- C++ symbol demangling (cpp_demangle)
- Fallback to original name

**Storage** (`laleprism/src/storage.rs`)
- Persists analysis results
- Schedule metadata management
- Uses local filesystem storage

### Frontend (React + TypeScript)
- Vite build system
- Tauri API integration
- Component-based UI
- Interactive visualizations

## Design Patterns

### 1. Builder Pattern
Used in platform models and configuration

### 2. Strategy Pattern
Scheduling policies (RMA vs EDF) are interchangeable

### 3. Pipeline Pattern
Analysis flows through distinct stages

### 4. Repository Pattern
Storage abstraction for schedule persistence

### 5. Facade Pattern
`WCETAnalyzer` provides simplified API over complex subsystems

## Critical Implementation Paths

### Path 1: Single Function WCET
```
Function → CFG → Loops → Timings → IPET → WCET
```

### Path 2: Module Analysis
```
Module → [Functions] → [WCETs] → Task Mapping
```

### Path 3: Complete Pipeline
```
Directory → [Modules] → [WCETs] → Tasks → Schedulability → Schedule → Report
```

### Path 4: GUI Analysis
```
User Config → Tauri Command → lale Analysis → JSON Report → Frontend Display
```

## Error Handling Strategy
- `Result<T, String>` for user-facing errors
- `anyhow::Result<T>` for internal errors with context
- Conservative defaults when analysis uncertain
- Graceful degradation (skip unparseable files)

## Performance Considerations
- Lazy evaluation where possible
- Caching of CFG and loop analysis
- Parallel module parsing (potential future optimization)
- ILP solver timeout handling
- Memory-efficient graph representations (petgraph)
