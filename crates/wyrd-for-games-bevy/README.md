# wyrd-for-games-bevy

Thin Bevy adapter for Wyrd. Core stays engine-neutral; this crate only:

1. Holds bound `Runtime`s as the sole executable Bevy resource
2. Binds typed `Recipe` ports during startup through `WyrdRecipePlugin<R>`
3. Runs `begin_frame` + `loom` in an ordered `SystemSet`
4. Leaves **sample** and **apply** to the host game

Install the core and adapter under their library target names:

```toml
[dependencies]
wyrd = { package = "wyrd-for-games", version = "0.4.0" }
wyrd_bevy = { package = "wyrd-for-games-bevy", version = "0.4.0" }
```

`Runtime::bind` consumes the validated `Weave`; the adapter does not retain a
second graph that could drift from runtime state. Never store `Entity` as a
Thread endpoint. Resolve `SenseId` / `HostPathId` at setup.

## Host order

```text
WyrdSet::Sample  → write senses (dense SenseId)
WyrdSet::Loom    → begin_frame + loom (plugin)
WyrdSet::Apply   → read outbox → mutate Components → optional Messages
```

`WyrdWorld` can hold several independently bound `WyrdInstance`s. The plugin looms every active
instance, so the host chooses whether to keep a runtime per currently simulated chamber or remove
and stash inactive rooms. Instances do not share Threads; persist cross-room progress in the host
and sample it into the next room's senses. See [vision and scope](https://github.com/bnomei/wyrd/blob/v0.4.0/docs/concepts/vision-and-scope.md)
for the room/world boundary.

## Typed recipes

For a reusable weave, implement `Recipe` once and add the generic plugin beside
`WyrdPlugin`:

```rust,no_run
# use bevy::prelude::*;
# use wyrd::runtime::Recipe;
# use wyrd_bevy::{WyrdPlugin, WyrdRecipePlugin};
# struct DoorRecipe;
# impl Recipe for DoorRecipe {
#     type Ports = ();
#     fn weave() -> Result<wyrd::graph::Weave, wyrd::BuildError> { todo!() }
#     fn resolve_ports(_: &wyrd::Runtime) -> Result<Self::Ports, wyrd::RecipeResolveError> { Ok(()) }
# }
# let mut app = App::new();
app.add_plugins((WyrdPlugin, WyrdRecipePlugin::<DoorRecipe>::default()));
```

Startup builds the recipe, binds it, resolves `DoorRecipe::Ports`, and inserts
the resulting runtime in `WyrdWorld`. Systems read
`Res<WyrdRecipeInstance<DoorRecipe>>` and pair its typed ports with a live
runtime through `get` or `get_mut`. A failed build/bind/resolve remains visible
through `error()` and never inserts a partial instance; if the host later
removes the instance, `get` and `get_mut` safely return `None`.

The recipe plugin never adds Sample or Apply systems. Games still own the
translation between components and typed ports, keeping game effects explicit.

**Messages ≠ Threads.** `WyrdSignalConfirm` is a host confirmation after apply
(VFX/UI). Topology lives only in the Weave.

**Door is a host effect.** The demo `Door` component is not a Knot; the Weave
only has `SignalOut("door.open")`.

Helpers: `set_sense_bool`, `signal_truthy`, `apply_signal_bool`. Each returns a
`Result` and rejects a handle resolved from a different `WyrdInstance`.

## Numeric path: **signal-f32 only**

Bevy is float-native (`Transform`, time, etc.). This crate **always** depends on
`wyrd-for-games` with `signal-f32`. It does **not** offer `signal-i32`.

CI exercises integer / Q16 dual-path coverage on **wyrd-for-games**.

Playdate-class hosts should depend on `wyrd-for-games` with `signal-i32` directly,
not through `wyrd-for-games-bevy`.

## Example

```bash
cargo run -p wyrd-for-games-bevy --example and_door
```

Headless loop: two plate senses → And → SignalOut; host applies to a `Door`
entity and emits `WyrdSignalConfirm` when `open` changes. The sample uses
`WyrdRecipePlugin<AndDoorRecipe>` rather than hand-building port resources.
