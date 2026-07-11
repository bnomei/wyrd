# wyrd-runtime

Bind author Weaves to dense ids, loom settle, outbox for hosts.

## Host loop

```text
let mut rt = Runtime::bind(weave, BindOpts::default())?;
// once: resolve sense_id / path_id for hot path

// each frame:
rt.begin_frame(HostTime { tick });
{
    let mut w = rt.port_writer();
    w.set_sense(plate_id, value)?; // SenseId — never string on hot path
}
rt.loom();
for s in rt.outbox().signals() {
    // s.path: HostPathId → rt.path_name(s.path) if needed
}
for e in rt.outbox().emits() {
    // e.cmd: CmdId
}
```

Steady-state loom does not allocate topology (inbound edges + slots precomputed at bind). It is a
single full pass over the bound DAG, so keep active Weaves scoped to a room or puzzle island and
bind them on load—not once per frame. Read the [performance model](../../docs/concepts/performance-model.md)
for limits, measurement, and the precise allocation boundary.

## Tutorial ladder

Pedagogy module [`cookbook`](src/cookbook/) (Tier A foundations → B first Weaves → C game-logic
patterns → D chamber composition):

```bash
cargo test -p wyrd-runtime --test tutorial_ladder
cargo test -p wyrd-runtime --doc
```

Tier D joins those small recipes into an engine-neutral chamber: several host observations latch a
gate, a continuous output controls a host-owned mover target, and a rising edge requests a room
transition. See [choose a puzzle shape](../../docs/examples/README.md) and
[`tier_d`](src/cookbook/tier_d.rs).
