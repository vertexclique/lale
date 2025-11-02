# LLVM Version Compatibility

## Issue

The Veecle OS binaries in `ral/` are compiled with Rust 1.89 (LLVM 18+) which uses newer LLVM IR syntax:

```llvm
# LLVM 18+ syntax (NOT supported by llvm-ir 0.8)
define dso_local noundef range(i8 0, 16) i8 @func(...)
    #dbg_value(ptr %self, !38, !DIExpression(), !39)
```

The `llvm-ir` crate (v0.8) only supports LLVM 14 syntax.

## Incompatible Features

1. **`range` attribute**: `range(i8 0, 16)` - return value range metadata
2. **`#dbg_value` intrinsic**: New debug info format
3. **Other LLVM 15+ features**

## Solutions

### Option 1: Downgrade Rust Toolchain (Temporary)

Compile test binaries with older Rust/LLVM:

```bash
# Use Rust 1.70 (LLVM 14)
rustup install 1.70
cd veecle-project
cargo +1.70 build --release --target armv7r-none-eabihf
cargo +1.70 rustc --release -- --emit=llvm-ir
```

### Option 2: Upgrade llvm-ir Crate (Preferred)

The `llvm-ir` crate needs updates for LLVM 15+:
- Fork and patch llvm-ir
- Or wait for upstream support
- Or use alternative parser (llvm-sys bindings)

### Option 3: Manual Pattern Extraction

Since we documented the patterns from manual analysis:
- State machine structure is known
- Detection patterns are validated
- Can implement text-based pattern matching as fallback

## Current Status

**Detection Implementation**: ✓ Complete and correct
