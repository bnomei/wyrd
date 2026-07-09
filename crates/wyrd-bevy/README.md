# wyrd-bevy

Thin Bevy adapter for Wyrd. Core stays engine-neutral; this crate only:

1. Holds bound `Runtime` + `Weave` as a Bevy resource  
2. Runs `begin_frame` + `loom` in a ordered `SystemSet`  
3. Leaves **sample** and **apply** to the host game  

Never store `Entity` as a Thread endpoint. Resolve `KnotId` / `HostPathId` at setup.

## Numeric path: **signal-f32 only**

Bevy is float-native (`Transform`, time, etc.). This crate **always** depends on
`wyrd-*` with `signal-f32`. It does **not** offer `signal-i32`.

Integer / Q16 dual-path coverage lives on **core / graph / runtime**:

```bash
./scripts/dual-check.sh
# or CI job dual-signal (signal-i32)
```

Playdate-class hosts should depend on `wyrd-runtime` with `signal-i32` directly,
not through `wyrd-bevy`.

## Example

```bash
cargo run -p wyrd-bevy --example and_door
```

Headless: drives two plate senses and prints `door.open`.
