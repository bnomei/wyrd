# Wyrd

[![Crates.io Version](https://img.shields.io/crates/v/wyrd-for-games)](https://crates.io/crates/wyrd-for-games)
[![Crates.io Downloads](https://img.shields.io/crates/d/wyrd-for-games)](https://crates.io/crates/wyrd-for-games)
[![CI](https://img.shields.io/github/actions/workflow/status/bnomei/wyrd/ci.yml?branch=main&label=CI)](https://github.com/bnomei/wyrd/actions/workflows/ci.yml)
[![CodSpeed](https://img.shields.io/github/actions/workflow/status/bnomei/wyrd/codspeed.yml?branch=main&label=CodSpeed)](https://github.com/bnomei/wyrd/actions/workflows/codspeed.yml)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Discord](https://flat.badgen.net/badge/discord/bnomei?color=7289da&icon=discord&label)](https://discordapp.com/users/bnomei)
[![Buymecoffee](https://flat.badgen.net/badge/icon/donate?icon=buymeacoffee&color=FF813F&label)](https://www.buymeacoffee.com/bnomei)

Wyrd is a Rust library for composing game behavior as validated signal graphs. You author a
`Weave`, bind it once into dense runtime state, sample host inputs, settle the graph, and apply
its outputs back to your game.

**Engine-neutral** · **`no_std` + `alloc`** · **`f32` or Q16 `i32` signals** · **Bevy 0.19 adapter**

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

## From a knot to a puzzle world

Wyrd is for more than a single switch. Its small unit is a typed knot and its useful unit is a
readable piece of game behaviour: a latched gate, a timed bridge, a multi-object puzzle, or a
one-shot transition request. Compose those machines into rooms; let the host compose rooms into a
game.

```text
Host queries world state
        │
        ▼
active room Weave(s) ── SignalOut / EmitCommand ──► host moves, opens, persists, or transitions
        ▲                                                        │
        └──────────── host samples saved / next-room state ─────┘
```

This division is deliberate. Wyrd has no `Door`, `Room`, `Warp`, `Entity`, or physics knot.
Instead, a host turns "crate is on the sun pad" into a `SignalIn`, and interprets
`"shrine.gate.open"`, `"bridge.target"`, or `"world.request_transition"` as its own effects.
That keeps a puzzle rule portable between engines and makes world ownership explicit.

- Start with a single signal path in the [quickstart](#quickstart).
- Learn the intended scope, multi-room handoff, and the Zelda-/Game Builder Garage-inspired
  composition model in the [vision and scope guide](docs/concepts/vision-and-scope.md).
- Choose a tested puzzle shape—including the chamber-scale capstone—in the
  [executable examples index](docs/examples/README.md).
- Read the [performance model](docs/concepts/performance-model.md) before tuning a per-frame
  integration.

## Quickstart

### Prerequisites

- Rust 1.75 or later
- Rust 1.95 or later when using the Bevy adapter

Add the engine-neutral package under the `wyrd` crate name:

```bash
cargo add wyrd-for-games --rename wyrd
```

This writes the following dependency entry:

```toml
[dependencies]
wyrd = { package = "wyrd-for-games", version = "0.3.0" }
```

To verify a checkout with a small end-to-end test:

```bash
cargo test -p wyrd-for-games --test hello_and_door
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

use wyrd::{is_truthy, weave, BindOpts, HostTime, KnotKind, Runtime, ONE};

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

You can implement `Host` and call `tick_once`, use
`NullHost` or `ScriptedHost` for headless execution, or schedule sample and apply systems around
the Bevy adapter.

## Choose a package

| Package | Use it for |
| --- | --- |
| [`wyrd-for-games`](https://crates.io/crates/wyrd-for-games) | The engine-neutral `wyrd` crate: signals, authoring, validation, binding, runtime, and headless hosts |
| [`wyrd-for-games-bevy`](https://crates.io/crates/wyrd-for-games-bevy) | The `wyrd_bevy` crate: Bevy 0.19 scheduling and host-integration helpers for the `f32` path |

The dependency direction stays one-way:

```text
wyrd-for-games → wyrd-for-games-bevy
```

## Author graphs

Choose the authoring surface by how much topology is known up front:

- Use `weave!` for a fixed, readable graph. It supports explicit author IDs, numeric-path
  selection, pattern instances, and knot-to-pattern connections while lowering through the same
  checked builder API.
- Use `pattern!` when that fixed topology is a reusable fragment with named inputs and outputs.
  It lowers to the same validated `Pattern` model, so a pattern can be included by a weave or
  builder without a second runtime representation.
- Implement `Recipe` when a graph is reusable by a host. Its associated `Ports` type resolves the
  recipe's `SenseId`, `HostPathId`, and `CmdId` handles once after bind; `RecipeInstance` keeps
  them tied to the runtime that created them.
- Use `Scenario::<MyRecipe>::run` for deterministic, closure-scoped recipe frames and typed
  assertions. It is a test and example helper; applications can still drive `Runtime` directly.
- Use `Weave::compose` for generated topology. `Composer` has Bool, Level, and Count wire helpers
  for common operations, while `knot`, `input`, `output`, and `thread` retain the complete
  `WeaveBuilder` catalog as an escape hatch.

`Recipe::manifest` derives a deterministic endpoint summary from the validated weave. With the
optional `schema` feature, the serializable graph and recipe manifest types also implement
`schemars::JsonSchema`; this feature enables `std` and `serde` and is intentionally absent from
the default and `no_std` dependency sets.

Use `WeaveDef` and `PatternDef` for editable or serialized data. Converting a definition into an
immutable `Weave` or `Pattern` performs structural validation. The optional RON and JSON codecs
also validate while loading.

See the [`wyrd` guide](crates/wyrd-for-games/README.md) for complete authoring examples.

## Integrate with Bevy

`wyrd-for-games-bevy` configures three ordered system sets:

```text
WyrdSet::Sample → WyrdSet::Loom → WyrdSet::Apply
```

Add both published packages under their library target names:

```toml
[dependencies]
wyrd = { package = "wyrd-for-games", version = "0.3.0" }
wyrd_bevy = { package = "wyrd-for-games-bevy", version = "0.3.0" }
```

Use the adapter through `wyrd_bevy` alongside the engine-neutral `wyrd` API.

The plugin owns only the loom step. Your systems read components during `Sample` and mutate
components during `Apply`. Bevy messages such as `WyrdSignalConfirm` confirm applied host effects;
they are not graph threads.

Run the headless two-plate door example:

```bash
cargo run -p wyrd-for-games-bevy --example and_door
```

The example samples two plate states, settles an `And` knot, applies `SignalOut("door.open")` to a
host-owned `Door` component, and emits confirmations when the component changes. See the
[`wyrd_bevy` guide](crates/wyrd-for-games-bevy/README.md) for the exact ownership boundary.

## Learn through executable recipes

[`wyrd::cookbook`](crates/wyrd-for-games/README.md) provides 21 recipes used by both
rustdoc and integration tests.

| Tier | Focus | Recipes |
| --- | --- | --- |
| **A** | Foundations | Not, two-input And, bind/sample/loom, `tick_once`, validation failure |
| **B** | Reusable Weaves | Monostable Pattern, two-plate door, Flag, Counter threshold, Delay |
| **C** | Game-logic patterns | Latches, timers, cooldowns, Threshold, Map, Digitize, OnStart, Emit, Or |
| **D** | Chamber-scale composition | Multi-object latch, moving-host target, and one-shot room-transition request |

Run the complete ladder:

```bash
cargo test -p wyrd-for-games --test tutorial_ladder
cargo test -p wyrd-for-games --doc
```

For a narrative index rather than a flat recipe list, see [choose a puzzle
shape](docs/examples/README.md). The Tier D capstone is deliberately engine-neutral: it proves
the rule circuit while leaving spatial queries, movement, persistence, and room loading to the
host.

## Features and numeric paths

Enable exactly one of `signal-f32` and `signal-i32`.

| Feature | Crates | Behavior |
| --- | --- | --- |
| `std` (default) | `wyrd-for-games` | Desktop/test support through `no-std-compat` |
| `alloc` | `wyrd-for-games` | Heap-backed graph/runtime storage without `std` |
| `signal-f32` (default) | `wyrd-for-games` | Floating-point signal path; uses `libm` for `no_std` square root |
| `signal-i32` | `wyrd-for-games` | Q16 integer signal path for constrained hosts |
| `serde` | `wyrd-for-games` | Serde derives for author definitions |
| `serde-ron` | `wyrd-for-games` | RON load/save with validation on load |
| `serde-json` | `wyrd-for-games` | JSON load/save with validation on load |
| `schema` | `wyrd-for-games` | Opt-in `std` + `serde` JSON Schema support for graph and recipe manifests |
| `bevy_log` | `wyrd-for-games-bevy` | Forwards Bevy's `bevy_log` feature |

`wyrd-for-games-bevy` always uses `signal-f32`. Use `wyrd-for-games` directly for `signal-i32`
hosts.

CI verifies both numeric paths, codecs, runtime `no_std` builds, and Bevy.

## Playdate / constrained hosts

Use `wyrd-for-games` directly rather than the Bevy adapter, selecting the integer signal
path and the allocator supplied by the host application:

```toml
[dependencies]
wyrd = { package = "wyrd-for-games", version = "0.3.0", default-features = false, features = ["alloc", "signal-i32"] }
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
```

Run the full local performance suite:

```bash
cargo bench --workspace
```

CodSpeed runs the same runtime and Bevy benchmark targets on pushes to `main` and pull requests.

CI enforces the numeric/codec matrix, Bevy builds, runtime `no_std` checks, warnings as errors,
and line-coverage gates.

## Reference and next steps

- [`wyrd` authoring, runtime, and cookbook](crates/wyrd-for-games/README.md)
- [`wyrd_bevy` integration boundary](crates/wyrd-for-games-bevy/README.md)
- [Vision, scope, and game-scale composition](docs/concepts/vision-and-scope.md)
- [Executable puzzle-shape index](docs/examples/README.md)
- [Performance model and measurement guidance](docs/concepts/performance-model.md)
- [`wyrd` API reference](https://docs.rs/wyrd-for-games)
- [`wyrd_bevy` API reference](https://docs.rs/wyrd-for-games-bevy)
- [Changelog](CHANGELOG.md)

## License

MIT
