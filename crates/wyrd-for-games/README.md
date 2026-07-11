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
