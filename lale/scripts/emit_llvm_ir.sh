#!/usr/bin/env bash
# Note: Not using 'set -e' to allow continuation on individual target failures
set -uo pipefail

# Script to emit LLVM IR for embedded bare-metal targets
# Organizes output by target triple in llvmir/ directory
# Continues processing all targets even if some fail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Allow specifying project directory as first argument, otherwise use current directory
if [ $# -gt 0 ]; then
    PROJECT_ROOT="$(cd "$1" && pwd)"
else
    PROJECT_ROOT="$(pwd)"
fi

OUTPUT_DIR="$PROJECT_ROOT/llvmir"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Common embedded bare-metal targets
EMBEDDED_TARGETS=(
    # ARM Cortex-M (Thumb)
    "thumbv6m-none-eabi"        # Cortex-M0, M0+, M1
    "thumbv7m-none-eabi"        # Cortex-M3
    "thumbv7em-none-eabi"       # Cortex-M4, M7 (no FPU)
    "thumbv7em-none-eabihf"     # Cortex-M4F, M7F (with FPU)
    "thumbv8m.base-none-eabi"   # Cortex-M23
    "thumbv8m.main-none-eabi"   # Cortex-M33 (no FPU)
    "thumbv8m.main-none-eabihf" # Cortex-M33 (with FPU)
    
    # ARM Cortex-A/R (32-bit)
    "armv7a-none-eabi"          # Cortex-A (bare metal)
    "armv7r-none-eabi"          # Cortex-R (real-time)
    "armv7r-none-eabihf"        # Cortex-R (with FPU)
    
    # ARM 64-bit
    "aarch64-unknown-none"      # ARM64 bare metal
    "aarch64-unknown-none-softfloat"
    
    # RISC-V
    "riscv32i-unknown-none-elf"     # RV32I
    "riscv32imac-unknown-none-elf"  # RV32IMAC
    "riscv32imc-unknown-none-elf"   # RV32IMC
    "riscv64gc-unknown-none-elf"    # RV64GC
    "riscv64imac-unknown-none-elf"  # RV64IMAC
    
    # AVR (Arduino, etc.)
    "avr-unknown-gnu-atmega328"
    
    # MSP430 (Texas Instruments)
    "msp430-none-elf"
    
    # Xtensa (ESP32)
    "xtensa-esp32-none-elf"
    "xtensa-esp32s2-none-elf"
    "xtensa-esp32s3-none-elf"
)

# Check if we're in a Rust project
if [ ! -f "$PROJECT_ROOT/Cargo.toml" ]; then
    log_error "Not in a Rust project directory (Cargo.toml not found)"
    exit 1
fi

log_info "Project root: $PROJECT_ROOT"
log_info "Output directory: $OUTPUT_DIR"

# Create output directory
mkdir -p "$OUTPUT_DIR"

# Check if rustup is installed
if ! command -v rustup &> /dev/null; then
    log_error "rustup not found. Please install rustup first."
    exit 1
fi

# Check if cargo is installed
if ! command -v cargo &> /dev/null; then
    log_error "cargo not found. Please install Rust toolchain."
    exit 1
fi

# Function to check if target is installed
is_target_installed() {
    local target=$1
    rustup target list --installed | grep -q "^${target}$"
}

# Function to install target
install_target() {
    local target=$1
    log_info "Installing target: $target"
    if rustup target add "$target" 2>/dev/null; then
        log_success "Installed $target"
        return 0
    else
        log_warn "Failed to install $target (may not be available)"
        return 1
    fi
}

# Function to emit LLVM IR for a target
emit_llvm_ir() {
    local target=$1
    local target_dir="$OUTPUT_DIR/$target"
    
    log_info "Processing target: $target"
    
    # Create target-specific directory
    mkdir -p "$target_dir"
    
    # Check if target is installed
    if ! is_target_installed "$target"; then
        if ! install_target "$target"; then
            log_warn "Skipping $target (not available)"
            return 1
        fi
    fi
    
    # Check if LLVM IR files already exist
    local target_build_dir="$PROJECT_ROOT/target/$target/release"
    local existing_ll_count=0
    
    if [ -d "$target_build_dir/deps" ]; then
        existing_ll_count=$(find "$target_build_dir/deps" -name "*.ll" -type f 2>/dev/null | wc -l)
    fi
    
    if [ $existing_ll_count -gt 0 ]; then
        log_info "Found $existing_ll_count existing LLVM IR files for $target, skipping build"
    else
        # Build with LLVM IR emission
        log_info "Building and emitting LLVM IR for $target"
        
        # Set RUSTFLAGS to emit LLVM IR
        export RUSTFLAGS="--emit=llvm-ir"
        
        # Create log directory
        mkdir -p "$PROJECT_ROOT/target/$target"
        
        # Try to build (may fail for some targets without std)
        # Keep logs in target directory, not in llvmir
        local build_log="$PROJECT_ROOT/target/$target/build.log"
        if cargo build --target "$target" --release > "$build_log" 2>&1; then
            log_success "Build succeeded for $target"
        else
            log_warn "Build failed for $target (see $build_log)"
            
            # Try with no_std if regular build failed
            log_info "Attempting no_std build for $target"
            local build_no_std_log="$PROJECT_ROOT/target/$target/build_no_std.log"
            if cargo build --target "$target" --release --no-default-features > "$build_no_std_log" 2>&1; then
                log_success "no_std build succeeded for $target"
            else
                log_warn "Both builds failed for $target (see $build_no_std_log)"
                # Don't return yet - check if .ll files exist anyway
            fi
        fi
    fi
    
    # Find and copy LLVM IR files from all locations
    local ir_files_found=0
    local target_build_dir="$PROJECT_ROOT/target/$target/release"
    
    # Search in multiple locations for .ll files
    local search_dirs=(
        "$target_build_dir/deps"
        "$target_build_dir"
        "$target_build_dir/build"
        "$target_build_dir/examples"
    )
    
    for search_dir in "${search_dirs[@]}"; do
        if [ -d "$search_dir" ]; then
            log_info "Searching for LLVM IR files in $search_dir"
            
            # Find all .ll files recursively in this directory
            while IFS= read -r ll_file; do
                if [ -f "$ll_file" ]; then
                    local basename=$(basename "$ll_file")
                    log_info "Copying $basename"
                    cp "$ll_file" "$target_dir/"
                    ((ir_files_found++))
                fi
            done < <(find "$search_dir" -maxdepth 2 -name "*.ll" -type f 2>/dev/null)
        fi
    done
    
    if [ $ir_files_found -eq 0 ]; then
        log_warn "No LLVM IR files found for $target"
        return 1
    else
        log_success "Copied $ir_files_found LLVM IR file(s) for $target"
    fi
    
    # Create a summary file
    cat > "$target_dir/README.md" <<EOF
# LLVM IR for $target

Generated: $(date)
Files: $ir_files_found

## Target Information
- Triple: $target
- Architecture: $(echo $target | cut -d'-' -f1)

## Files
$(ls -1 "$target_dir"/*.ll 2>/dev/null | xargs -n1 basename || echo "No .ll files")

EOF
    
    return 0
}

# Main execution
log_info "Starting LLVM IR emission for embedded targets"
log_info "Total targets to process: ${#EMBEDDED_TARGETS[@]}"

successful_targets=0
failed_targets=0

for target in "${EMBEDDED_TARGETS[@]}"; do
    echo ""
    echo "========================================"
    if emit_llvm_ir "$target"; then
        ((successful_targets++))
    else
        ((failed_targets++))
    fi
    echo "========================================"
done

# Summary
echo ""
log_info "========== SUMMARY =========="
log_success "Successful targets: $successful_targets"
if [ $failed_targets -gt 0 ]; then
    log_warn "Failed targets: $failed_targets"
fi
log_info "Output directory: $OUTPUT_DIR"
log_info "============================="

# Create index file
cat > "$OUTPUT_DIR/INDEX.md" <<EOF
# LLVM IR Collection

Generated: $(date)

## Summary
- Total targets processed: ${#EMBEDDED_TARGETS[@]}
- Successful: $successful_targets
- Failed: $failed_targets

## Available Targets

EOF

for target in "${EMBEDDED_TARGETS[@]}"; do
    if [ -d "$OUTPUT_DIR/$target" ] && [ -n "$(ls -A "$OUTPUT_DIR/$target"/*.ll 2>/dev/null)" ]; then
        file_count=$(ls -1 "$OUTPUT_DIR/$target"/*.ll 2>/dev/null | wc -l)
        echo "- ✓ **$target** ($file_count files)" >> "$OUTPUT_DIR/INDEX.md"
    else
        echo "- ✗ $target (failed)" >> "$OUTPUT_DIR/INDEX.md"
    fi
done

log_success "Index created at $OUTPUT_DIR/INDEX.md"

exit 0
