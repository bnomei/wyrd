# wyrd-bevy

Thin Bevy adapter for Wyrd. Core stays engine-neutral; this crate only:

1. Holds the bound `Runtime` as the sole executable Bevy resource
2. Runs `begin_frame` + `loom` in an ordered `SystemSet`
3. Leaves **sample** and **apply** to the host game  

`Runtime::bind` consumes the validated `Weave`; the adapter does not retain a
second graph that could drift from runtime state. Never store `Entity` as a
Thread endpoint. Resolve `SenseId` / `HostPathId` at setup.

## Host order

```text
WyrdSet::Sample  → write senses (dense SenseId)
WyrdSet::Loom    → begin_frame + loom (plugin)
WyrdSet::Apply   → read outbox → mutate Components → optional Messages
```

**Messages ≠ Threads.** `WyrdSignalConfirm` is a host confirmation after apply
(VFX/UI). Topology lives only in the Weave.

**Door is a host effect.** The demo `Door` component is not a Knot; the Weave
only has `SignalOut("door.open")`.

Helpers: `set_sense_bool`, `signal_truthy`, `apply_signal_bool`. Each returns a
`Result` and rejects a handle resolved from a different `WyrdInstance`.

## Numeric path: **signal-f32 only**

Bevy is float-native (`Transform`, time, etc.). This crate **always** depends on
`wyrd-*` with `signal-f32`. It does **not** offer `signal-i32`.

CI exercises integer / Q16 dual-path coverage on **core / graph / runtime**.

Playdate-class hosts should depend on `wyrd-runtime` with `signal-i32` directly,
not through `wyrd-bevy`.

## Example

```bash
cargo run -p wyrd-bevy --example and_door
```

Headless loop: two plate senses → And → SignalOut; host applies to a `Door`
entity and emits `WyrdSignalConfirm` when `open` changes.
