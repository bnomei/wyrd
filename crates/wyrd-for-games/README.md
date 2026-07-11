# wyrd-for-games

`wyrd-for-games` provides engine-neutral signal-graph game logic: author and
validate a `Weave`, bind it to dense runtime ids, settle it once per frame, and
apply its output in your game host.

The library target is named `wyrd`:

```rust
use wyrd::{weave, KnotKind, SignalDomain};

let weave = weave! {
    id: "door";
    knots {
        plate = KnotKind::signal_in(SignalDomain::Bool);
        door = KnotKind::signal_out("door.open", SignalDomain::Bool);
    }
    threads {
        plate.out -> door.in;
    }
}?;
# Ok::<(), wyrd::BuildError>(())
```

Use `wyrd::core`, `wyrd::graph`, and `wyrd::runtime` when you want an explicit
layer namespace. The crate supports `no_std` plus `alloc`, `signal-f32`
(default) or `signal-i32`, and optional `serde` codecs.

## Host loop

```text
let mut runtime = Runtime::bind(weave, BindOpts::default())?;
// Once: resolve SenseId / HostPathId for the hot path.

// Each frame:
runtime.begin_frame(HostTime { tick });
{
    let mut ports = runtime.port_writer();
    ports.set_sense(plate_id, value)?; // SenseId — never a string on the hot path.
}
runtime.loom();
for signal in runtime.outbox().signals() {
    // signal.path: HostPathId → runtime.path_name(signal.path) when needed.
}
for command in runtime.outbox().emits() {
    // command.cmd: CmdId
}
```

Steady-state loom does not allocate topology: inbound edges and slots are
precomputed during bind. Keep active Weaves scoped to a room or puzzle island
and bind them on load, not once per frame.

## Tutorial ladder

The `wyrd::cookbook` module provides Tier A foundations, Tier B first Weaves,
Tier C game-logic patterns, and Tier D chamber composition. Tier D combines
host-owned observations into a latched gate, continuous mover target, and
edge-triggered room-transition request.

```bash
cargo test -p wyrd-for-games --test tutorial_ladder
cargo test -p wyrd-for-games --doc
```
