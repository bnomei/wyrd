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
| `bind` | `benches/bind.rs` | Load path: validate + topo + buffers |

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
| `Runtime::loom` | Clear inputs → seed senses → topo loop |
| `Runtime::gather_inputs` | Inbound edge walk + port copy |
| `Runtime::eval_knot` / `KindTag::from_kind` | Per-knot dispatch (from_kind every tick) |
| `Runtime::get_port` / `set_port` | Dense port store access |

Re-run flamegraph after KindTag caching or clear/gather fusion and update this table with real symbols + % time.

## Scaled catalog / delay (P0 — amortized)

These chains use **N copies** of the interesting arm so fixed loom tax is amortized. Compare ns/knot to `settle_not_chain` of similar size.

### f32

| Bench | N (arm copies) | Total knots ≈ | Median | ~items/s (**all** knots) |
| --- | ---: | ---: | ---: | ---: |
| `settle_map_chain` | 16 | 18 | ~196 ns | ~92 M |
| `settle_map_chain` | 64 | 66 | ~656 ns | ~101 M |
| `settle_digitize_chain` | 16 | 18 | ~317 ns | ~57 M |
| `settle_digitize_chain` | 64 | 66 | ~1.16 µs | ~57 M |
| `settle_calc_mul_chain` | 16 | 19 (+const ONE) | ~192 ns | ~99 M |
| `settle_calc_mul_chain` | 64 | 67 | ~541 ns | ~124 M |
| `settle_sqrt_chain` | 16 | 18 | ~284 ns | ~63 M |
| `settle_sqrt_chain` | 64 | 66 | ~1.26 µs | ~52 M |
| `settle_delay_chain` (ticks=4) | 8 | 10 | ~55 ns | ~183 M |
| `settle_delay_chain` (ticks=4) | 32 | 34 | ~209 ns | ~163 M |

### i32 Q16

| Bench | N | Median | ~items/s |
| --- | ---: | ---: | ---: |
| `settle_map_chain` | 16 / 64 | ~109 / ~643 ns | ~165 / ~103 M |
| `settle_digitize_chain` | 16 / 64 | ~216 / ~869 ns | ~83 / ~76 M |
| `settle_calc_mul_chain` | 16 / 64 | ~242 / ~561 ns | ~79 / ~119 M |
| `settle_sqrt_chain` | 16 / 64 | ~238 / ~562 ns | ~76 / ~117 M |
| `settle_delay_chain` | 8 / 32 | ~54 / ~203 ns | ~186 / ~168 M |

**Notes**

- **Digitize** and **Sqrt (f32 libm)** are clearly heavier than Map/Not on this host (~half the knot/s of Not chains).
- **Sqrt i32** (integer isqrt) is faster than f32 at N=64 here — dual-path cost is not uniform.
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
| `settle_clamp_neg_chain` 16 / 64 | ~207 / ~838 ns | ~165 / ~155 M |
| `settle_compare_chain` 16 / 64 | ~155 / ~614 ns | ~116 / ~107 M |
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

## Adding a new bench

1. Add a builder in `benches/common.rs` if the Weave is reusable.
2. Prefer a **new** `benches/<family>.rs` (or extend an existing family) over a single kitchen-sink file.
3. Register `[[bench]]` in `crates/wyrd-runtime/Cargo.toml` (`path` + `harness = false`).
4. Re-measure f32 **and** i32; paste medians into this file.

## Next measurement / opt candidates

P1–P3 expand product/stateful/emit/pattern/Bevy coverage. After that: cache `KindTag` at bind, reduce clear+gather traffic, flat inbound CSR — prioritize where **scaled** benches show tax (Digitize/Sqrt first).
