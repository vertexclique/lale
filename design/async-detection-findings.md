# Async Detection Findings from Veecle OS LLVM IR

## Date: 2025-11-01

## Summary
Analysis of compiled Veecle OS actor binaries reveals Rust async state machine patterns in LLVM IR that can be detected and analyzed for WCET.

## Files Analyzed
- `ral/veecle_service_mesh_actors-4380682ccdddf393.ll` (actor implementation)
- `ral/veecle_service_mesh_core-7e0650ea24ec85a3.ll` (core mesh)
- `ral/futures_util-d2cc80ffae74ea83.ll` (futures primitives)

## Key Findings

### 1. Async State Machine Structure

Rust async functions compile to state machines with distinct states:

```
DICompositeType: variant_part with discriminator (i8)
├── State 0: "Unresumed" - Initial state before first poll
├── State 1: "Returned" - Completed execution
├── State 2: "Panicked" - Error state
├── State 3: "Suspend0" - First await point
├── State 4: "Suspend1" - Second await point
└── ... (additional Suspend states for each await)
```

### 2. State Discriminant Pattern

**Entry Block Pattern:**
```llvm
%0 = load i8, ptr %state_ptr, align 8
switch i8 %0, label %default.unreachable [
    i8 0, label %bb_unresumed
    i8 1, label %bb_returned
    i8 2, label %bb_panicked
    i8 3, label %bb_suspend0
    i8 4, label %bb_suspend1
]
```

**Key Characteristics:**
- Discriminant is `i8` type (8-bit integer)
- Load from state pointer at function entry
- Switch instruction dispatches to state blocks
- Each state has dedicated basic block(s)

### 3. State Metadata in Debug Info

**Unresumed State:**
```llvm
!24919 = !DICompositeType(
    tag: DW_TAG_structure_type,
    name: "Unresumed",
    scope: !24913,
    size: 702016,
    align: 64,
    elements: !24920,
    identifier: "b74dcc492e5472534753e2ad7fd2c9ab"
)
```

**Suspend State:**
```llvm
!24931 = !DICompositeType(
    tag: DW_TAG_structure_type,
    name: "Suspend0",
    scope: !24913,
    size: 702016,
    align: 64,
    elements: !24932,
    identifier: "c25b44b9d1ca53627b08e0cea8506071"
)
```

### 4. State Transition Pattern

**Updating State (Await Point):**
```llvm
; Store new state value
store i8 3, ptr %state_discriminant, align 8

; Return Poll::Pending
ret i1 false
```

**Characteristics:**
- State update via `store i8 <new_state>, ptr %disc_ptr`
- Followed by return (Poll::Pending = false, Poll::Ready = true)
- Each suspend point increments state ID

### 5. Future::poll Signature

```llvm
define noundef zeroext i1 @"..Future..poll"(
    ptr noalias nocapture noundef align 1 %self,
    ptr noalias nocapture noundef align 4 %context
) {
    ; Returns i1: false = Pending, true = Ready
}
```

### 6. Segment Boundaries

**Segment Entry:** State dispatch target block
**Segment Exit:** 
- Store to state discriminant (await point)
- Return instruction (completion)
- Unreachable (panic)

**Example Segment (State 3 → State 4):**
```
bb_suspend0:
    ; Execute code segment
    ; ...
    ; Await point - transition to next state
    store i8 4, ptr %state_ptr
    ret i1 false  ; Poll::Pending
```

### 7. Detection Heuristics

**Strong Indicators (Confidence: 5/10):**
1. Entry block with `load i8` + `switch i8` pattern
2. Switch cases to blocks named with state patterns
3. Multiple `store i8` to same pointer (state updates)
4. Debug metadata with "Unresumed", "Suspend0", "Suspend1" names

**Medium Indicators (Confidence: 3/10):**
1. Function signature: `(ptr, ptr) -> i1`
2. Return type is boolean (Poll result)
3. Multiple basic blocks with similar structure

**Weak Indicators (Confidence: 2/10):**
1. Function name contains "poll" or "future"
2. References to generator types in metadata

### 8. State Machine Sizes

From analyzed binaries:
- Small async fn: 512 bits (64 bytes)
- Medium async fn: 39,104 bits (~4.8 KB)
- Large async fn: 702,016 bits (~86 KB)

Size includes:
- Self reference
- Captured variables
- Intermediate results
- State discriminant

### 9. Veecle OS Specific Patterns

**Actor Run Loop:**
```rust
// Compiles to state machine with:
// - State 0: Unresumed
// - State 3: Suspend0 (first await)
// - State 4: Suspend1 (second await)
async fn run(&mut self) {
    // Segment 0: Initial setup
    let config = self.get_config().await;  // → Suspend0
    
    // Segment 1: Process config
    self.process(config).await;  // → Suspend1
    
    // Segment 2: Cleanup
}
```

### 10. WCET Analysis Implications

**Per-Segment WCET:**
- Segment 0 (Unresumed → Suspend0): Initial execution
- Segment 1 (Suspend0 → Suspend1): Resume after first await
- Segment 2 (Suspend1 → Returned): Final segment

**Actor WCET = max(Segment WCETs)**
- Conservative: Sum of all segments
- Realistic: Maximum single segment (run-to-completion)

## Implementation Recommendations

### Updated Detection Algorithm

```rust
fn detect_async_state_machine(function: &Function) -> Option<AsyncInfo> {
    let entry_bb = function.basic_blocks.first()?;
    
    // 1. Look for state discriminant load
    let disc_load = entry_bb.instrs.iter()
        .find(|i| matches!(i, Instruction::Load(l) if is_i8_type(&l.dest)))?;
    
    // 2. Check for switch on loaded value
    let switch = match &entry_bb.term {
        Terminator::Switch(s) if s.operand == disc_load.dest => s,
        _ => return None,
    };
    
    // 3. Validate state pattern
    let has_unresumed = switch.dests.iter()
        .any(|(val, _)| matches!(val, Constant::Int { value: 0, .. }));
    
    let has_suspend = switch.dests.iter()
        .any(|(val, _)| matches!(val, Constant::Int { value: 3.., .. }));
    
    if has_unresumed && has_suspend && switch.dests.len() >= 3 {
        Some(extract_async_info(function, disc_load, switch))
    } else {
        None
    }
}
```

### Segment Extraction Strategy

1. **Start from each switch destination** (state entry point)
2. **BFS until state update or return**
3. **Track state transitions** via store instructions
4. **Build segment graph** for control flow

### WCET Calculation

```rust
fn calculate_segment_wcet(segment: &ActorSegment) -> Cycles {
    let mut total = 0;
    
    for block_name in &segment.blocks {
        let block = find_block(function, block_name);
        total += calculate_block_wcet(block, platform);
    }
    
    total
}

fn calculate_actor_wcet(segments: &[ActorSegment]) -> Cycles {
    // Conservative: max single segment (run-to-completion)
    segments.iter()
        .map(|s| calculate_segment_wcet(s))
        .max()
        .unwrap_or(0)
}
```

## Next Steps

1. **Implement enhanced detection** with switch pattern matching
2. **Test on all 4 Veecle OS binaries** (assembler, inspector, packager, parts_picker)
3. **Validate segment extraction** against known async boundaries
4. **Benchmark detection accuracy** and performance
5. **Document edge cases** (optimized IR, inlined functions)

## References

- Veecle OS Runtime: `/home/vclq/.cargo/registry/.../veecle-os-runtime-0.1.0/`
- Rust async lowering: State machine transformation
- LLVM IR switch instruction: Discriminant-based dispatch
