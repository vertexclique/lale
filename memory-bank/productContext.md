# Product Context

## Problem Space

### Core Problem
Embedded systems require timing guarantees to ensure correct operation. Developers need to:
- Prove that tasks meet their deadlines
- Understand worst-case execution paths
- Generate schedules that guarantee real-time constraints
- Analyze timing behavior across different hardware platforms

### Current Challenges
1. Manual timing analysis is error-prone and time-consuming
2. Different platforms have vastly different timing characteristics
3. Complex control flow makes WCET calculation difficult
4. Loop bounds are often unknown or unbounded
5. Schedulability analysis requires mathematical expertise
6. Existing tools are expensive or platform-specific

## Solution Approach

### LALE Library
Automated WCET analysis pipeline:
1. **Parse** LLVM IR files (platform-independent representation)
2. **Build** Control Flow Graphs to understand execution paths
3. **Analyze** loops to determine iteration bounds
4. **Calculate** instruction timings based on target platform
5. **Solve** WCET using IPET (Integer Linear Programming)
6. **Test** schedulability using RMA or EDF algorithms
7. **Generate** static schedules for task execution

### LALE Prism GUI
Desktop application providing:
- Directory selection for batch analysis
- Platform configuration (13 supported platforms)
- Task configuration (manual or auto-generated)
- Interactive Gantt chart visualization
- Symbol demangling for readable function names
- Schedule persistence and comparison
- Export capabilities

## User Experience Goals

### For CLI Users (lale)
- Simple command-line interface
- Batch processing of IR files
- Flexible task configuration
- JSON output for integration
- Clear error messages

### For GUI Users (laleprism)
- Intuitive directory selection
- Visual platform selection
- Interactive task configuration
- Real-time analysis feedback
- Beautiful schedule visualization
- Easy schedule management

## Value Proposition

### Speed
- Automated analysis in seconds
- Batch processing of multiple functions
- No manual calculation required

### Accuracy
- Mathematical guarantees via ILP
- Platform-specific timing models
- Conservative WCET estimates

### Flexibility
- 13 platform models (ARM, RISC-V)
- Multiple scheduling policies
- Auto-task generation
- Custom task configuration

### Accessibility
- Free and open-source
- Cross-platform support
- No specialized hardware required
- Desktop GUI for non-experts

## Success Metrics
- Analysis completes in <5s for 1000 functions
- Memory usage <200MB
- Accurate WCET bounds (conservative but tight)
- Schedulability correctly determined
- User-friendly visualization
- Cross-platform compatibility
