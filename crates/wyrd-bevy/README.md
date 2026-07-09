# wyrd-bevy

Thin Bevy adapter for Wyrd. Core stays engine-neutral; this crate only:

1. Holds bound `Runtime` + `Weave` as a Bevy resource  
2. Runs `begin_frame` + `loom` in a ordered `SystemSet`  
3. Leaves **sample** and **apply** to the host game  

Never store `Entity` as a Thread endpoint. Resolve `KnotId` / `HostPathId` at setup.

## Example

```bash
cargo run -p wyrd-bevy --example and_door
```

Headless: drives two plate senses and prints `door.open`.
