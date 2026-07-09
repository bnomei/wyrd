#!/usr/bin/env bash
# Line coverage via cargo-llvm-cov (requires llvm-tools component).
# Usage:
#   ./scripts/coverage.sh              # HTML + summary for f32 core suite
#   ./scripts/coverage.sh --i32        # also run i32 suite (separate report dir)
#   ./scripts/coverage.sh --open       # open HTML in browser (macOS)
set -euo pipefail
cd "$(dirname "$0")/.."

if ! command -v cargo-llvm-cov >/dev/null 2>&1; then
  echo "install: cargo install cargo-llvm-cov"
  echo "         rustup component add llvm-tools"
  exit 1
fi

OUT="${COVERAGE_OUT:-target/coverage}"
mkdir -p "$OUT"

run_one() {
  local label=$1
  shift
  local dir="$OUT/$label"
  mkdir -p "$dir"
  echo "==> coverage ($label) $*"
  cargo llvm-cov --workspace --exclude wyrd-bevy \
    --html --output-dir "$dir/html" \
    "$@"
  cargo llvm-cov report --summary-only 2>/dev/null | tee "$dir/summary.txt" || true
  # Refresh summary after run (profiles in llvm-cov-target)
  cargo llvm-cov --workspace --exclude wyrd-bevy --summary-only "$@" | tee "$dir/summary.txt"
  echo "HTML: $dir/html/html/index.html"
}

# Default: f32 (crate defaults)
run_one f32

if [[ "${1:-}" == "--i32" || "${2:-}" == "--i32" ]]; then
  cargo llvm-cov clean --workspace 2>/dev/null || true
  run_one i32 --no-default-features --features "std,signal-i32" \
    -p wyrd-core -p wyrd-graph -p wyrd-runtime
fi

if [[ "${1:-}" == "--open" || "${2:-}" == "--open" ]]; then
  if [[ -d "$OUT/f32/html/html" ]]; then
    open "$OUT/f32/html/html/index.html" 2>/dev/null || true
  fi
fi

echo "Done. See $OUT/*/summary.txt and HTML reports."
