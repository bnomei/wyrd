# Coverage gaps (how to read + what to test next)

Generated with `./scripts/coverage.sh` (`cargo-llvm-cov`).  
**Bevy excluded** from the core suite (heavy; covered by `cargo test -p wyrd-bevy`).

## Snapshot (default `signal-f32`, core/graph/runtime)

| File | Lines | Cover | Priority |
| --- | ---: | ---: | --- |
| `wyrd-core/src/error.rs` | 0/17 | **0%** | Low — Display only; add once if hosts match on messages |
| `wyrd-core/src/signal.rs` | ~58% | **Low** | **High** — i32/`from_level`/mul/div only half-hit under f32 |
| `wyrd-runtime/src/loom.rs` | ~73% | **Med** | **High** — Or, Map, Abs, Neg, Flag SetWins, Calc variants, Emit enable |
| `wyrd-core/src/kind.rs` | ~75% | Med | Helpers unused (`or2`, etc.) |
| `wyrd-core/src/ports.rs` | ~88% | OK | Arity 3–4 tables, sparse kinds |
| `wyrd-graph/*` | ~90–100% | Good | Thin gaps (builder `wire` path, validate edges) |
| `wyrd-runtime/src/bind.rs` | ~96% | Good | Rare error arms |
| **TOTAL** | ~85% lines | | |

Open HTML: `target/coverage/html/html/index.html` (or `target/coverage/f32/html/...` via script).

## Where to add tests (recommended order)

### 1. Loom branches (`loom.rs`) — biggest bang

| Gap | Suggested test |
| --- | --- |
| **Or** knot | Two SignalIn → Or → SignalOut |
| **Map** | Constant/Map range remap + clamp endpoints |
| **Abs / Neg** | Negative constant → Abs/Neg → out |
| **Calc** Add/Sub/Mul | Not only Div0 |
| **Flag SetWins** | set + reset same tick priority |
| **Counter dec** | Rising dec from N |
| **OnStart** | First loom ONE, later ZERO |
| **Emit enable** (if wired) | Optional port gating once implemented |

### 2. Signal dual path (`signal.rs`)

| Gap | Suggested test |
| --- | --- |
| `from_level` on **i32** | Round-trip ~0.5 → Q16 |
| `mul` / `div` Q-mul | Via Calc knots under `--features signal-i32` |
| `sat_add` / `sat_sub` | Extreme i32 values if exposed |

Run: `./scripts/coverage.sh --i32` and compare summaries.

### 3. Graph / validate

| Gap | Suggested test |
| --- | --- |
| `builder.wire` (dense PortSlot) | Not only `wire_named` |
| Cycle detect | A→B→A |
| Budget overflow | knots > hard max |
| Unconnected required | Not with no input |

### 4. Bevy

Not in llvm-cov workspace exclude. Use:

```bash
cargo test -p wyrd-bevy
# optional later: cargo llvm-cov -p wyrd-bevy --html
```

## Commands

```bash
# Install once
cargo install cargo-llvm-cov
rustup component add llvm-tools

# Report
./scripts/coverage.sh
./scripts/coverage.sh --i32
./scripts/coverage.sh --open   # macOS open HTML

# CI-style fail under threshold (optional)
cargo llvm-cov --workspace --exclude wyrd-bevy --fail-under-lines 80
```

## Policy

- Treat **line %** as a guide, not a game. Prefer **behavior tests** for knot semantics over chasing 100%.
- Always dual-check critical math under **both** `signal-f32` and `signal-i32`.
