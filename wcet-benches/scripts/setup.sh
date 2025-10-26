#!/bin/bash
# Setup WCET benchmark suite - download and organize TACLeBench

set -e

BENCH_DIR="data/sources"
mkdir -p "$BENCH_DIR"

echo "=== WCET Benchmark Suite Setup ==="
echo "Target: ARM Cortex-M7"
echo ""

# TACLeBench
echo "Downloading TACLeBench v1.9..."
if [ ! -d "$BENCH_DIR/taclebench-repo" ]; then
    git clone --depth 1 --branch V1.9 \
      https://github.com/tacle/tacle-bench.git \
      "$BENCH_DIR/taclebench-repo"
else
    echo "  TACLeBench already downloaded, skipping..."
fi

# Organize TACLeBench by category
echo ""
echo "Organizing TACLeBench benchmarks..."

mkdir -p "$BENCH_DIR/taclebench/kernel"
mkdir -p "$BENCH_DIR/taclebench/sequential"
mkdir -p "$BENCH_DIR/taclebench/parallel"
mkdir -p "$BENCH_DIR/taclebench/test"

# Copy kernel benchmarks
echo "  Copying kernel benchmarks..."
for bench_dir in "$BENCH_DIR/taclebench-repo/bench/kernel"/*; do
    if [ -d "$bench_dir" ]; then
        bench_name=$(basename "$bench_dir")
        mkdir -p "$BENCH_DIR/taclebench/kernel/$bench_name"
        cp -r "$bench_dir"/* "$BENCH_DIR/taclebench/kernel/$bench_name/"
        echo "    ✓ $bench_name"
    fi
done

# Copy sequential benchmarks
echo "  Copying sequential benchmarks..."
for bench_dir in "$BENCH_DIR/taclebench-repo/bench/sequential"/*; do
    if [ -d "$bench_dir" ]; then
        bench_name=$(basename "$bench_dir")
        mkdir -p "$BENCH_DIR/taclebench/sequential/$bench_name"
        cp -r "$bench_dir"/* "$BENCH_DIR/taclebench/sequential/$bench_name/"
        echo "    ✓ $bench_name"
    fi
done

# Copy parallel benchmarks
echo "  Copying parallel benchmarks..."
for bench_dir in "$BENCH_DIR/taclebench-repo/bench/parallel"/*; do
    if [ -d "$bench_dir" ]; then
        bench_name=$(basename "$bench_dir")
        mkdir -p "$BENCH_DIR/taclebench/parallel/$bench_name"
        cp -r "$bench_dir"/* "$BENCH_DIR/taclebench/parallel/$bench_name/"
        echo "    ✓ $bench_name"
    fi
done

# Copy test benchmarks
echo "  Copying test benchmarks..."
for bench_dir in "$BENCH_DIR/taclebench-repo/bench/test"/*; do
    if [ -d "$bench_dir" ]; then
        bench_name=$(basename "$bench_dir")
        mkdir -p "$BENCH_DIR/taclebench/test/$bench_name"
        cp -r "$bench_dir"/* "$BENCH_DIR/taclebench/test/$bench_name/"
        echo "    ✓ $bench_name"
    fi
done

# Create validation programs
echo ""
echo "Creating validation programs..."
mkdir -p "$BENCH_DIR/validation"

cat > "$BENCH_DIR/validation/straight_line.c" << 'EOF'
// Simple straight-line program for validation
int main(void) {
    int a = 1;
    int b = 2;
    int c = a + b;
    int d = c * 2;
    return d;
}
EOF

cat > "$BENCH_DIR/validation/simple_loop.c" << 'EOF'
// Simple loop for validation
int main(void) {
    int sum = 0;
    for (int i = 0; i < 10; i++) {
        sum += i;
    }
    return sum;
}
EOF

cat > "$BENCH_DIR/validation/nested_loops.c" << 'EOF'
// Nested loops for validation
int main(void) {
    int sum = 0;
    for (int i = 0; i < 5; i++) {
        for (int j = 0; j < 4; j++) {
            sum += i * j;
        }
    }
    return sum;
}
EOF

echo "  ✓ straight_line.c"
echo "  ✓ simple_loop.c"
echo "  ✓ nested_loops.c"

# Count benchmarks
kernel_count=$(find "$BENCH_DIR/taclebench/kernel" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)
seq_count=$(find "$BENCH_DIR/taclebench/sequential" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)
parallel_count=$(find "$BENCH_DIR/taclebench/parallel" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)
test_count=$(find "$BENCH_DIR/taclebench/test" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | wc -l)

echo ""
echo "=== Setup Complete ==="
echo "TACLeBench benchmarks organized:"
echo "  Kernel:     $kernel_count"
echo "  Sequential: $seq_count"
echo "  Parallel:   $parallel_count"
echo "  Test:       $test_count"
echo "  Total:      $((kernel_count + seq_count + parallel_count + test_count))"
echo ""
echo "Validation programs: 3"
echo ""
echo "Next steps:"
echo "  1. Run: ./scripts/compile_benchmarks.sh"
echo "  2. Run: cargo run --release -- run"
