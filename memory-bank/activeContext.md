# Active Context

## Current State
Project is in initial development phase (v0.1.0). Core functionality is implemented and working. Both CLI and GUI applications are functional.

## Recent Work
Initial project exploration and memory bank creation completed. All core components have been documented.

## Active Focus
Understanding the complete LALE system architecture and workflow.

## Key Insights

### WCET Analysis Approach
LALE uses IPET (Implicit Path Enumeration Technique) with Integer Linear Programming to calculate WCET. This is mathematically sound and provides guaranteed upper bounds.

### Critical Path: LLVM IR → WCET
1. Parse .ll files using `llvm-ir` crate
2. Build CFG (Control Flow Graph) from functions
3. Detect loops and extract bounds
4. Calculate instruction timings per platform
5. Formulate ILP problem with constraints
6. Solve using CBC solver
7. Extract WCET from solution

### Platform Models
13 platforms supported with different CPU frequencies and instruction timings:
- ARM Cortex-M (embedded): 48-400 MHz
- ARM Cortex-R (real-time): 600-800 MHz  
- ARM Cortex-A (application): 1200-1400 MHz
- RISC-V: 100-1500 MHz

### Scheduling Policies
Two approaches implemented:
- **RMA** (Rate Monotonic): Static priority based on period
- **EDF** (Earliest Deadline First): Dynamic priority based on deadline

### GUI Architecture
LALE Prism uses Tauri 2.0 (not Electron):
- Rust backend wraps lale library
- React + TypeScript frontend
- IPC via Tauri commands
- Much smaller binary size (~5-10MB)
- Native performance

## Important Patterns

### Conservative Analysis
When uncertain, LALE defaults to conservative estimates:
- Unknown loop bounds → 100 iterations
- Missing timing data → safe defaults
- Unparseable files → skip gracefully

### Error Handling
- User-facing: `Result<T, String>`
- Internal: `anyhow::Result<T>` with context
- Graceful degradation preferred over hard failures

### Stateless Design
No persistent state between analyses. Each analysis is independent. This simplifies the design but means no caching across runs.

## Project Conventions

### Code Organization
- Modules are feature-based (analysis, ir, platform, scheduling, output)
- Public API exposed through `lib.rs`
- CLI binary in `main.rs`
- Tests co-located with implementation

### Naming
- Functions: snake_case
- Types: PascalCase
- Constants: SCREAMING_SNAKE_CASE
- Modules: snake_case

### Documentation
- Public APIs have doc comments
- Complex algorithms explained inline
- Examples in doc tests where appropriate

## Development Workflow

### CLI Testing
```bash
cd lale
cargo run -- analyze ./data/armv7e-m --platform cortex-m4 --auto-tasks
```

### GUI Testing
```bash
cd laleprism
./run.sh  # Linux (handles Wayland issues)
# or
cargo tauri dev
```

### Linux Wayland Workaround
```bash
WEBKIT_DISABLE_COMPOSITING_MODE=1 GDK_BACKEND=x11 cargo tauri dev
```
This is baked into `run.sh` and `main.rs` for convenience.

## Next Steps (Potential)
Based on the codebase structure, potential areas for enhancement:
1. Parallel module parsing for performance
2. Custom platform definition support
3. Cache simulation for more accurate timing
4. Inter-procedural analysis
5. GUI improvements (PDF export, batch comparison)
6. More sophisticated loop bound analysis

## Known Issues
None documented yet. Project appears to be in working state.

## Dependencies to Watch
- `llvm-ir` - Must stay compatible with LLVM 17
- `good_lp` - ILP solver, critical for WCET calculation
- `tauri` - GUI framework, major version updates may require changes
- `petgraph` - Graph library, stable API

## Testing Approach
- Unit tests in each module
- Integration tests in `lale/tests/`
- Sample IR files in `data/` directory
- Manual GUI testing (no automated GUI tests)

## Performance Notes
- Analysis is fast (<5s for 1000 functions)
- ILP solver is the bottleneck
- Memory usage is reasonable (<200MB)
- Binary size is small (2-10MB)

## User Interaction Patterns

### CLI Users
Expect command-line interface with flags and options. Output is JSON for integration with other tools.

### GUI Users
Expect visual interface with:
- Directory picker
- Platform dropdown
- Task configuration
- Interactive Gantt chart
- Schedule persistence

## Symbol Demangling
Important for readability:
- Rust symbols: `rustc-demangle`
- C++ symbols: `cpp_demangle`
- Fallback to original if demangling fails
- Applied in GUI for display purposes
