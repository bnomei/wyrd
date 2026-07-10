#!/usr/bin/env bash
# Dual numeric path check for Wyrd (signal-f32 + signal-i32).
# wyrd-bevy is f32-only and is checked separately.
set -euo pipefail
cd "$(dirname "$0")/.."

echo "==> signal-f32 (default workspace tests)"
cargo test -p wyrd-core -p wyrd-graph -p wyrd-runtime

echo "==> signal-i32"
cargo test -p wyrd-core -p wyrd-graph -p wyrd-runtime \
  --no-default-features \
  --features "std,signal-i32"

echo "==> signal-i32 + serde-ron"
cargo test -p wyrd-graph \
  --no-default-features \
  --features "std,signal-i32,serde-ron"

echo "==> signal-f32 + serde-ron"
cargo test -p wyrd-graph \
  --no-default-features \
  --features "std,signal-f32,serde-ron"

echo "==> signal-i32 + serde-json"
cargo test -p wyrd-graph \
  --no-default-features \
  --features "std,signal-i32,serde-json"

echo "==> signal-f32 + serde-json"
cargo test -p wyrd-graph \
  --no-default-features \
  --features "std,signal-f32,serde-json"

echo "==> signal-f32 + serde-ron + serde-json (cross-codec)"
cargo test -p wyrd-graph \
  --no-default-features \
  --features "std,signal-f32,serde-ron,serde-json"

echo "==> signal-i32 + serde-ron + serde-json (cross-codec)"
cargo test -p wyrd-graph \
  --no-default-features \
  --features "std,signal-i32,serde-ron,serde-json"

echo "==> no_std core signal-i32"
cargo check -p wyrd-core \
  --no-default-features \
  --features "alloc,signal-i32"

echo "==> bevy f32-only"
cargo test -p wyrd-bevy

echo "dual-check OK"
