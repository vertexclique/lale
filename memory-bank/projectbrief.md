# Project Brief

## Project Identity
**Name**: LALE (LLVM Analysis Latency Engine)  
**Repository**: https://github.com/vertexclique/lale  
**Version**: 0.1.0  
**License**: MIT OR Apache-2.0

## Core Purpose
LALE is a Worst-Case Execution Time (WCET) analysis tool for embedded systems that analyzes LLVM IR files to determine timing guarantees and generate real-time schedules.

## Project Structure
Workspace with two main components:
1. **lale** - Core library for WCET analysis
2. **laleprism** - Desktop GUI application (Tauri-based)

## Primary Goals
1. Analyze LLVM IR files to calculate WCET for functions
2. Support multiple embedded platforms (ARM Cortex-M/R/A, RISC-V)
3. Perform schedulability analysis (RMA/EDF)
4. Generate static schedules for real-time tasks
5. Provide interactive visualization through desktop GUI

## Target Users
- Embedded systems developers
- Real-time systems engineers
- Safety-critical software developers
- Researchers in timing analysis

## Key Requirements
- Parse LLVM IR files (.ll format)
- Build Control Flow Graphs (CFG)
- Detect and analyze loops
- Calculate instruction timings per platform
- Solve WCET using IPET (Integer Linear Programming)
- Generate schedulable task sets
- Visualize schedules with Gantt charts
- Support symbol demangling (Rust/C++/C)

## Technical Constraints
- Must work with LLVM 17 IR format
- Requires ILP solver (coin_cbc)
- Cross-platform support (Linux/macOS/Windows)
- Desktop application using Tauri 2.0
- Frontend: React + TypeScript + Vite
