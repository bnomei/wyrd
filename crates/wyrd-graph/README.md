# wyrd-graph

Authoring surface: immutable validated `Weave` values, the typed builder,
validation (DAG, fan-in, ports, budgets), and expand-at-load `Pattern` values.

Use the builder for generated graphs. For static graphs, `weave!` provides the
same checks with a compact declarative syntax:

```rust
use wyrd_graph::{weave, KnotKind};

let graph = weave! {
    id: "door";
    knots {
        plate_a = KnotKind::signal_in();
        plate_b = KnotKind::signal_in();
        both = KnotKind::and2();
        door as "door.output" = KnotKind::signal_out("door.open");
    }
    threads {
        plate_a.out -> both.in_0;
        plate_b.out -> both.in_1;
        both.out -> door.in;
    }
}?;
# Ok::<(), wyrd_graph::BuildError>(())
```

An optional `numeric: ...;` declaration follows `id`. An optional `patterns`
block follows `knots`; declare instances as `hold = ("hold-1", &monostable);`
and connect exports with `hold.in("start")` and `hold.out("active")`. Knot and
pattern endpoints can be mixed in either direction supported by their port
types. Runtime never sees a `Pattern`; inclusion expands it before validation.
