# Inkwell Migration Plan

## Objective
Migrate WCET analysis pipeline from `llvm-ir` crate to `inkwell` to support LLVM 19+ IR syntax.

## Current Architecture

### Dependencies
- **llvm-ir v0.11**: Parses LLVM IR text → `llvm_ir::Module`, `llvm_ir::Function`
- **inkwell v0.6**: Provides LLVM 18 API via C++ bindings

### Analysis Pipeline
```
IR File → IRParser (llvm-ir) → llvm_ir::Function
                                      ↓
                                SegmentExtractor
                                      ↓
                                SegmentWCETAnalyzer
                                      ↓
                                  CFG Builder
                                      ↓
                              TimingCalculator
                                      ↓
                                 IPETSolver
                                      ↓
                                WCET Result
```

## Problem
- LLVM 19+ introduces new syntax (`samesign`, new debug intrinsics)
- llvm-ir v0.11 parser rejects this syntax
- Rust nightly now generates LLVM 19 IR by default

## Solution Approaches

### Option 1: Hybrid Approach (Current - IMPLEMENTED)
**Status**: ✅ Working

Keep both parsers:
- Use `InkwellParser` for async detection (works with LLVM 19)
- Use `IRParser` for WCET analysis (falls back to estimates on LLVM 19)

**Pros**:
- Minimal code changes
- Works immediately
- Graceful degradation

**Cons**:
- WCET estimates are inaccurate for LLVM 19+ IR
- Maintains dual parser complexity

### Option 2: Full Inkwell Migration (PROPOSED)
**Status**: 🚧 Not started

Rewrite entire pipeline to use inkwell types.

#### Components to Migrate

##### 1. InkwellSegmentExtractor
```rust
pub struct InkwellSegmentExtractor;

impl InkwellSegmentExtractor {
    pub fn extract_segments(
        function: &inkwell::values::FunctionValue,
        async_info: &AsyncFunctionInfo,
    ) -> Vec<ActorSegment> {
        // Extract segments from inkwell function
        // Use function.get_basic_blocks()
        // Analyze control flow via inkwell API
    }
}
```

##### 2. InkwellCFG
```rust
pub struct InkwellCFG<'ctx> {
    pub blocks: Vec<InkwellBasicBlock<'ctx>>,
    pub edges: Vec<(usize, usize)>,
}

impl<'ctx> InkwellCFG<'ctx> {
    pub fn from_function(function: &inkwell::values::FunctionValue<'ctx>) -> Self {
        // Build CFG from inkwell function
        // Iterate basic blocks
        // Extract terminators for edges
    }
}
```

##### 3. InkwellTimingCalculator
```rust
pub struct InkwellTimingCalculator;

impl InkwellTimingCalculator {
    pub fn calculate_block_timings(
        function: &inkwell::values::FunctionValue,
        cfg: &InkwellCFG,
        platform: &PlatformModel,
    ) -> AHashMap<usize, u64> {
        // Calculate timing for each basic block
        // Iterate instructions via inkwell API
        // Use platform model for instruction costs
    }
}
```

##### 4. InkwellSegmentWCETAnalyzer
```rust
pub struct InkwellSegmentWCETAnalyzer {
    platform: PlatformModel,
}

impl InkwellSegmentWCETAnalyzer {
    pub fn analyze_segments(
        &self,
        function: &inkwell::values::FunctionValue,
        segments: &[ActorSegment],
    ) -> AHashMap<usize, SegmentWCET> {
        // Use InkwellCFG
        // Use InkwellTimingCalculator
        // Use existing IPETSolver (works with any CFG)
    }
}
```

#### Migration Steps

1. **Phase 1: Core Infrastructure**
   - [ ] Create `InkwellCFG` struct
   - [ ] Implement CFG builder from inkwell function
   - [ ] Add tests comparing with llvm-ir CFG

2. **Phase 2: Timing Analysis**
   - [ ] Create `InkwellTimingCalculator`
   - [ ] Implement instruction timing via inkwell
   - [ ] Handle all instruction opcodes
   - [ ] Add tests for timing accuracy

3. **Phase 3: Segment Extraction**
   - [ ] Create `InkwellSegmentExtractor`
   - [ ] Extract segments from inkwell function
   - [ ] Map state blocks to basic blocks
   - [ ] Add tests for segment extraction

4. **Phase 4: WCET Analysis**
   - [ ] Create `InkwellSegmentWCETAnalyzer`
   - [ ] Integrate with InkwellCFG
   - [ ] Integrate with InkwellTimingCalculator
   - [ ] Add end-to-end tests

5. **Phase 5: Integration**
   - [ ] Update `ActorAnalyzer` to use inkwell pipeline
   - [ ] Remove fallback to estimates
   - [ ] Update all call sites
   - [ ] Comprehensive testing

6. **Phase 6: Cleanup**
   - [ ] Mark llvm-ir pipeline as deprecated
   - [ ] Update documentation
   - [ ] Consider removing llvm-ir dependency

#### Challenges

1. **API Differences**
   - inkwell uses LLVM C++ API (more verbose)
   - llvm-ir provides Rust-native types (cleaner)
   - Need to handle lifetimes carefully

2. **Instruction Handling**
   - inkwell exposes raw LLVM instructions
   - Need to map opcodes to timing costs
   - More complex than llvm-ir's typed instructions

3. **Testing**
   - Need comprehensive test suite
   - Compare results with llvm-ir pipeline
   - Ensure accuracy maintained

4. **Maintenance**
   - inkwell updates with LLVM versions
   - Need to track LLVM API changes
   - More maintenance burden than llvm-ir

### Option 3: Wait for llvm-ir v0.12
**Status**: ⏳ Waiting

Wait for llvm-ir crate to add LLVM 19 support.

**Pros**:
- No code changes needed
- Maintains clean architecture
- Proven parser

**Cons**:
- Unknown timeline
- May take months
- Blocks LLVM 19 support

## Recommendation

**Short term**: Keep Option 1 (Hybrid - already implemented)
- Works now
- Unblocks development
- Provides basic functionality

**Long term**: Evaluate Option 2 vs Option 3
- Monitor llvm-ir development
- If no progress in 3 months → start Option 2
- If llvm-ir v0.12 releases → upgrade and remove fallback

## Estimated Effort

**Option 2 (Full Migration)**:
- Phase 1-2: 2-3 days (CFG + Timing)
- Phase 3-4: 2-3 days (Segments + WCET)
- Phase 5-6: 1-2 days (Integration + Cleanup)
- **Total**: 5-8 days of focused development

## Current Status

✅ **Implemented**: Option 1 (Hybrid Approach)
- Async detection works with LLVM 19
- WCET falls back to estimates
- GUI doesn't crash
- Analysis completes successfully

🚧 **Next Steps**: Monitor llvm-ir development or start Option 2 if needed
