#!/bin/bash
# Development launcher that preserves CWD

# Store the current directory
export LALE_ROOT="$(pwd)"
echo "Setting LALE_ROOT=$LALE_ROOT"

# Change to laleprism directory and run
cd "$(dirname "$0")"
cargo tauri dev
