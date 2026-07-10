# Wyrd performance notes

**Machine:** local developer host (see date below). Numbers are **indicative**, not a guarantee.  
**Measured:** 2026-07-10 · Divan release · `cargo bench -p wyrd-runtime`

## How to bench

Bench targets are **split** so the suite can grow without one mega-file. Shared Weave builders live in `crates/wyrd-runtime/benches/common.rs` (not a stand-alone target; `autobenches = false`).

| Target | Path | Focus |
| --- | --- | --- |
| `settle_chain` | `benches/settle_chain.rs` | Not depth, And door, host `tick_once` |
| `settle_catalog` | `benches/settle_catalog.rs` | Map/Digitize, Calc/Abs, Threshold, fan-out |
| `settle_stateful` | `benches/settle_stateful.rs` | Delay rings, gated Random |
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

## Adding a new bench

1. Add a builder in `benches/common.rs` if the Weave is reusable.
2. Prefer a **new** `benches/<family>.rs` (or extend an existing family) over a single kitchen-sink file.
3. Register `[[bench]]` in `crates/wyrd-runtime/Cargo.toml` (`path` + `harness = false`).
4. Re-measure f32 **and** i32; paste medians into this file.

## Next measurement / opt candidates

See earlier perf discussion: cache `KindTag` at bind, reduce clear+gather traffic, flat inbound CSR, unchecked port access on the hot path. Re-baseline this doc after each change.
