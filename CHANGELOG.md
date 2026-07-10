# Changelog

## Unreleased

## 0.2.0

### Breaking API redesign

- Split serde-facing `WeaveDef` / `PatternDef` from immutable, validated `Weave` / `Pattern` values.
- Replaced the consuming builder with owner-aware knot handles and direction-typed ports.
- Made `Runtime` consume its `Weave`; `loom()` is now graph-free and infallible after bind.
- Replaced the monolithic `WyrdError` and silent handle failures with contextual graph, codec,
  bind, and handle errors.
- Privatized dense ID fields and added the host-writable `SenseId` type.
- Added the declarative `weave!` graph and pattern authoring macro.

The 0.1 Rust API and serialized graph schema are not compatibility targets. See
[`MIGRATION-0.2.md`](MIGRATION-0.2.md).

### Pedagogy

- **Tutorial ladder** (`wyrd_runtime::cookbook`): Tier A (5) → B (5) → C (10) runnable recipes;
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

### Host abstraction (`wyrd-runtime`)

- `Host` trait: `time`, `sample_into(PortWriter)`, `apply(Outbox)`
- `tick_once` — begin_frame → sample → loom → apply
- `HostCommand::{SetLevel, Emit}` (dense `HostPathId` / `CmdId`)
- `append_commands` / `outbox_to_commands`
- `NullHost`, `ScriptedHost` for headless / scripted replay

### Validate / budgets (`wyrd-graph`)

- Soft + hard budget fields (knots, threads, chain depth, fan-out, delay path sum)
- Hard enforcement of chain depth, fan-out, delay path sum
- `validate_report` + `BudgetWarning` / `ValidateReport` (soft never fails bind)
- `BindOpts.budget` and `BindOpts.max_emits_per_tick` (default 8)
- **JSON codec** (`serde-json`): `from_json` / `to_json` with same numeric + validate gates as RON

### Loom

- EmitCommand **enable** port (unconnected = enabled)
- Emit-per-tick hard cap (silent drop)

### Bevy (`wyrd-bevy`)

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
