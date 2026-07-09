#!/usr/bin/env bash
# Line coverage via cargo-llvm-cov (requires llvm-tools component).
# Usage:
#   ./scripts/coverage.sh              # f32 core + fail-under 100
#   ./scripts/coverage.sh --i32        # also i32 suite
#   ./scripts/coverage.sh --bevy       # also wyrd-bevy
#   ./scripts/coverage.sh --all        # f32 + i32 + bevy + serde-ron
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
FAIL_UNDER="${FAIL_UNDER:-100}"

run_one() {
  local label=$1
  shift
  local dir="$OUT/$label"
  mkdir -p "$dir"
  echo "==> coverage ($label) $*"
  cargo llvm-cov clean --workspace 2>/dev/null || true
  cargo llvm-cov \
    --html --output-dir "$dir/html" \
    --fail-under-lines "$FAIL_UNDER" \
    "$@"
  cargo llvm-cov report --summary-only 2>/dev/null | tee "$dir/summary.txt" || true
  cargo llvm-cov --summary-only "$@" | tee "$dir/summary.txt"
  echo "HTML: $dir/html/html/index.html"
}

ARGS=("$@")
DO_I32=0
DO_BEVY=0
DO_SERDE=0
DO_OPEN=0
for a in "${ARGS[@]+"${ARGS[@]}"}"; do
  case "$a" in
    --i32) DO_I32=1 ;;
    --bevy) DO_BEVY=1 ;;
    --serde) DO_SERDE=1 ;;
    --all) DO_I32=1; DO_BEVY=1; DO_SERDE=1 ;;
    --open) DO_OPEN=1 ;;
  esac
done

# Default: f32 core (exclude bevy)
run_one f32 --workspace --exclude wyrd-bevy

if [[ "$DO_I32" == 1 ]]; then
  run_one i32 -p wyrd-core -p wyrd-graph -p wyrd-runtime \
    --no-default-features --features "std,signal-i32"
fi

if [[ "$DO_SERDE" == 1 ]]; then
  run_one serde-ron -p wyrd-graph \
    --no-default-features --features "std,signal-f32,serde-ron"
fi

if [[ "$DO_BEVY" == 1 ]]; then
  run_one bevy -p wyrd-bevy
fi

if [[ "$DO_OPEN" == 1 ]]; then
  if [[ -d "$OUT/f32/html/html" ]]; then
    open "$OUT/f32/html/html/index.html" 2>/dev/null || true
  fi
fi

echo "Done. See $OUT/*/summary.txt and HTML reports (fail-under-lines=$FAIL_UNDER)."
