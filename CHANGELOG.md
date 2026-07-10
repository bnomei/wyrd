# Changelog

## Unreleased

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
- Divan: `settle_not_chain`, `settle_and_door`, `tick_once_not_chain`
