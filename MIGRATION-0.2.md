# Migrating from Wyrd 0.1 to 0.2

Wyrd 0.2 is a clean pre-1.0 redesign. It does not provide deprecated wrappers for the
0.1 Rust API, and 0.1 serialized assets should be regenerated through the 0.2 definition
types.

## Build an immutable graph

`WeaveBuilder` now mutates in place and returns owner-aware handles. Connections use
direction-typed endpoints, so a reversed edge cannot reach validation.

```rust
use wyrd_core::KnotKind;
use wyrd_graph::{BuildError, Weave, WeaveBuilder};

fn graph() -> Result<Weave, BuildError> {
    let mut builder = WeaveBuilder::new("hello")?;
    let source = builder.knot("source", KnotKind::signal_in())?;
    let invert = builder.knot("invert", KnotKind::not())?;
    let sink = builder.knot("sink", KnotKind::signal_out("debug.inverted"))?;

    let source_out = builder.output(&source, "out")?;
    let invert_in = builder.input(&invert, "in")?;
    builder.connect(source_out, invert_in)?;

    let invert_out = builder.output(&invert, "out")?;
    let sink_in = builder.input(&sink, "in")?;
    builder.connect(invert_out, sink_in)?;

    Ok(builder.build()?)
}
```

For static graphs, `wyrd_graph::weave!` expands through the same builder and returns the
same contextual errors:

```rust
# use wyrd_core::KnotKind;
# use wyrd_graph::{weave, BuildError, Weave};
fn graph() -> Result<Weave, BuildError> {
    weave! {
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
    }
}
```

`Weave`, `Pattern`, and their contents are immutable after validation. Code that loads or
generates editable data should use `WeaveDef` and `PatternDef`, then convert with
`TryFrom`. Pattern expansion belongs to `WeaveBuilder::include`; mutation through
`merge_expanded` no longer exists.

## Bind once and execute only the runtime

`Runtime::bind` consumes the validated graph. The bound runtime is the only executable
artifact, and `loom` no longer accepts or returns a graph result.

```rust
let mut runtime = Runtime::bind(graph()?, BindOpts::default())?;
let sense = runtime.sense_id("source").expect("validated sense");

runtime.begin_frame(HostTime { tick: 0 });
runtime.port_writer().set_sense(sense, ONE)?;
runtime.loom();
let outbox = runtime.outbox();
```

`tick_once` now accepts only `host` and `runtime`. `Host::sample_into` returns
`Result<(), HandleError>` so invalid sampling handles propagate instead of being ignored.

## Handle and error changes

- Dense ID tuple fields are private. Use runtime resolvers and `.get()` for diagnostics.
- Host sampling uses `SenseId`, not an arbitrary `KnotId`.
- `path_name` and `cmd_name` return `Option<&str>`.
- Checked runtime reads and writes return `Result`; invalid operations never return zero or
  silently do nothing.
- `WyrdError` and the shared `Result` alias are replaced by `BuildError`,
  `ValidationError`, `JsonCodecError`, `RonCodecError`, `BindError`, and `HandleError`.
  Match the error associated with the boundary being called and retain codec sources for
  parser diagnostics.

