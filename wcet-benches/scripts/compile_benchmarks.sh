#!/bin/bash
# Compile benchmark suites to LLVM IR

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BENCHMARKS_DIR="$PROJECT_ROOT/benchmarks"
LLVM_IR_DIR="$PROJECT_ROOT/llvm-ir"

# LLVM/Clang settings
CLANG=${CLANG:-clang}
OPT=${OPT:-opt}
LLVM_DIS=${LLVM_DIS:-llvm-dis}

# Compilation flags
CFLAGS="-O2 -emit-llvm -S"
OPTFLAGS="-mem2reg -simplifycfg"

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
        # Optimize
        $OPT $OPTFLAGS "$output_file" -S -o "$output_file.opt" 2>/dev/null || true
        if [ -f "$output_file.opt" ]; then
            mv "$output_file.opt" "$output_file"
        fi
        echo "  ✓ $output_file"
    else
        echo "  ✗ Failed to compile $src_file"
    fi
}

# Compile TACLeBench
if [ -d "$BENCHMARKS_DIR/tacle-bench" ]; then
    echo ""
    echo "=== TACLeBench ==="
    
    # Find all C files in TACLeBench
    find "$BENCHMARKS_DIR/tacle-bench" -name "*.c" -type f | while read -r src_file; do
        # Skip test files
        if [[ "$src_file" != *"test"* ]]; then
            compile_benchmark "$src_file" "tacle-bench"
        fi
    done
else
    echo "TACLeBench not found at $BENCHMARKS_DIR/tacle-bench"
    echo "Clone with: git clone https://github.com/tacle/tacle-bench.git $BENCHMARKS_DIR/tacle-bench"
fi

# Compile Mälardalen
if [ -d "$BENCHMARKS_DIR/malardalen" ]; then
    echo ""
    echo "=== Mälardalen ==="
    
    find "$BENCHMARKS_DIR/malardalen" -name "*.c" -type f | while read -r src_file; do
        compile_benchmark "$src_file" "malardalen"
    done
else
    echo "Mälardalen benchmarks not found at $BENCHMARKS_DIR/malardalen"
    echo "Download from: http://www.mrtc.mdh.se/projects/wcet/benchmarks.html"
fi

# Compile MRTC
if [ -d "$BENCHMARKS_DIR/mrtc" ]; then
    echo ""
    echo "=== MRTC ==="
    
    find "$BENCHMARKS_DIR/mrtc" -name "*.c" -type f | while read -r src_file; do
        compile_benchmark "$src_file" "mrtc"
    done
else
    echo "MRTC benchmarks not found at $BENCHMARKS_DIR/mrtc"
fi

echo ""
echo "=== Compilation Complete ==="
echo "LLVM IR files: $LLVM_IR_DIR"
echo ""
echo "Run benchmarks with:"
echo "  cd $PROJECT_ROOT"
echo "  cargo run --release -- run"
