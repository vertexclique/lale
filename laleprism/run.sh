#!/bin/bash
# LALE Prism launcher script for Linux
# Fixes Wayland/WebKit compatibility issues

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    export WEBKIT_DISABLE_COMPOSITING_MODE=1
    export GDK_BACKEND=x11
fi

# Get script directory
DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Run from target/release if exists, otherwise cargo run
if [ -f "$DIR/target/release/laleprism" ]; then
    exec "$DIR/target/release/laleprism" "$@"
else
    cd "$DIR"
    exec cargo run --release "$@"
fi
