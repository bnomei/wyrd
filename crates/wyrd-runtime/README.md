# wyrd-runtime

Bind author Weaves to dense ids, loom settle, outbox for hosts.

## Host loop

```text
let mut rt = Runtime::bind(&weave, BindOpts::default())?;
// once: resolve sense_id / path_id for hot path

// each frame:
rt.begin_frame(HostTime { tick });
{
    let mut w = rt.port_writer();
    w.set_sense(plate_id, value);  // KnotId — never string on hot path
}
rt.loom(&weave)?;
for s in rt.outbox().signals() {
    // s.path: HostPathId → rt.path_name(s.path) if needed
}
for e in rt.outbox().emits() {
    // e.cmd: CmdId
}
```

Steady-state loom does not allocate topology (inbound edges + slots precomputed at bind).
