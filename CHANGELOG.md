# Changelog

## Unreleased

### Catalog (high + medium impact)

- **Select** — truthy `sel` → `b`, else `a`
- **Digitize** — quantize into `steps` bins over in→out ranges; `steps=0` or inverted in-range → `InvalidParam` (same inverted-range rule on **Map**)
- **Threshold** — level out + crossed_up/crossed_down; optional hysteresis
- **Random** — host `Seed` / `reseed`; optional rising gate; min/max ports; holds last sample
- **Sqrt** — `libm::sqrtf` (f32) / integer isqrt (i32); non-positive → 0
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
