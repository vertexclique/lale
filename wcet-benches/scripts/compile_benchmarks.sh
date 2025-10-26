#!/bin/bash
# Compile benchmark suites to LLVM IR

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SOURCES_DIR="$PROJECT_ROOT/data/sources"
LLVM_IR_DIR="$PROJECT_ROOT/data/llvm-ir"

# LLVM/Clang settings
CLANG=${CLANG:-clang}
OPT=${OPT:-opt}
LLVM_DIS=${LLVM_DIS:-llvm-dis}

# Compilation flags for ARM Cortex-M7
CFLAGS="-O0 -S -emit-llvm --target=armv7em-none-eabi -mcpu=cortex-m7 -mfloat-abi=hard -mfpu=fpv5-d16"

echo "=== WCET Benchmark Compilation ==="
echo "Clang: $CLANG"
echo "Output: $LLVM_IR_DIR"
echo ""

# Create output directory
mkdir -p "$LLVM_IR_DIR"

# Function to compile a C file to LLVM IR
compile_benchmark() {
    local src_file="$1"
    local suite="$2"
    local benchmark_name=$(basename "$src_file" .c)
    local output_dir="$LLVM_IR_DIR/$suite"
    local output_file="$output_dir/${benchmark_name}.ll"
    
    mkdir -p "$output_dir"
    
    echo "Compiling: $suite/$benchmark_name"
    
    # Compile to LLVM IR
    if $CLANG $CFLAGS "$src_file" -o "$output_file" 2>/dev/null; then
        echo "  ✓ $output_file"
    else
        echo "  ✗ Failed to compile $src_file"
    fi
}

# Compile validation programs
if [ -d "$SOURCES_DIR/validation" ]; then
    echo ""
    echo "=== Validation Programs ==="
    
    for src_file in "$SOURCES_DIR/validation"/*.c; do
        if [ -f "$src_file" ]; then
            compile_benchmark "$src_file" "validation"
        fi
    done
fi

# Compile TACLeBench
if [ -d "$SOURCES_DIR/taclebench" ]; then
    echo ""
    echo "=== TACLeBench ==="
    
    # Find all C files in TACLeBench
    find "$SOURCES_DIR/taclebench" -name "*.c" -type f | while read -r src_file; do
        compile_benchmark "$src_file" "taclebench"
    done
else
    echo "TACLeBench not found at $SOURCES_DIR/taclebench"
    echo "Run: ./scripts/setup.sh"
fi


echo ""
echo "=== Compilation Complete ==="
echo "LLVM IR files: $LLVM_IR_DIR"
echo ""
echo "Run benchmarks with:"
echo "  cd $PROJECT_ROOT"
echo "  cargo run --release -- run"
