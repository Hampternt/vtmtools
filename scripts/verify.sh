#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

echo "[1/3] npm run check (svelte-check + tsc)"
npm run check

echo "[2/3] cargo test (Rust unit tests + compile)"
cargo test --manifest-path src-tauri/Cargo.toml

echo "[3/3] npm run build (Vite frontend build)"
npm run build

echo
echo "verify: all checks passed"
