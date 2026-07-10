# Wyrd performance notes

**Machine:** local developer host (see date below). Numbers are **indicative**, not a guarantee.  
**Measured:** 2026-07-10 · Divan release · `cargo bench -p wyrd-runtime`

## How to bench

Bench targets are **split** so the suite can grow without one mega-file. Shared Weave builders live in `crates/wyrd-runtime/benches/common.rs` (not a stand-alone target; `autobenches = false`).

| Target | Path | Focus |
| --- | --- | --- |
| `settle_chain` | `benches/settle_chain.rs` | Not depth, And door, host `tick_once` |
| `settle_catalog` | `benches/settle_catalog.rs` | Micro + scaled Map/Digitize/Mul/Div/Sqrt + edges/logic packs + Compare/Clamp chains + OnStart |
| `settle_stateful` | `benches/settle_stateful.rs` | Delay, Random, **stateful kit**, **emit storm** |
| `settle_iso` | `benches/settle_iso.rs` | Isolation: Not structural, fan-out, Digitize/Sqrt/Map eval |
| `bind` | `benches/bind.rs` | Load path: validate + topo + buffers + **pattern expand/include** |
| `host_tick` (`wyrd-bevy`) | `crates/wyrd-bevy/benches/host_tick.rs` | Headless Bevy door update (f32 only) |

```bash
# All targets (f32 default)
cargo bench -p wyrd-runtime

# One family
cargo bench -p wyrd-runtime --bench settle_chain
cargo bench -p wyrd-runtime --bench settle_catalog
cargo bench -p wyrd-runtime --bench settle_stateful
cargo bench -p wyrd-runtime --bench bind

# Integer Signal path
cargo bench -p wyrd-runtime --no-default-features --features "std,signal-i32"
```

Divan prints fastest/median/mean and items/s (knots settled or bound per second when `ItemsCount` is set).

Deep Not-chains exceed the default hard `max_chain_depth` (16). Chain builders raise depth via `BindOpts.budget` (`benches/common.rs`).

## Steady-state settle — f32 (measured)

| Bench | Size / notes | Median | ~items/s (knots) |
| --- | --- | ---: | ---: |
| `settle_and_door` | 4 knots | ~27 ns | ~146 M |
| `settle_not_chain` | 16 Nots | ~98 ns | ~183 M |
| `settle_not_chain` | 64 Nots | ~343 ns | ~192 M |
| `settle_not_chain` | 128 Nots | ~703 ns | ~185 M |
| `tick_once_not_chain` | 16 Nots, NullHost | ~97 ns | ~185 M |
| `tick_once_not_chain` | 64 Nots | ~351 ns | ~188 M |
| `settle_map_digitize` | 4 knots | ~28 ns | ~142 M |
| `settle_calc_abs` | 5 knots | ~33 ns | ~152 M |
| `settle_threshold` | 3 knots | ~14 ns | ~211 M |
| `settle_fanout_nots` | 8 Not+Out (~17 knots) | ~84 ns | ~202 M |
| `settle_fanout_nots` | 32 (~65 knots) | ~359 ns | ~181 M |
| `settle_delay` | ticks 1 / 8 / 32 (3 knots) | ~14–15 ns | ~210 M |
| `settle_random_gated` | 3 knots; **2 looms/sample** (fall+rise) | ~32 ns | ~93 M |

At **60 FPS** (~16.7 ms/frame), a 64-knot Not chain at ~0.35 µs is **far under 0.01%** of the frame.

**Reading delay:** median barely moves with ring length 1→32 — fixed loom overhead (clear/seed/topo) dominates ring traffic at human-scale delay depths. Use larger delay + more knots if you need to stress the ring path.

**Reading random:** each sample runs two settles (gate low then high) so raw items/s understates per-settle cost vs other rows.

## Steady-state settle — i32 Q16 (measured)

| Bench | Size / notes | Median | ~items/s (knots) |
| --- | --- | ---: | ---: |
| `settle_and_door` | 4 knots | ~25 ns | ~159 M |
| `settle_not_chain` | 16 Nots | ~95 ns | ~189 M |
| `settle_not_chain` | 64 Nots | ~341 ns | ~194 M |
| `settle_not_chain` | 128 Nots | ~687 ns | ~189 M |
| `tick_once_not_chain` | 16 / 64 | ~95 / ~338 ns | ~190 / ~195 M |
| `settle_map_digitize` | 4 knots | ~21 ns | ~193 M |
| `settle_calc_abs` | 5 knots | ~32 ns | ~156 M |
| `settle_threshold` | 3 knots | ~14 ns | ~214 M |
| `settle_fanout_nots` | 8 / 32 | ~82 / ~325 ns | ~208 / ~200 M |
| `settle_delay` | ticks 1–32 | ~14 ns | ~211 M |
| `settle_random_gated` | 2 looms/sample | ~32 ns | ~94 M |

On this host, **i32 is not slower** than f32 for Not/And/delay/threshold. Map/Digitize is slightly **faster** under i32 (integer path vs float). Re-measure after Calc-heavy or Sqrt-heavy benches if those land.

## Bind / load path (not per-frame)

| Bench | Size | Median (f32) | Median (i32) | ~knots bound/s |
| --- | --- | ---: | ---: | ---: |
| `bind_small` | ~4 knots | ~1.3 µs | ~1.3 µs | ~3.1 M |
| `bind_not_chain` | 16 Nots | ~7.5 µs | ~7.5 µs | ~2.4 M |
| `bind_not_chain` | 64 Nots | ~29 µs | ~31 µs | ~2.2 M |
| `bind_not_chain` | 128 Nots | ~58 µs | ~58 µs | ~2.2 M |

Bind is **~100×** a single settle of the same graph — fine for room load / asset bind; do not put re-bind on the hot tick.

## Zero-alloc / preallocation

- Topology (adj, topo order, inbound) is built at **bind**.
- Delay rings are sized at bind (`delay_buf`).
- Outbox vectors are **reserved** from act counts at bind; `begin_frame` clears length, not capacity.

CI checks:

```bash
cargo test -p wyrd-runtime --test zero_alloc_loom
```

These prove steady-state **outbox length stability** and delay bind sizing — not a full global allocator audit under `no_std`.

## Flamegraph (optional)

```bash
# Requires cargo-flamegraph + frame pointers recommended for accuracy
CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph -p wyrd-runtime --bench settle_chain
```

Expect hot frames inside `Runtime::loom` / `gather_inputs` / `eval_knot` match arms, not bind/validate.

### Hot functions (expected; capture optional)

Until a committed flamegraph SVG lives under `docs/` or CI artifacts, use this checklist when sampling:

| Likely hot | Role |
| --- | --- |
| `Runtime::loom` | Flat clear → seed senses → topo loop |
| `Runtime::gather_inputs` | CSR inbound walk + hot port copy |
| `Runtime::eval_knot` | Match on **bind-time** `kind_tags[ki]` (no per-tick `from_kind`) |
| `get_port_hot` / `set_port_hot` | Dense port store (debug_assert in-range) |

### Structural settle pass (post P0–P3)

**Learning that drove the work:** scaled Digitize/Sqrt rows were ~2× slower than Not; short microbenches were too noisy — use **scaled N** + longer Divan weight.

**Shipped changes**

1. **KindTag cache** at bind (`kind_tags: Vec<KindTag>`) — zero `from_kind` per tick  
2. **CSR inbound** (`inbound_off` + `inbound_edges`) — flatter gather  
3. **Flat clear list** (`clear_port_idx`) — no per-knot input_slots walk each loom  
4. **Hot port access** on eval path; safe checked APIs retained for OOB tests  
5. **Skip Sense** in topo (already seeded)  
6. **Digitize** early-exit for `steps<=1` + tighter bin cast  

**Measurement settings (decision runs):**  
`cargo bench -p wyrd-runtime --bench settle_iso -- --sample-count 300 --min-time 1`  
(same weight on catalog filters). Two consecutive f32 after-runs agreed on go.

| Bench (f32) | Before (P0 short) | After structural (long) | Notes |
| --- | ---: | ---: | --- |
| `settle_not_chain` 64 | ~343 ns | ~317–338 ns | structural |
| `settle_digitize_chain` 64 | ~1.16 µs | **~1.10 µs** | + digitise path |
| `settle_sqrt_chain` 64 | ~1.26 µs | **~1.08 µs** | mostly structural |
| `settle_map_chain` 64 | ~656 ns | ~651 ns | small |

| Bench (i32) | Before | After structural (long) |
| --- | ---: | ---: |
| `settle_digitize_chain` 64 | ~869 ns | **~396 ns** |
| `settle_sqrt_chain` 64 | ~562 ns | ~562 ns (flat) |
| `settle_not_chain` 64 | ~341 ns | ~346 ns (flat) |

### Isolation sub-benches (`settle_iso`)

| Name | Isolates |
| --- | --- |
| `iso_struct_not_chain` | Structural clear+gather+Not (N=64/128) |
| `iso_gather_fanout` | Wide fan-out / outbox (N=32/64) |
| `iso_eval_digitize_chain` | Digitize eval stack (N=64) |
| `iso_eval_sqrt_chain` | Sqrt eval stack (N=64) |
| `iso_eval_map_chain` | Lighter eval control (N=64) |

```bash
cargo bench -p wyrd-runtime --bench settle_iso -- --sample-count 300 --min-time 1
```

Local Divan only — **not** run in CI.

### Arm math + residual structure (area-by-area)

Decision weight: `--sample-count 300 --min-time 1` on `settle_iso` (and catalog filters).  
Each area: isolation baseline → change → after×2 (f32) + i32 check when dual-path differs.

| Area | Change | Iso f32 before → after (median, N=64) |
| --- | --- | ---: |
| **1 Digitize** | Bind precompute: scales / i32 `den`+`out_span` | **~1.093 µs → ~854 ns** |
| **1b Digitize** | f32 `bin_scale=steps/span` + `last_f` clamp + `mul_add` (integer bins) | **~854 ns → ~646–693 ns** |
| **2 Sqrt** | f32: core `f32::sqrt` (drop `libm`); i32: Newton `isqrt` | **~1.08 µs → ~395–437 ns** (i32 ~562 → ~310 ns) |
| **3 Map** | Bind precompute: reciprocal × + i32 `den`/`out_span_i64` | **~651 ns → ~505–515 ns** |
| **4 Residual** | Sense seed list; Calc tags split by op; Compare `rhs` as `Signal`; Emit/Random wire flags at bind | Not 64 **~307–310 → ~268 ns** (r1/r2 agree) |

**Area 4 residual evidence (long Divan `iso_struct_not_chain`):**

| Capture | Not64 median | Notes |
| --- | ---: | --- |
| before (cite `bench-area-digitize-before` / `bench-area-map-before`) | ~307 / ~309.6 ns | pre sense_seeds |
| after r1 (`bench-area-residual-after-r1`) | **~268 ns** | `--sample-count 300 --min-time 1` |
| after r2 (`bench-area-residual-after-r2`) | **~267.9 ns** | agrees with r1 |
| map control (`bench-area-residual-control-map`) | ~502 ns | no Map regression |

**Final isolation medians (f32, long):** Digitize **~646–693 ns** · Map ~502–505 ns · Sqrt ~395 ns · Not64 **~268 ns** · Fanout64 ~572 ns.

**Final isolation (i32, long):** Map ~286 ns · Sqrt ~291 ns · Not64 ~284 ns · Digitize still noisy on iso (prefer catalog / fastest).

**Why stop here:** Digitize f32 still above Map/Not (bin cast + clamp) but ~1.6× closer than pre-area1. Remaining arms (Calc Div i32 Q-div) are live arithmetic; further wins need algorithmic/host batching.

## Scaled catalog / delay (P0 — amortized)

These chains use **N copies** of the interesting arm so fixed loom tax is amortized. Compare ns/knot to `settle_not_chain` of similar size.

### f32 (post arm-math pass)

| Bench | N (arm copies) | Total knots ≈ | Median | ~items/s (**all** knots) |
| --- | ---: | ---: | ---: | ---: |
| `settle_map_chain` | 16 | 18 | ~131 ns | ~138 M |
| `settle_map_chain` | 64 | 66 | **~578 ns** | ~114 M |
| `settle_digitize_chain` | 16 | 18 | ~205 ns | ~88 M |
| `settle_digitize_chain` | 64 | 66 | **~650–700 ns** (iso) | ~95–100 M |
| `settle_calc_mul_chain` | 16 | 19 (+const ONE) | ~252 ns | ~75 M |
| `settle_calc_mul_chain` | 64 | 67 | ~547 ns | ~123 M |
| `settle_sqrt_chain` | 16 | 18 | ~105 ns | ~172 M |
| `settle_sqrt_chain` | 64 | 66 | **~448 ns** | ~147 M |
| `settle_delay_chain` (ticks=4) | 8 | 10 | ~55 ns | ~183 M |
| `settle_delay_chain` (ticks=4) | 32 | 34 | ~209 ns | ~163 M |

### i32 Q16

| Bench | N | Median | ~items/s |
| --- | ---: | ---: | ---: |
| `settle_map_chain` | 16 / 64 | ~86 / ~552 ns | ~210 / ~120 M |
| `settle_digitize_chain` | 16 / 64 | ~86 / ~396 ns | ~210 / ~167 M |
| `settle_calc_mul_chain` | 16 / 64 | ~242 / ~561 ns | ~79 / ~119 M |
| `settle_sqrt_chain` | 16 / 64 | ~205 / **~310 ns** (iso) | ~88 / ~213 M |
| `settle_delay_chain` | 8 / 32 | ~54 / ~203 ns | ~186 / ~168 M |

**Notes**

- **Digitize** is still the heaviest common pure-eval arm on f32; Map/Sqrt now sit near Mul/Not after precompute + core sqrt.
- **Sqrt f32** no longer depends on `libm` (desktop IEEE `f32::sqrt`).
- **Calc Mul** uses level `ONE` on the `b` port so i32 Q-mul stays non-zero (`ONE*ONE=ONE`). Whole-count mul would collapse to 0 on i32. Total knots = N muls + in + const + out.
- **Delay chain** scales with **number of Delay knots**, unlike the single-delay microbench (often flat).
- Digitize/Map/Sqrt/Mul benches feed **level `ONE`** (not `from_count(1)`) so f32 and i32 sit at the same end of ZERO..ONE.

Filter runs:

```bash
cargo bench -p wyrd-runtime --bench settle_catalog -- \
  settle_map_chain settle_digitize_chain settle_calc_mul_chain settle_sqrt_chain
cargo bench -p wyrd-runtime --bench settle_stateful -- settle_delay_chain
```

## P1 — Stateful kit + emit storm + Calc Div

### f32

| Bench | Notes | Median | ~items/s (knots) |
| --- | --- | ---: | ---: |
| `settle_stateful_kit` | 11 knots; 4-phase start/feed script | ~76 ns | ~145 M |
| `settle_emit_storm` | 8 emits (+gate); **2 looms**/sample | ~113 ns | ~80 M |
| `settle_emit_storm` | 32 emits; **2 looms**/sample | ~465 ns | ~71 M |
| `settle_calc_div_chain` | N=16 Div (ONE divisor) | ~139 ns | ~137 M |
| `settle_calc_div_chain` | N=64 | ~536 ns | ~125 M |

### i32 Q16

| Bench | Median | ~items/s |
| --- | ---: | ---: |
| `settle_stateful_kit` | ~69 ns | ~159 M |
| `settle_emit_storm` 8 / 32 (2 looms) | ~114 / ~461 ns | ~79 / ~72 M |
| `settle_calc_div_chain` 16 / 64 | ~259 / ~1.11 µs | ~73 / ~60 M |

**Notes**

- **Stateful kit** one loom/sample; phase cycles idle → start → start+feed → feed so Rising/Counter/Flag/Timers exercise over iterations.
- **Emit storm** ItemsCount = knots (gate + n EmitCommands), not emit count; **two looms per sample** (low then high) so every sample forces a rising edge and n emits.
- **Calc Div** on i32 is slower than f32 at N=64 here (Q-div vs float `/`).

```bash
cargo bench -p wyrd-runtime --bench settle_stateful -- settle_stateful_kit settle_emit_storm
cargo bench -p wyrd-runtime --bench settle_catalog -- settle_calc_div_chain
```

## P2 — Edges + remaining catalog kinds

### f32

| Bench | Median | ~items/s |
| --- | ---: | ---: |
| `settle_edges_pack` | ~34 ns | ~203 M |
| `settle_logic_pack` | ~82 ns | ~134 M |
| `settle_clamp_neg_chain` 16 / 64 | ~192 / **~771 ns** | ~177 / ~169 M |
| `settle_compare_chain` 16 / 64 | ~135 / **~573 ns** | ~133 / ~115 M |
| `settle_onstart` | ~8.8 ns | ~227 M |

### i32

| Bench | Median | ~items/s |
| --- | ---: | ---: |
| `settle_edges_pack` | ~38 ns | ~182 M |
| `settle_logic_pack` | ~90 ns | ~123 M |
| `settle_clamp_neg_chain` 16 / 64 | ~196 / ~734 ns | ~173 / ~177 M |
| `settle_compare_chain` 16 / 64 | ~130 / ~442 ns | ~139 / ~149 M |
| `settle_onstart` | ~9.6 ns | ~209 M |

### Catalog completeness (`KnotKind` → bench)

| Kind | Bench graph(s) |
| --- | --- |
| Constant | many (mul/div/logic) |
| SignalIn | most settle benches |
| OnStart | `settle_onstart` |
| Not | `settle_not_chain`, fanout |
| And | `settle_and_door` |
| Or | `settle_logic_pack` |
| Compare | `settle_compare_chain` |
| RisingFromZero | `settle_edges_pack`, stateful kit |
| FallingToZero | `settle_edges_pack` |
| Change | `settle_edges_pack` |
| Flag | `settle_stateful_kit` |
| Counter | `settle_stateful_kit` |
| Timer (PulseHold + FedCountdown) | `settle_stateful_kit` |
| Delay | `settle_delay`, `settle_delay_chain` |
| Calc Add | `settle_calc_abs` |
| Calc Mul / Div | scaled chains |
| Calc Sub | **skip** (shared Calc dispatch; distinct `sat_sub` not separately timed) |
| Map | micro + `settle_map_chain` |
| Abs / Neg | calc_abs / clamp_neg chain |
| Select / Xor | `settle_logic_pack` |
| Digitize | micro + chain |
| Threshold | `settle_threshold` |
| Random | `settle_random_gated` |
| Sqrt | `settle_sqrt_chain` |
| Clamp | `settle_clamp_neg_chain` |
| SignalOut | most settle graphs |
| EmitCommand | `settle_emit_storm` |

```bash
cargo bench -p wyrd-runtime --bench settle_catalog -- \
  settle_edges_pack settle_logic_pack settle_clamp_neg_chain \
  settle_compare_chain settle_onstart
```

## P3 — Pattern load + Bevy host tick

### Pattern expand / bind (f32; i32 similar on pure graph)

| Bench | What | Median | Notes |
| --- | --- | ---: | --- |
| `expand_pattern_monostable` | expand only | ~536 ns | 2 inner knots |
| `bind_after_pattern_include` | bind pre-built include weave | ~1.5 µs | ~4 knots |
| `include_build_bind_monostable` | include+build+bind each sample | ~2.9 µs | authoring reload |

```bash
cargo bench -p wyrd-runtime --bench bind -- expand_pattern_monostable bind_after_pattern_include
```

### Bevy headless (f32 only)

| Bench | Median | Notes |
| --- | ---: | --- |
| `bevy_door_tick_both` | ~1.33 µs | Sample → Loom → Apply (WyrdSet); both plates high |
| `bevy_door_tick_scripted` | ~1.32 µs | 4-phase plate script |

Roughly **~50×** a raw `settle_and_door` loom (~27 ns) — Bevy schedule + resource/query overhead, not loom math.

```bash
cargo bench -p wyrd-bevy --bench host_tick
```

Benches are **local-only** (not run in CI). Use the commands above when measuring.

## Adding a new bench

1. Add a builder in `benches/common.rs` if the Weave is reusable.
2. Prefer a **new** `benches/<family>.rs` (or extend an existing family) over a single kitchen-sink file.
3. Register `[[bench]]` in `crates/wyrd-runtime/Cargo.toml` (`path` + `harness = false`).
4. Re-measure f32 **and** i32; paste medians into this file.

## Next measurement / opt candidates

Structural KindTag/CSR/clear landed (see above). Further: Digitize/Sqrt **math-only** kernels if still hot in flamegraph; Bevy host path is a separate budget (~50× loom).
