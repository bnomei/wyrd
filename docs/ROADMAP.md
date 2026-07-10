# Wyrd — Roadmap checklist

**Status:** living checklist · 2026-07-10  
**Source of truth for intent:** [`vision.md`](./vision.md), [`research/decisions.md`](./research/decisions.md)  
**Safe public line:** A small Rust library for composing game behavior from typed signal graphs, using a Norse-inspired weaving metaphor.

Use this file as the **progress board**. Mark items `[x]` only when they are true in the **shipped crates** (or explicitly documented as complete tooling), not when they exist only in research notes.

| Mark | Meaning |
| --- | --- |
| `[x]` | Done in tree (tests/CI or demo as noted) |
| `[ ]` | Not done — still open for current / near-term product |
| `[ ]` *(later)* | Intentionally deferred; do not treat as v0 blocker |

---

## At a glance

| Phase | Theme | Status |
| --- | --- | --- |
| **0** | Vision / research | Done (local `docs/`) |
| **1** | Workspace scaffold + CI | Done |
| **2** | Core loom (value, weave, validate, settle) | Done |
| **3** | Catalog v0 | Mostly done (polish remains) |
| **4** | Bevy MVP | Component + Message demo done (headless) |
| **5** | Perf pass | Measured benches + docs/perf.md |
| **6** | Patterns as product | Expand-at-load + cookbook CI recipes |
| **Host** | Generic Host trait + command apply | Done in runtime |
| **7** | Loom editor | Later |
| **8+** | Other engines, Vec2, editor, … | Later |

---

## Phase 0 — Vision / research

- [x] Vision doc (`docs/vision.md`)
- [x] Provenance research (SoG trigger-effect wiring under `docs/research/provenance/`)
- [x] Ideation + synthesis + Tavily raw notes
- [x] Locked decisions log (`docs/research/decisions.md`)
- [x] Primitives deep dive (`docs/primitives/`)
- [x] Dual numeric path research (f32 + i32 Q16)
- [x] `no_std` strategy (`no-std-compat`)
- [x] Math-shape alignment notes
- [x] Outside-in API preview + harsh reviews (`docs/api-preview/`)
- [x] Revised surface lock: **D-id-space**, **D-port-schema**, **D-hostpath**
- [x] IP-safe public language checklist (no Nintendo / GBG product claims in shipped text)

---

## Phase 1 — Workspace scaffold

- [x] Cargo workspace: `wyrd-core`, `wyrd-graph`, `wyrd-runtime`, `wyrd-bevy`
- [x] Core free of engine deps (Bevy only in `wyrd-bevy`)
- [x] MIT license + root README (safe public wording)
- [x] CI: dual `signal-f32` / `signal-i32` matrix
- [x] CI: Bevy f32-only job
- [x] CI: `no_std` + `signal-i32` check on core
- [x] CI: coverage jobs with **fail-under-lines 100** (f32, i32, serde-ron, bevy)
- [x] Local `./scripts/dual-check.sh`
- [x] Local `./scripts/coverage.sh` (+ `--i32` / `--bevy` / `--all`)
- [ ] Public crate docs on docs.rs (publish path)
- [ ] Versioned changelog / release notes

---

## Phase 2 — Core loom

### Domain model

- [x] Editable `WeaveDef` (string knot ids + catalog port names) converts to an immutable validated `Weave`
- [x] `KnotDef` / `ThreadDef` / `PortRefDef`
- [x] Closed `enum KnotKind` dispatch (**D-dispatch**)
- [x] Closed port tables per kind (**D-port-schema**)
- [x] Dense runtime ids: internal `KnotId` / `PortSlot` plus owner-aware `SenseId`, `HostPathId`, and `CmdId` handles (**D-id-space**)
- [x] Scalar `Signal` monomorphized f32 **or** i32 Q16 (**D-numeric-dual**)
- [x] Truthy = non-zero
- [x] Custom vec DAG storage (no petgraph required)

### Validate

- [x] Empty weave rejected
- [x] Unique knot ids
- [x] Known ports only (no folklore port names)
- [x] Direction Out → In
- [x] Fan-in ≤ 1 per input port
- [x] Required inputs connected (Compare `rhs` relaxed when `rhs_const` set)
- [x] DAG only; cycles fail
- [x] Numeric path matches compiled feature
- [x] Hard budgets: max knots / max threads (`Budget`)
- [x] Soft budgets (warn via validate_report; soft fields on Budget)
- [x] Chain-depth budget (soft/hard from vision table)
- [x] Fan-out budget (soft/hard)
- [x] Delay path-sum budget
- [ ] Pattern nest-depth budget (expand-at-load depth)
- [x] Soft BudgetWarning Display with knot ids (`validate_report`); multi-error still open
- [ ] Unsupported And/Or arity policy beyond 1..=4 (today empty table → reject)

### Bind → sample → loom → outbox

- [x] `Runtime::bind` consumes validated `Weave`; `Runtime` is the sole executable artifact
- [x] Intern path / cmd strings → dense ids
- [x] Topo order at bind
- [x] Precomputed inbound edges + input slot lists
- [x] `begin_frame` clears outbox
- [x] Host sample via checked dense `PortWriter::set_sense(SenseId, Signal)`
- [x] One settle pass per tick (scheduled topo)
- [x] Outbox: `SignalOutSample` + `Emit { cmd, payload }`
- [x] Path/cmd name reverse lookup
- [x] Steady-state settle avoids topology alloc (bind-time buffers)
- [x] Buffer stability at bind (outbox capacity / delay_buf); full counting allocator still open
- [ ] Second settle pass opt-in (documented hybrid) — *(later if ever)*

### Serde

- [x] Optional `serde` derives on author types
- [x] `serde-ron`: `from_ron` / `to_ron` + validate on load
- [x] Reject numeric path mismatch at load
- [x] JSON codec (`serde-json`: from_json / to_json; same gates as RON)
- [ ] Optional bincode / binary asset path — *(later)*

---

## Phase 3 — Catalog v0 (engine-neutral)

### Sense

- [x] **Constant**
- [x] **SignalIn** (LevelIn / PulseIn are pedagogy + host write policy)
- [x] **OnStart** (ONE first loom only)
- [ ] Distinct PulseIn helper / docs example that auto-zeros after sample (host recipe)

### Runes — logic & edge

- [x] **Not**
- [x] **And** (arity tables 1..=4)
- [x] **Or** (arity tables 1..=4)
- [x] **Compare** (Eq/Ne/Lt/Lte/Gt/Gte; wired rhs or `rhs_const`)
- [x] **RisingFromZero**
- [x] **FallingToZero**
- [x] **Change** (either-edge truthiness pulse)
- [x] **Xor**
- [x] **Select** (sel → a/b multiplex)
- [x] **Threshold** (optional hysteresis; crossed_up / crossed_down)

### Runes — math

- [x] **Calc** Add / Sub / Mul / Div (div0 → 0)
- [x] **Map** (linear remap; zero span → out_min)
- [x] **Abs**
- [x] **Neg**
- [x] **Digitize** / quantize (steps bins; steps=0 → InvalidParam)
- [x] **Sqrt** (f32 libm; i32 isqrt; non-positive → 0)
- [x] **Clamp** (min/max; min > max → InvalidParam)

### Runes — state

- [x] **Flag** (set / reset / optional toggle; ResetWins + SetWins)
- [x] **Counter** (rising-edge inc/dec; reset)
- [x] **Timer** FedCountdown + PulseHold (tick integers)
- [x] **Delay** Rune (ring buffer; no Thread delay metadata yet)
- [x] **Random** (host Seed; optional gate; min/max ports; reseed)

### Act

- [x] **SignalOut** (LevelOut / PulseOut by host policy)
- [x] **EmitCommand** (rising-edge trigger → outbox)
- [x] Emit **enable** port (unconnected = enabled)
- [x] Emit-per-tick sandbox cap (`BindOpts.max_emits_per_tick`)
- [ ] PulseOut as first-class Act variant (or documented host recipe only)

### Catalog intentionally not v0

- [ ] **Vec2** / multi-axis PortType — *(later; axes = separate SignalIns)*
- [ ] Wormhole / global bus Knots — *(later; discouraged)*
- [ ] Thread-level invert / delay metadata — *(later; Delay Rune only in v0)*
- [ ] Free-running **Clock** Knot — *(out of default core forever unless host owns time)*
- [ ] Multi-wire fan-in policies (And/Or/Replace on one port) — *(later)*
- [ ] Runtime-nested Patterns — *(later; expand-at-load only)*
- [ ] Multi-settle per tick — *(later / opt-in)*

---

## Phase 4 — Bevy MVP (`wyrd-bevy`)

- [x] Thin adapter crate; Bevy **not** a core dependency
- [x] Bevy 0.18 pin; `default-features = false` style deps
- [x] **signal-f32 only** (i32 not via Bevy adapter)
- [x] `WyrdPlugin` + `WyrdSet::{Sample, Loom, Apply}` chain
- [x] `WyrdWorld` / `WyrdInstance` (private label + bound Runtime + tick; no retained Weave)
- [x] Dense `sense_id` / `path_id` helpers
- [x] Headless and-door test (sample → loom → apply)
- [x] `and_door` example binary
- [x] Headless component slice (scripted plates → Door); windowed input still open
- [x] Apply systems that mutate real Bevy components (Door)
- [x] Host **Messages as confirmations only** (`WyrdSignalConfirm`) documented + demoed
- [ ] Feature-gated host kit (optional small demos: sound/transform) without polluting core
- [ ] Asset pipeline: load Weave RON as Bevy asset — *(near-term or later)*
- [ ] `bevy_log` loom-failure path exercised in CI when enabled

---

## Phase 5 — Perf pass

- [x] Divan settle suite (**split**): `settle_chain`, `settle_catalog`, `settle_stateful`, `bind`
- [x] Shared builders in `benches/common.rs` (`autobenches = false`)
- [x] Parameterized N ∈ {16, 64, 128} with items/sec counters
- [x] Separate benches: bind (topo/validate) vs settle families vs host tick
- [x] Catalog + delay + Random + fan-out settle benches
- [x] **P0–P3** measurement coverage: scaled chains, stateful/emit, edges completeness, pattern + Bevy
- [x] Full host-tick bench (`tick_once_not_chain` + Bevy `host_tick`)
- [x] `docs/perf.md` — how to bench + f32/i32 tables + completeness matrix + flamegraph how-to
- [x] Expected hot-function checklist in `docs/perf.md` (SVG capture still optional)
- [x] Steady-state buffer stability proven (capacity/delay_buf); global alloc hook open
- [x] Measured settle + bind numbers in `docs/perf.md` (local host)
- [ ] Parallel settle — *(later; default single-threaded for determinism)*
- [x] KindTag-at-bind + CSR inbound + flat clear + hot ports (settle structural pass)
- [x] Isolation benches (`settle_iso`) + long Divan decision runs documented
- ~~Optional Criterion / bench CI~~ — **not planned** (local Divan only)

---

## Phase 6 — Patterns as product

- [x] Editable `PatternDef` + immutable validated `Pattern` with named input/output exports
- [x] Builder `include` expands Patterns at authoring time
- [x] Typed `PatternInstance::input` / `output` endpoints connect through `WeaveBuilder::connect`
- [x] Declarative `weave!` supports knots, aliases, Patterns, and typed endpoint combinations
- [x] Prefixed instance ids (`instance/knot`)
- [x] Validate after parent wires exports
- [x] **Pattern cookbook** shipped as CI tests (`patterns_cookbook`)
- [x] Cookbook as `tests/patterns_cookbook.rs` (CI)
- [x] Public README section: “First five Weaves”
- [ ] Nested Pattern stamps pre-flatten tooling (author helper) — *(later if needed)*
- [ ] Runtime-nested Patterns — *(later; locked out of v0)*

---

## Host abstraction (vision “generic” crate)

Vision target: `wyrd-generic` — Host trait + headless / null host. **Not started as a crate.**

- [x] `trait Host`: `sample_into` / `apply` / `time`
- [x] Typed **HostCommand** (SetLevel / Emit) + outbox map helpers
- [x] Null / headless host for unit tests
- [x] Scripted input sequences + deterministic replay tests
- [x] Command apply phase clearly separated from loom
- [ ] Host-registered extension commands (camera, sound, …) stay **outside** core
- [x] Host surface in `wyrd-runtime` (no separate crate yet)

---

## Budgets & sandbox (vision tables)

Soft / hard defaults from vision (library may raise/lower via config):

| Budget | Soft | Hard | Shipped? |
| --- | ---: | ---: | --- |
| Knots per Weave | 64 | 256 | Hard only (default 256) |
| Threads per Weave | 128 | 512 | Hard only (default 512) |
| Chain depth | 8 | 16 | [x] |
| Fan-in / fan-out | 4 | 8 | Fan-in=1 hard; fan-out hard+soft [x] |
| Delay ticks (Rune) | — | ring sized | Partial (per-Delay) |
| Delay path sum | — | 32 | [x] |
| Pattern nest depth | 2 | 4 | [ ] (expand one level only today) |
| Emits per tick | soft pedagogy | BindOpts default 8 | [x] |

- [x] Hard max knots / threads
- [x] Soft vs hard policy + `BindOpts.budget` override
- [x] Author-facing soft report via `validate_report`; hard failures use contextual `ValidationError` / `BindError`

---

## Pedagogy & public language

- [x] Metaphor vocabulary in README (Wyrd, Weave, Knot, Thread, Pattern, loom)
- [x] No Nintendo / GBG product names in shipped crate text
- [x] Public README host tick + first five Weaves (deep vision docs still gitignored)
- [ ] Promote a short **public** tutorial out of gitignored `docs/` (or un-ignore a curated subset)
- [ ] Glossary: Pulse vs Level as **host policy**, not PortTypes
- [ ] Sense / Rune / Act layering explained with one diagram
- [x] “Door is a host effect” demo wording (core never owns Door Knots)

---

## Tooling & quality

- [x] Dual-path tests (f32 + i32)
- [x] Line coverage 100% gate on core suites + bevy + serde-ron
- [x] Integration tests: hello, and-door, timers, counter/flag/emit, delay, compare/calc, loom arms
- [ ] Property / fuzz tests on validate or signal ops — *(later)*
- [ ] Miri / sanitizer CI optional — *(later)*
- [ ] Playdate or other `no_std` **target** CI (not only `check`) — *(later)*

---

## Phase 7 — Loom editor *(later)*

- [ ] `wyrd-editor` crate (no engine required in core)
- [ ] Node graph UI (egui_node_graph / egui-snarl class — **editing only**, semantics stay in Wyrd)
- [ ] Load/save Weave (RON)
- [ ] Live validate + budget feedback
- [ ] Dense-id debug overlay optional
- [ ] Pattern stamp UI
- [ ] No claim of visual-scripting as the **library’s** first product surface until runtime is the product

---

## Other host adapters *(later)*

- [ ] `wyrd-godot`
- [ ] `wyrd-macroquad`
- [ ] Sea of Grass (or any game) depends on Wyrd for wiring without door types in core
- [ ] Shared adapter cookbook (“sample → loom → apply” checklist)

---

## Explicitly out of scope (do not build as core)

Keep unchecked forever unless the project re-opens decisions:

- [ ] Nintendo IP, UI, tutorials, mascots, “GBG-compatible” claims
- [ ] Full redstone / dust / free clocks as core features
- [ ] Player-facing visual scripting as the **crate’s** primary product (editor is optional phase 7)
- [ ] Core knowledge of doors, portals, moisture, FOV, physics impulses as Knots
- [ ] Silent global buses as default wiring
- [ ] Random-as-truth without host-owned seed

---

## MVP vertical slices (from vision)

Acceptance for “MVP done” in vision: slices **1–6 green** + flamegraph notes + safe README.

| # | Slice | Status |
| --- | --- | --- |
| 1 | Headless truth: Constant → Not → SignalOut | [x] |
| 2 | Pulse path: edge → Counter → Compare → out | [x] (via RisingFromZero / Counter tests) |
| 3 | AND fan-in: two SignalIn → And → SignalOut | [x] |
| 4 | Serde Weave RON + budget validate + topo | [x] (hard + soft report) |
| 5 | Bevy demo: input → Weave → component/message | [x] headless component + Message confirm |
| 6 | Divan settle N knots + items/sec | [x] N∈{16,64,128} + tick_once |
| 7 | Flamegraph documented | [~] how-to in docs/perf.md; capture optional |

**MVP not closed** until playable Bevy slice, fuller perf package, and soft-budget/diagnostics story land.

---

## Suggested near-term order (product, not coverage)

Work these in order when prioritizing **vision** over infra:

1. [x] **Host trait + headless/scripted host** (evaluation model as product)
2. [x] **One beautiful Bevy vertical slice** (component + message confirmation; headless)
3. [x] **Budgets + author diagnostics** (soft/hard, depth, clear errors)
4. [x] **Perf package** (numbers + flamegraph recipe + buffer stability)
5. [x] **Pattern cookbook** + short public tutorial path
6. [ ] Only then **Loom editor** / other engines / Random / Digitize

---

## Crate layout target vs today

```text
wyrd-core       [x] values, ids, ports, KnotKind, errors
wyrd-graph      [x] Weave, validate, builder, Pattern expand, RON
wyrd-runtime    [x] bind, loom, outbox, settle bench
wyrd-generic    [x] Host in runtime (Null/Scripted) ← done as module
wyrd-bevy       [x] thin adapter (demo polish open)
wyrd-editor     [ ] Loom UI                         ← later
wyrd-godot      [ ]                                 ← later
wyrd-macroquad  [ ]                                 ← later
```

---

## Related docs

| Path | Role |
| --- | --- |
| [`vision.md`](./vision.md) | Product pillars, architecture, success signals |
| [`research/decisions.md`](./research/decisions.md) | Locked D-* / deferred Q-* |
| [`primitives/`](./primitives/) | Port tables, knot semantics |
| [`api-preview/11_revised_surface.rs`](./api-preview/11_revised_surface.rs) | v2 API pencil (historical previews 01–10) |
| Root `README.md` | Shipped public surface only |

---

## How to update this file

1. Prefer checking boxes when **tests or demos** prove the item.
2. When locking a new decision, update `research/decisions.md` first, then reflect it here.
3. Do not check “later” items to force a ship; move them up only after a decision log entry.
4. Coverage/CI work supports the roadmap; it is not a substitute for Host, Bevy product slice, budgets, or pedagogy.
