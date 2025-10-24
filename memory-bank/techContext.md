# Technical Context

## Technology Stack

### Core Language
**Rust** (Edition 2021)
- Memory safety without garbage collection
- Zero-cost abstractions
- Excellent performance for systems programming
- Strong type system
- Cargo build system and package manager

### Key Dependencies

#### LALE Library
```toml
llvm-ir = "0.11"              # LLVM IR parsing (LLVM 17)
petgraph = "0.8"              # Graph data structures (CFG, CallGraph)
serde = "1.0"                 # Serialization/deserialization
serde_json = "1.0"            # JSON output
good_lp = "1.14"              # Linear programming (ILP solver)
  └─ coin_cbc                 # CBC solver backend
thiserror = "1.0"             # Error type derivation
anyhow = "1.0"                # Error handling with context
chrono = "0.4"                # Timestamp generation
ahash = "0.8"                 # Fast hash maps
```

#### LALE Prism
```toml
tauri = "2.0"                 # Desktop app framework
  └─ protocol-asset           # Asset serving
tauri-plugin-dialog = "2.0"   # File dialogs
tauri-plugin-fs = "2.0"       # Filesystem access
tauri-plugin-shell = "2.0"    # Shell commands
rustc-demangle = "0.1"        # Rust symbol demangling
cpp_demangle = "0.4"          # C++ symbol demangling
uuid = "1.0"                  # Unique identifiers
dirs = "5.0"                  # Platform directories
```

#### Frontend
```json
"react": "^18.x"              # UI framework
"typescript": "^5.x"          # Type safety
"vite": "^5.x"                # Build tool
"tailwindcss": "^3.x"         # CSS framework (likely)
```

## Development Environment

### Required Tools
- **Rust**: Latest stable (rustup recommended)
- **Node.js**: v18+ (for frontend)
- **npm/yarn**: Package management
- **LLVM**: For generating .ll files (not runtime dependency)

### Platform-Specific Requirements

#### Linux
```bash
libwebkit2gtk-4.1-dev         # WebView
build-essential               # C/C++ compiler
libxdo-dev                    # X11 automation
libssl-dev                    # SSL/TLS
libayatana-appindicator3-dev  # System tray
librsvg2-dev                  # SVG rendering
```

#### macOS
- Xcode Command Line Tools

#### Windows
- Microsoft Visual Studio C++ Build Tools
- WebView2 Runtime

### Build Configuration

#### Release Profile
```toml
opt-level = 3        # Maximum optimization
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit for better optimization
strip = true         # Strip symbols for smaller binary
```

#### Development Profile
```toml
opt-level = 0        # No optimization (fast compilation)
debug = true         # Debug symbols
```

## Technical Constraints

### LLVM IR Format
- Must support LLVM 17 IR format
- Parses textual .ll files (not bitcode .bc)
- Requires valid LLVM IR (no syntax errors)
- Function-level analysis (not module-level optimizations)

### ILP Solver
- Uses CBC (COIN-OR Branch and Cut) solver
- Requires `coin_cbc` feature in `good_lp`
- Solver must be available at runtime
- May have performance limits on very large CFGs

### Platform Models
- Timing models are approximations
- No cache simulation (conservative estimates)
- No pipeline modeling
- No memory hierarchy modeling
- Assumes single-core execution

### GUI Framework (Tauri)
- WebView-based (not Electron)
- Smaller binary size (~5-10MB vs 100MB+)
- Native performance
- Platform-specific WebView requirements
- IPC between Rust backend and JS frontend

## File Formats

### Input: LLVM IR (.ll)
```llvm
; Example LLVM IR
define i32 @add(i32 %a, i32 %b) {
entry:
  %sum = add i32 %a, %b
  ret i32 %sum
}
```

### Output: JSON Report
```json
{
  "timestamp": "2025-01-15T20:00:00Z",
  "platform": "ARM Cortex-M4",
  "cpu_frequency_mhz": 168,
  "wcet_results": {
    "function_name": 1234
  },
  "tasks": [...],
  "schedulability": {...},
  "schedule": {...}
}
```

## Development Workflow

### CLI Development
```bash
cd lale
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run tests
cargo run -- analyze <dir> --platform cortex-m4
```

### GUI Development
```bash
cd laleprism

# Frontend only
cd frontend
npm install
npm run dev              # Vite dev server

# Full application
cd ..
cargo tauri dev          # Development mode with hot-reload
cargo tauri build        # Production build
```

### Testing Strategy
- Unit tests in each module
- Integration tests in `lale/tests/`
- Sample IR files for testing
- No GUI tests currently (manual testing)

## Performance Characteristics

### Analysis Speed
- Single function: <100ms typical
- 1000 functions: <5s target
- Dominated by ILP solver time
- CFG construction is fast (petgraph)

### Memory Usage
- <200MB typical for large projects
- Graph structures are memory-efficient
- No persistent caching (stateless analysis)

### Binary Size
- lale CLI: ~2-3MB (stripped)
- laleprism: ~5-10MB (stripped)
- Much smaller than Electron alternatives

## Tool Integration

### Generating LLVM IR
```bash
# Rust
rustc --emit=llvm-ir source.rs

# C/C++
clang -S -emit-llvm source.c -o source.ll

# With optimizations
clang -O2 -S -emit-llvm source.c -o source.ll
```

### Script Helper
```bash
# lale/scripts/emit_llvm_ir.sh
# Automates IR generation for projects
```

## Known Limitations

### Technical
1. No inter-procedural analysis (function-level only)
2. Conservative loop bounds (default 100 if unknown)
3. No cache modeling (worst-case assumptions)
4. No pipeline modeling
5. Single-threaded analysis (no parallelization yet)

### Platform Support
1. Limited to 13 predefined platforms
2. No custom platform definition (yet)
3. Fixed CPU frequencies (no runtime configuration)
4. Simplified instruction timing models

### GUI
1. No batch comparison of schedules
2. No PDF export
3. No real-time monitoring
4. Limited visualization customization

## Future Technical Considerations

### Potential Improvements
- Parallel module parsing
- Custom platform definition
- Cache simulation
- Inter-procedural analysis
- More sophisticated loop bound analysis
- GPU acceleration for ILP solving

### Scalability
- Current design handles 1000s of functions
- ILP solver may struggle with very large CFGs (>10k blocks)
- Memory usage scales linearly with code size
- No distributed analysis support
