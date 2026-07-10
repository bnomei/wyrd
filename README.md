# Wyrd

A small Rust library for composing game behavior from typed signal graphs, using a Norse-inspired weaving metaphor.

**Engine-neutral** · **dual numeric paths** (`f32` / `i32` Q16) · **`no_std` + `alloc`**

## Crates

| Crate | Role |
| --- | --- |
| [`wyrd-core`](crates/wyrd-core) | `Signal`, dense ids, closed port tables, `KnotKind` |
| [`wyrd-graph`](crates/wyrd-graph) | Author `Weave`, builder, validate |
| [`wyrd-runtime`](crates/wyrd-runtime) | Bind → sample → loom → outbox |
| [`wyrd-bevy`](crates/wyrd-bevy) | Thin Bevy 0.18 adapter (`WyrdPlugin`, dense bindings) |

## Quick taste

```rust
use wyrd_core::{HostTime, KnotKind, ONE};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

let (b, _) = Weave::builder("hello")
    .knot("c", KnotKind::constant(ONE))?;
let (b, _) = b.knot("n", KnotKind::not())?;
let (b, _) = b.knot("o", KnotKind::signal_out("debug.inverted"))?;
let weave = b
    .wire_named("c", "out", "n", "in")
    .wire_named("n", "out", "o", "in")
    .build()?;

let mut rt = Runtime::bind(&weave, BindOpts::default())?;
rt.begin_frame(HostTime { tick: 0 });
rt.loom(&weave)?;
// outbox: path "debug.inverted" = falsey (Not of ONE)
```

Host tick: resolve `sense_id("plate")` once, then `set_sense(KnotId, Signal)` each frame — **no string lookup on the hot path**.

## Host tick (sample → loom → apply)

Game engines **own** world I/O. Wyrd never knows doors, cameras, or Entities.

```text
1. host.sample_into(PortWriter)   // dense set_sense(KnotId, Signal)
2. begin_frame + loom             // settle DAG once
3. host.apply(outbox)             // SetLevel / Emit via HostPathId / CmdId
```

Use `wyrd_runtime::{Host, tick_once, NullHost, ScriptedHost, HostCommand}` for
headless/scripted loops, or free-form systems in Bevy (`WyrdSet::{Sample, Loom, Apply}`).

**Door is a host effect** — a Bevy `Door` component (or your own type), not a Knot.
Bevy **Messages** (`WyrdSignalConfirm`) are post-apply confirmations for VFX/UI only;
they are **never** Weave Threads.

```bash
cargo run -p wyrd-bevy --example and_door   # Door component + confirmation Message
```

## First five Weaves

CI recipes in `crates/wyrd-runtime/tests/patterns_cookbook.rs`:

| # | Recipe | Knots |
| --- | --- | --- |
| 1 | Monostable (Pattern) | RisingFromZero → PulseHold |
| 2 | Two-plate door | SignalIn ×2 → And → SignalOut |
| 3 | Flag toggle | rising toggle + reset |
| 4 | Counter threshold | edge → Counter → Compare |
| 5 | Delayed pulse | Delay Rune |

```bash
cargo test -p wyrd-runtime --test patterns_cookbook
```

## Features

| Feature | Meaning |
| --- | --- |
| `std` (default) | Desktop / tests via `no-std-compat` |
| `signal-f32` (default) | Float wire path |
| `signal-i32` | Integer Q16 path (Playdate-class) |
| `serde` (graph/core) | Derive Serialize/Deserialize on author types |
| `serde-ron` (graph) | `from_ron` / `to_ron` + validate on load |
| `serde-json` (graph) | `from_json` / `to_json` + validate on load (same schema as RON) |

Enable exactly one of `signal-f32` / `signal-i32`.

```bash
# Dual numeric paths (f32 + i32) + Bevy f32-only
./scripts/dual-check.sh

# Line coverage (HTML under target/coverage/; see scripts/coverage-gaps.md)
./scripts/coverage.sh
./scripts/coverage.sh --i32

cargo test --workspace
cargo test -p wyrd-graph --features serde-ron
cargo test -p wyrd-graph --features serde-json
cargo bench -p wyrd-runtime   # settle_chain | settle_catalog | settle_stateful | bind
cargo test -p wyrd-bevy
cargo run -p wyrd-bevy --example and_door
```

| Path | Crates | Notes |
| --- | --- | --- |
| `signal-f32` | core, graph, runtime, **bevy** | Default; Bevy adapter is f32-only |
| `signal-i32` | core, graph, runtime | Playdate-class; **not** via `wyrd-bevy` |

CI: `.github/workflows/ci.yml` runs both signal matrices + Bevy + `no_std` i32 check.

## Docs (local / gitignored)

Design notes live under `docs/` (see `.gitignore`). Start with:

- `docs/ROADMAP.md` — full checklist (done / open / later)  
- `docs/vision.md` — product vision  
- `docs/research/decisions.md` — locked decisions  
- `docs/api-preview/11_revised_surface.rs` — v2 API pencil  
- `docs/primitives/port-schema.md` — closed port tables  

## License

MIT
