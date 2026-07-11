# Wyrd

Wyrd is a Rust library for composing game behavior as validated signal graphs. You author a
`Weave`, bind it once into dense runtime state, sample host inputs, settle the graph, and apply
its outputs back to your game.

**Engine-neutral** · **`no_std` + `alloc`** · **`f32` or Q16 `i32` signals** · **Bevy 0.18 adapter**

Wyrd 0.2 is pre-1.0 and contains a breaking API redesign. If you used 0.1, start with the
[0.2 migration guide](MIGRATION-0.2.md).

## When Wyrd fits

Use Wyrd when you want game logic that is:

- authored as a closed catalog of typed knots and ports;
- validated before execution for cycles, fan-in, required inputs, numeric compatibility, and
  budgets;
- independent from engine entities, components, and world access;
- executed through dense handles without string lookup on the tick path; and
- portable between desktop `f32` builds and Q16 `i32` hosts.

Wyrd does not own doors, cameras, entities, or other game state. Your host samples that state
into the runtime and applies the resulting signal levels and commands.

## Quickstart

### Prerequisites

- Rust 1.75 or later
- A checkout of this repository

The commands in this README use the repository workspace; they do not assume that the 0.2 crates
have been published to crates.io.

Clone the repository:

```bash
git clone https://github.com/bnomei/wyrd.git
cd wyrd
```

Verify the workspace with a small end-to-end test:

```bash
cargo test -p wyrd-runtime --test hello_and_door
```

Expected result excerpt:

```text
test result: ok. 2 passed; 0 failed; ...
```

### Core API example

The test above exercises the same author → bind → sample → loom → outbox flow shown here. In an
application, the core API looks like this:

```rust
use std::error::Error;

use wyrd_core::{is_truthy, HostTime, KnotKind, ONE};
use wyrd_graph::weave;
use wyrd_runtime::{BindOpts, Runtime};

fn main() -> Result<(), Box<dyn Error>> {
    let weave = weave! {
        id: "hello";
        knots {
            source = KnotKind::signal_in();
            invert = KnotKind::not();
            sink = KnotKind::signal_out("debug.inverted");
        }
        threads {
            source.out -> invert.in;
            invert.out -> sink.in;
        }
    }?;

    let mut runtime = Runtime::bind(weave, BindOpts::default())?;
    let source = runtime.sense_id("source").expect("validated SignalIn");
    let output = runtime
        .path_id("debug.inverted")
        .expect("validated SignalOut");

    runtime.begin_frame(HostTime { tick: 0 });
    runtime.port_writer().set_sense(source, ONE)?;
    runtime.loom();

    let outbox = runtime.outbox();
    let sample = outbox
        .signals()
        .iter()
        .find(|sample| sample.path == output)
        .expect("SignalOut sample");
    assert!(!is_truthy(sample.value));

    Ok(())
}
```

`Runtime::bind` consumes the validated `Weave`. The resulting `Runtime` is the sole executable
artifact, and `loom()` is infallible after a successful bind.

## How a host tick works

```text
Host world
   │ sample_into(PortWriter) using SenseId
   ▼
Runtime::begin_frame → Runtime::loom
   │
   ├─ SignalOutSample { path: HostPathId, value }
   └─ Emit { cmd: CmdId, payload }
   ▼
Host applies effects to its own world
```

Resolve `SenseId`, `HostPathId`, and `CmdId` once during setup. These handles are owned by the
runtime that created them; using one with another runtime returns `HandleError::ForeignRuntime`.

You can implement [`Host`](crates/wyrd-runtime/src/host.rs) and call `tick_once`, use
`NullHost` or `ScriptedHost` for headless execution, or schedule sample and apply systems around
the Bevy adapter.

## Choose a crate

| Crate | Use it for |
| --- | --- |
| [`wyrd-core`](crates/wyrd-core) | `Signal`, numeric helpers, `KnotKind`, and the closed port catalog |
| [`wyrd-graph`](crates/wyrd-graph) | Immutable `Weave`/`Pattern` values, definitions, typed builder, validation, codecs, and `weave!` |
| [`wyrd-runtime`](crates/wyrd-runtime) | Binding, host handles, sample/loom/outbox execution, and headless hosts |
| [`wyrd-bevy`](crates/wyrd-bevy) | Bevy 0.18 scheduling and host-integration helpers for the `f32` path |

The dependency direction stays one-way:

```text
wyrd-core → wyrd-graph → wyrd-runtime → wyrd-bevy
```

## Author graphs

Use `weave!` for static graphs. It supports explicit author IDs, numeric-path selection, pattern
instances, and knot-to-pattern connections while expanding through the same checked builder API.

Use `WeaveBuilder` when code generates topology dynamically. Its owner-aware `KnotHandle`,
`InputPort`, and `OutputPort` values reject cross-builder and reversed connections before final
validation.

Use `WeaveDef` and `PatternDef` for editable or serialized data. Converting a definition into an
immutable `Weave` or `Pattern` performs structural validation. The optional RON and JSON codecs
also validate while loading.

See the [`wyrd-graph` guide](crates/wyrd-graph/README.md) and the executable
[`weave!` tests](crates/wyrd-graph/tests/weave_macro.rs) for complete authoring examples.

## Integrate with Bevy

`wyrd-bevy` configures three ordered system sets:

```text
WyrdSet::Sample → WyrdSet::Loom → WyrdSet::Apply
```

The plugin owns only the loom step. Your systems read components during `Sample` and mutate
components during `Apply`. Bevy messages such as `WyrdSignalConfirm` confirm applied host effects;
they are not graph threads.

Run the headless two-plate door example:

```bash
cargo run -p wyrd-bevy --example and_door
```

The example samples two plate states, settles an `And` knot, applies `SignalOut("door.open")` to a
host-owned `Door` component, and emits confirmations when the component changes. See the
[`wyrd-bevy` guide](crates/wyrd-bevy/README.md) for the exact ownership boundary.

## Learn through executable recipes

[`wyrd_runtime::cookbook`](crates/wyrd-runtime/src/cookbook/) provides 20 recipes used by both
rustdoc and integration tests.

| Tier | Focus | Recipes |
| --- | --- | --- |
| **A** | Foundations | Not, two-input And, bind/sample/loom, `tick_once`, validation failure |
| **B** | Reusable Weaves | Monostable Pattern, two-plate door, Flag, Counter threshold, Delay |
| **C** | Game-logic patterns | Latches, timers, cooldowns, Threshold, Map, Digitize, OnStart, Emit, Or |

Run the complete ladder:

```bash
cargo test -p wyrd-runtime --test tutorial_ladder
cargo test -p wyrd-runtime --doc
```

## Features and numeric paths

Enable exactly one of `signal-f32` and `signal-i32`.

| Feature | Crates | Behavior |
| --- | --- | --- |
| `std` (default) | core, graph, runtime | Desktop/test support through `no-std-compat` |
| `alloc` | core, graph, runtime | Heap-backed graph/runtime storage without `std` |
| `signal-f32` (default) | core, graph, runtime | Floating-point signal path; uses `libm` for `no_std` square root |
| `signal-i32` | core, graph, runtime | Q16 integer signal path for constrained hosts |
| `serde` | core, graph | Serde derives for author definitions |
| `serde-ron` | graph | RON load/save with validation on load |
| `serde-json` | graph | JSON load/save with validation on load |
| `bevy_log` | bevy | Forwards Bevy's `bevy_log` feature |

`wyrd-bevy` always uses `signal-f32`. Use `wyrd-runtime` directly for `signal-i32` hosts.

Verify both numeric paths, codecs, runtime `no_std` builds, and Bevy:

```bash
./scripts/dual-check.sh
```

## Playdate / constrained hosts

Use the runtime directly rather than `wyrd-bevy`, selecting the integer signal
path and the allocator supplied by the host application:

```toml
[dependencies]
wyrd-runtime = { version = "0.2.0", default-features = false, features = ["alloc", "signal-i32"] }
```

Bind a Weave when loading a room or scene, resolve its dense sense/path handles
once, then call `begin_frame` → write senses → `loom` once per host tick. Keep
buttons and counts as integers; quantize continuous host input at the boundary.

Use the Playdate Rust toolchain's device build (for example,
[`cargo-playdate`](https://github.com/boozook/playdate)'s `cargo playdate run --device`)
to validate a consuming game. The simulator is useful for iteration, but profile
representative Map/Sqrt-heavy Weaves on physical hardware before making a frame-time
claim.

## Validate changes

Run the same primary checks used during development:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
./scripts/dual-check.sh
```

For local line-coverage reports:

```bash
./scripts/coverage.sh
./scripts/coverage.sh --i32
```

The HTML report is written under `target/coverage/`. CI enforces the numeric/codec matrix, Bevy
builds, runtime `no_std` checks, warnings as errors, and line-coverage gates.

## Reference and next steps

- [Graph authoring and `weave!`](crates/wyrd-graph/README.md)
- [Runtime host loop and cookbook](crates/wyrd-runtime/README.md)
- [Bevy integration boundary](crates/wyrd-bevy/README.md)
- [Closed knot port tables](crates/wyrd-core/src/ports.rs)
- [Runtime error contracts](crates/wyrd-runtime/src/error.rs)
- [0.2 migration guide](MIGRATION-0.2.md)
- [Changelog](CHANGELOG.md)

## License

MIT
