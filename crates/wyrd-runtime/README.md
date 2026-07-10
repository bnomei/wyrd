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

Steady-state loom does not allocate topology (inbound edges + slots precomputed at bind).

## Tutorial ladder

Pedagogy module [`cookbook`](src/cookbook/) (Tier A foundations → B first Weaves → C GBG/Zelda patterns):

```bash
cargo test -p wyrd-runtime --test tutorial_ladder
cargo test -p wyrd-runtime --doc
```
