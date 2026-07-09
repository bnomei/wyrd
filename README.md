# Wyrd

A small Rust library for composing game behavior from typed signal graphs, using a Norse-inspired weaving metaphor.

**Engine-neutral** · **dual numeric paths** (`f32` / `i32` Q16) · **`no_std` + `alloc`**

## Crates

| Crate | Role |
| --- | --- |
| [`wyrd-core`](crates/wyrd-core) | `Signal`, dense ids, closed port tables, `KnotKind` |
| [`wyrd-graph`](crates/wyrd-graph) | Author `Weave`, builder, validate |
| [`wyrd-runtime`](crates/wyrd-runtime) | Bind → sample → loom → outbox |

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

## Features

| Feature | Meaning |
| --- | --- |
| `std` (default) | Desktop / tests via `no-std-compat` |
| `signal-f32` (default) | Float wire path |
| `signal-i32` | Integer Q16 path (Playdate-class) |

Enable exactly one of `signal-f32` / `signal-i32`.

## Docs (local / gitignored)

Design notes live under `docs/` (see `.gitignore`). Start with:

- `docs/research/decisions.md` — locked decisions  
- `docs/api-preview/11_revised_surface.rs` — v2 API pencil  
- `docs/primitives/port-schema.md` — closed port tables  

## License

MIT
