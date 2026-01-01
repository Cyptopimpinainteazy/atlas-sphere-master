#!/usr/bin/env bash
set -euo pipefail

# X3 Benchmark Runner Script
# Run from the repository root

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

cd "$REPO_ROOT"

echo "Building x3-bench..."
cargo build --release -p x3-bench

BIN="target/release/x3-bench"
if [ ! -f "$BIN" ]; then
    echo "Release build not found, trying debug..."
    cargo build -p x3-bench
    BIN="target/debug/x3-bench"
fi

echo ""
echo "Running x3-bench..."
echo ""

$BIN

echo ""
echo "Benchmark run complete."
