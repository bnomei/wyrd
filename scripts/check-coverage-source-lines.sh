#!/usr/bin/env bash
# Enforce that every executable source line under the supplied directory ran.
#
# cargo-llvm-cov's aggregate line percentage can report false misses for
# partially-covered macro expansions (taiki-e/cargo-llvm-cov#404). Its
# `--show-missing-lines` report uses the JSON source-line view, which is the
# relevant physical-line contract for this gate.
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "usage: $0 <source-directory>" >&2
  exit 2
fi

source_root="$(cd "$1" && pwd)/"
uncovered="$(cargo llvm-cov report --show-missing-lines | awk -v root="$source_root" '
  /^Uncovered Lines:$/ { in_uncovered = 1; next }
  in_uncovered && index($0, root) == 1 { print }
')"

if [[ -n "$uncovered" ]]; then
  echo "uncovered source lines:" >&2
  echo "$uncovered" >&2
  exit 1
fi

echo "source line coverage: 100% ($source_root)"
