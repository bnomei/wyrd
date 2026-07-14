# Changelog

All notable public changes to Wyrd are documented here. The crates follow
[Cargo SemVer](https://doc.rust-lang.org/cargo/reference/semver.html); while versions are below
1.0, a minor release may contain breaking public-API changes.

## Unreleased

### Added

- Add durable, versioned `RuntimeState` checkpoints with Serde plus optional RON/JSON codecs,
  fresh-runtime restore, and semantic inspection reports.
- Add authored-name `RuntimePreset` startup values for Flags, Counters, and held SignalIn values.
- Add a Tier D executable example covering checkpoint load boundaries and presets.

## 0.4.0 - 2026-07-14

Version 0.4.0 is the first crates.io release after 0.2.0 and includes the repository-only 0.3
milestones described below.

### Added

- Add opaque, versioned `RuntimeState` continuation snapshots with executable fingerprints,
  transactional restore validation, and reusable `snapshot_into` buffers.

### Changed

- Replace the executable `wyrd::cookbook` API with human-readable, doctest-backed lessons under
  `wyrd::examples`, organized as an ordered Tier A through Tier D learning path.

### Performance

- Reduce immutable graph metadata retained by a bound runtime and cap emit-outbox reservation.

### Compatibility

- Establish 0.4.0 as the published public Rust API baseline for automated compatibility checks
  before later releases. Serialized graph definitions remain a versioned compatibility boundary
  rather than a stable cross-minor file format.

## 0.3.0 - 2026-07-12

The repository's `v0.3.1` tag did not change the Cargo package version and was not published to
crates.io. It is not a separate crate release.

### Breaking changes

- Add Bool, Level, and Count signal domains with graph-time compatibility validation; use explicit
  conversion knots to cross domains.

### Added

- Add declarative `weave!` recipe topology across the cookbook and Bevy door example, plus
  `pattern!` for reusable validated fragments with named inputs and outputs.
- Add `Recipe`, `RecipeInstance`, and contextual port-resolution errors so generic hosts resolve
  typed runtime handles once rather than carrying endpoint strings through their tick loop.
- Add closure-scoped `Scenario` frames and assertions for deterministic recipe examples and tests.
- Add `Weave::compose` and `Composer`: Bool, Level, and Count typed wires for generated topology,
  with full-catalog `WeaveBuilder` escape hatches.
- Add deterministic `RecipeManifest` endpoint summaries and the opt-in `schema` feature for
  `schemars::JsonSchema`; default and `no_std` builds do not include schema dependencies.
- Add `WyrdRecipePlugin<R>` and `WyrdRecipeInstance<R>` for generic Bevy recipe binding while
  retaining game-owned Sample and Apply systems.

### Changed

- Update the Bevy adapter to Bevy 0.19.

### Performance

- Optimize integer `Map` and `Sqrt` execution for constrained hosts such as Playdate.

## 0.2.0

### Breaking API redesign

- Split serde-facing `WeaveDef` / `PatternDef` from immutable, validated `Weave` / `Pattern` values.
- Replaced the consuming builder with owner-aware knot handles and direction-typed ports.
- Made `Runtime` consume its `Weave`; `loom()` is now graph-free and infallible after bind.
- Replaced the monolithic `WyrdError` and silent handle failures with contextual graph, codec,
  bind, and handle errors.
- Privatized dense ID fields and added the host-writable `SenseId` type.
- Added the declarative `weave!` graph and pattern authoring macro.

The 0.1 Rust API and serialized graph schema are not compatibility targets.

### Packaging

- Publish the engine-neutral API as `wyrd-for-games` (library target `wyrd`) and the Bevy adapter
  as `wyrd-for-games-bevy` (library target `wyrd_bevy`). The former internal crate split is not
  published and has no compatibility packages.

### Pedagogy

- **Tutorial ladder** (`wyrd::cookbook`): Tier A (5) → B (5) → C (10) runnable recipes;
  `tests/tutorial_ladder.rs`; `patterns_cookbook` thin-wraps Tier B; short rustdoc doctests

### Catalog (high + medium impact)

- **Select** — truthy `sel` → `b`, else `a`
- **Digitize** — quantize into `steps` bins over in→out ranges; `steps=0` or inverted in-range → `InvalidParam` (same inverted-range rule on **Map**)
- **Threshold** — level out + crossed_up/crossed_down; optional hysteresis
- **Random** — host `Seed` / `reseed`; optional rising gate; min/max ports; holds last sample
- **Sqrt** — core `f32::sqrt` (f32) / integer isqrt (i32); non-positive → 0
- **Xor** — truthy exclusive-or
- **FallingToZero** / **Change** — edge pulses
- **Clamp** — `[min, max]`; `min > max` → `InvalidParam`

### Host abstraction (`wyrd-for-games`)

- `Host` trait: `time`, `sample_into(PortWriter)`, `apply(Outbox)`
- `tick_once` — begin_frame → sample → loom → apply
- `HostCommand::{SetLevel, Emit}` (dense `HostPathId` / `CmdId`)
- `append_commands` / `outbox_to_commands`
- `NullHost`, `ScriptedHost` for headless / scripted replay

### Validate / budgets (`wyrd-for-games`)

- Soft + hard budget fields (knots, threads, chain depth, fan-out, delay path sum)
- Hard enforcement of chain depth, fan-out, delay path sum
- `validate_report` + `BudgetWarning` / `ValidateReport` (soft never fails bind)
- `BindOpts.budget` and `BindOpts.max_emits_per_tick` (default 8)
- **JSON codec** (`serde-json`): `from_json` / `to_json` with same numeric + validate gates as RON

### Loom

- EmitCommand **enable** port (unconnected = enabled)
- Emit-per-tick hard cap (silent drop)

### Bevy (`wyrd-for-games-bevy`)

- `Door` host component + `apply_signal_bool`
- `WyrdSignalConfirm` Message (confirmation only, not topology)
- `and_door` example applies to Door entity

### Pedagogy / docs

- Pattern cookbook: five CI recipes (`patterns_cookbook`)
- Root README: Host tick, first five Weaves, door-as-host-effect
- Local `docs/perf.md` with measured settle benches (gitignored `docs/`)

### Tests / perf

- `zero_alloc_loom` — outbox capacity + delay_buf length stability
- Divan suite **split**: `settle_chain`, `settle_catalog`, `settle_stateful`, `bind`
  (shared `benches/common.rs`; `autobenches = false`)
- Measured f32 + i32 tables in local `docs/perf.md`
- **P0 scaled chains**: Map / Digitize / Calc(Mul) / Sqrt / Delay×n (amortized arm cost)
- **P1**: stateful kit (Counter/Flag/Timers), emit storm, Calc(Div) chain
- **P2**: edges pack, Or/Xor/Select, Clamp/Neg chain, Compare chain, OnStart; completeness table
- **P3**: pattern expand/include bind benches; Bevy headless `host_tick`
- **Settle structural pass**: bind-time `KindTag` cache, CSR inbound, flat clear indices,
  hot port access, Sense skip in topo; Digitize bin path tweak; `settle_iso` isolation benches
- **Arm-math + residual structure**: Digitize/Map bind precompute (+ f32 Digitize
  `bin_scale`/`mul_add`); Sqrt via `f32::sqrt` + Newton isqrt (drop `libm`); sense seed
  list; Calc tags split by op; Compare const as `Signal`; Emit/Random wire flags at bind
- **Ranks 1–8 settle pass**: `div` by ONE identity; `CalcDivConst` when `b` is Constant;
  gather n=1/2 fast path; clear only unwired Ins; Delay power-of-two head mask; new
  `settle_iso` filters (div/clamp/compare/delay)
