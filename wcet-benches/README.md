# WCET Benchmark Suite

Validation benchmarks for lale WCET analysis.

## Benchmark Suites

### TACLeBench
- **Source**: https://github.com/tacle/tacle-bench
- **Description**: Modern WCET benchmark suite with diverse algorithms
- **Programs**: ~50 benchmarks covering various domains
- **License**: Various open-source licenses

### Mälardalen WCET Benchmarks
- **Source**: http://www.mrtc.mdh.se/projects/wcet/benchmarks.html
- **Description**: Classic WCET benchmark collection
- **Programs**: ~35 small to medium programs
- **License**: Public domain / research use

### MRTC Benchmarks
- **Source**: http://www.mrtc.mdh.se/projects/wcet/
- **Description**: Real-time system kernels
- **Programs**: Control flow intensive programs
- **License**: Research use

## Setup

1. Clone benchmark repositories:
```bash
cd wcet-benches

# TACLeBench
git clone https://github.com/tacle/tacle-bench.git benchmarks/tacle-bench

# Mälardalen (manual download from website)
# Download from http://www.mrtc.mdh.se/projects/wcet/benchmarks.html
# Extract to benchmarks/malardalen/
```

2. Compile benchmarks to LLVM IR:
```bash
./scripts/compile_benchmarks.sh
```

3. Run analysis:
```bash
cargo run --release -- run
```

## Directory Structure

```
wcet-benches/
├── Cargo.toml              # Benchmark project config
├── README.md               # This file
├── benchmarks/             # Benchmark source code
│   ├── tacle-bench/        # TACLeBench suite
│   ├── malardalen/         # Mälardalen suite
│   └── mrtc/               # MRTC suite
├── llvm-ir/                # Compiled LLVM IR files
├── results/                # Analysis results
├── scripts/                # Helper scripts
│   ├── compile_benchmarks.sh
│   ├── run_analysis.sh
│   └── compare_results.py
└── src/
    ├── lib.rs              # Benchmark runner
    ├── validation.rs       # Result validation
    └── comparison.rs       # Compare with reference
```

## Usage

### Run Single Benchmark
```bash
cargo run --release -- run --benchmark adpcm
```

### Run Full Suite
```bash
cargo run --release -- run --suite tacle-bench
```

### List Available Benchmarks
```bash
cargo run --release -- list
```

### Compare with Reference
```bash
./scripts/compare_results.py results/lale.json results/reference.json
```

## Validation Metrics

1. **Soundness**: WCET ≥ measured execution time
2. **Precision**: WCET / actual execution time (lower is better)
3. **Analysis time**: Time to compute WCET
4. **Comparison**: Difference from reference tools (LLVMTA, aiT)

## Expected Results

Typical precision ratios:
- Simple programs: 1.0 - 1.2x
- Medium complexity: 1.2 - 1.5x
- Complex programs: 1.5 - 3.0x

## Notes

- Benchmarks must be compiled with loop bounds annotations
- Platform configuration affects results significantly
- Cache analysis has major impact on precision
- Some benchmarks may require manual annotations
