# API preview (faux Rust)

**Status:** historical · outside-in sketches · **not** compile targets  

**Shipped tutorials:** use [`wyrd_runtime::cookbook`](../../crates/wyrd-runtime/src/cookbook/) and  
`cargo test -p wyrd-runtime --test tutorial_ladder` (Tier A → B → C). Prefer those over `01`–`10` below — several v1 sketches have wrong ports (see [review 05](./reviews/05-pedagogy-and-patterns.md)).

**Purpose (archived):** Feel the public surface before crates exist. Prefer **rustic** — small types, explicit steps, few traits.

These files are **design sketches**. Names and shapes may change when real crates land. They encode:

- Dual numeric paths (`f32` / `i32`) as a type alias or feature, not dual graphs
- Educational Sense → Rune → Act wiring (GBG-category pedagogy, Wyrd names)
- Host at the edge; core never knows doors/cameras
- `no_std` + `alloc` via `no-std-compat` (shown as `std::` imports)

## v2 surface (preferred)

**Consolidated redesign:** [`11_revised_surface.rs`](./11_revised_surface.rs)

Synthesis of harsh reviews on the v1 sketches: dense ids after bind, closed port tables, host outbox without `String`, monomorphic `Signal`, corrected pedagogy graphs, expand-at-load Patterns.

| Doc | Role |
| --- | --- |
| [`reviews/00-synthesis.md`](./reviews/00-synthesis.md) | Unified principles · MUST vs NICE |
| [`reviews/01-typed-ids-vs-strings.md`](./reviews/01-typed-ids-vs-strings.md) | Stringly vs typed ids |
| [`reviews/02-builder-ergonomics.md`](./reviews/02-builder-ergonomics.md) | Builder / wire API |
| [`reviews/03-host-boundary.md`](./reviews/03-host-boundary.md) | Host, outbox, Bevy, no_std |
| [`reviews/04-dual-signal-api.md`](./reviews/04-dual-signal-api.md) | Dual path + SignalOps |
| [`reviews/05-pedagogy-and-patterns.md`](./reviews/05-pedagogy-and-patterns.md) | Sense/Rune/Act examples, Pattern, Seed |

Treat **`01`–`10` as historical v1 sketches**; implement from **`11` + synthesis**.

**Shipped skeleton:** workspace crates `wyrd-core`, `wyrd-graph`, `wyrd-runtime` implement v2 ids, port schema, bind/loom, and the hello / and-door tests.

## Reading order (v1 sketches)

| # | File | Outside-in layer |
| --- | --- | --- |
| 1 | [`01_hello_weave.rs`](./01_hello_weave.rs) | Author a tiny Weave in code |
| 2 | [`02_host_tick.rs`](./02_host_tick.rs) | Host samples → loom → commands |
| 3 | [`03_and_door.rs`](./03_and_door.rs) | Classic multi-switch → Act (host binds “door”) |
| 4 | [`04_timer_counter.rs`](./04_timer_counter.rs) | Timer + Counter + Flag |
| 5 | [`05_builder_style.rs`](./05_builder_style.rs) | Fluent rustic builder |
| 6 | [`06_serde_ron.rs`](./06_serde_ron.rs) | Authored asset shape |
| 7 | [`07_seed_and_random.rs`](./07_seed_and_random.rs) | Seeder / Random (v1 sketch) |
| 8 | [`08_pattern_expand.rs`](./08_pattern_expand.rs) | Pattern stamp at load |
| 9 | [`09_bevy_thin.rs`](./09_bevy_thin.rs) | Bevy adapter is thin |
| 10 | [`10_signal_ops.rs`](./10_signal_ops.rs) | Dual path ops (f32 vs i32) |
| **11** | [`11_revised_surface.rs`](./11_revised_surface.rs) | **v2 consolidated surface** |

Narrative notes: [`outside-in.md`](./outside-in.md) · Random: [`random-and-seed.md`](./random-and-seed.md)

## Rustic rules for this preview

1. Prefer `enum` + `match` over trait objects.
2. Prefer `Weave::builder()` or plain `struct` fill over macro DSLs (macros optional later).
3. One settle call: `runtime.loom(&weave)` after `begin_frame(tick)`.
4. **Author ports/names are strings; runtime ports are `PortSlot` / dense ids — not Entity.** (v2; v1 said “stringy or small ids.”)
5. No engine types in core examples.
6. Fail loud at `validate()`; settle never panics.
7. Examples monomorphic on feature-selected `Signal`; no public `Runtime<S>`.
8. Host hot path: `set_sense(KnotId, …)` + `Outbox` with `HostPathId` / `CmdId` — no owned `String` in loom.
