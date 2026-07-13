# Runtime and embedded performance audit

**Date:** 2026-07-13
**Baseline revision:** `adc35ce8b7ffb7a972ca436a2b7555d786642ee3`
**Implementation branch:** `perf/runtime-audit-portable`
**Status:** portable changes implemented and validated; target-sensitive experiments remain hardware-gated
**Primary metric:** release-mode latency per `Runtime::loom()` on representative benchmark shapes
**Secondary metrics:** allocation behavior, bound-runtime footprint, snapshot cost, bind cost, and physical-device cycles/bytes

## Outcome

The steady-state loom is already structurally lean: bind-time dense IDs, CSR inbound edges, bind-time dispatch/arithmetic plans, unwired-input-only clearing, small-fan-in fast paths, reused outboxes, and preallocated delay rings are all present. The current measurements do not justify another broad dispatch rewrite.

The audit ranked the highest-confidence work as:

1. repair benchmark validity before using the suite for cross-feature decisions;
2. cap Emit outbox reservation at the configured emit limit;
3. cache the immutable runtime fingerprint and add reusable snapshot storage;
4. measure and reduce bound-runtime memory (unused `KindTag` fields, fixed port stride, sparse state, and cold retained bind artifacts);
5. benchmark exact i32 Digitize plans and non-trivial numeric operands;
6. treat dirty/incremental evaluation as a research branch, not a default replacement for the full scan.

The strongest baseline numeric result is a crossover, not a winner: on this Apple M4 host, Q16 Map is about 2.2x faster than f32 Map for the existing identity-range chain, while exact Q16 square root is about 6.4x slower than f32 square root for the existing `sqrt(ONE)` chain. Neither result predicts Playdate hardware. Physical-device measurement remains mandatory.

## Implementation outcome

The portable branch keeps each independently reviewable change in a separate commit:

| Commit | Outcome | Evidence |
| --- | --- | --- |
| `c3d0db9` | Repaired benchmark inputs, work counters, first/steady-state separation, bind timing regions, and output observation. | Both numeric paths compile and the workspace bench targets execute under `cargo test --all-targets`. |
| `5bd453d` | Capped Emit outbox reservation at `min(emit_knots, max_emits_per_tick)`. | Unit coverage verifies caps 0, 8, and all without changing emit/drop semantics. |
| `fc666d1` | Cached the immutable runtime fingerprint at bind. | Golden values cover f32 and i32; policy and graph mutations change the fingerprint. |
| `457743f` | Added the portable `runtime_foundation` benchmark matrix. | Covers numeric work, Delay 3/4, Emit caps, snapshots, sense density/writes, activity, and the tier-D representative flow. |
| `ca50ee6` | Added `Runtime::snapshot_into` for reusable snapshot storage while retaining allocating `snapshot()`. | Fresh/reused equivalence, continuation, cross-owner restore, and incompatible-buffer overwrite tests pass. |
| `96f409d` | Removed retained `id_to_name` storage after bind. | Name and handle behavior remain covered by the runtime test suite. |
| `bd548d3` | Removed retained per-knot `input_slots` storage after bind. | Checked ports and hot-path bind tables remain covered. |
| `1b54111` | Stopped retaining the temporary thread list after bind and fingerprint construction. | Topology, fingerprint, restore, and continuation tests pass. |
| `6649e5f` | Added focused snapshot restore branch coverage required by the repository gate. | Both numeric coverage runs exercise version, shape, handle-index, and valid reconstruction paths. |
| `db92e25` | Added stable diagnostic coverage for every `RestoreError` variant. | Formatting and source-chain assertions pass. |
| `25b98fc` | Isolated and covered the Tier-C composer error mapping exposed by the full coverage gate. | Build and validation error mappings both pass. |

The portable runtime changes reduce retained or repeated work without changing evaluation order or numeric semantics. Dense state compaction, `KindTag` layout changes, port-stride changes, Digitize arithmetic plans, sense/topology changes, and dirty evaluation are intentionally not in this branch: their value or safety depends on final-target layout, cycles, memory, or workload evidence.

## Experiment

### Environment

| Field | Value |
| --- | --- |
| Machine | MacBook Air `Mac16,12`, Apple M4, 10 cores, 16 GB |
| OS | Darwin 25.5.0, arm64 |
| Rust | `rustc 1.96.0 (ac68faa20 2026-05-25)`, LLVM 22.1.2 |
| Cargo | 1.96.0 |
| Harness | `codspeed-divan-compat` 4.7.0, wall-time mode |
| Bench profile | optimized, workspace release settings `lto = true`, `codegen-units = 1` |
| Timer precision | 41 ns as reported by Divan |
| Git state before measurement | `main`, two commits ahead of `origin/main`, otherwise clean |

Commands:

```bash
cargo test -p wyrd-for-games --test zero_alloc_loom --locked
cargo test -p wyrd-for-games --no-default-features --features "std,signal-i32" --test zero_alloc_loom --locked
cargo test -p wyrd-for-games --test runtime_state --locked

cargo bench -p wyrd-for-games --bench settle_chain --locked -- \
  settle_not_chain --sample-count 100 --min-time 0.5
cargo bench -p wyrd-for-games --bench settle_chain \
  --no-default-features --features "std,signal-i32" --locked -- \
  settle_not_chain --sample-count 100 --min-time 0.5

cargo bench -p wyrd-for-games --bench settle_iso --locked -- \
  --sample-count 100 --min-time 0.2
cargo bench -p wyrd-for-games --bench settle_iso --locked -- \
  iso_eval_ --sample-count 300 --min-time 1
cargo bench -p wyrd-for-games --bench settle_iso \
  --no-default-features --features "std,signal-i32" --locked -- \
  --sample-count 100 --min-time 0.2
cargo bench -p wyrd-for-games --bench settle_iso \
  --no-default-features --features "std,signal-i32" --locked -- \
  iso_eval_ --sample-count 300 --min-time 1

cargo bench -p wyrd-for-games --bench settle_chain --locked -- \
  settle_and_door --sample-count 300 --min-time 1
cargo bench -p wyrd-for-games --bench settle_chain --locked -- \
  tick_once_not_chain --sample-count 300 --min-time 1
cargo bench -p wyrd-for-games-bevy --bench host_tick --locked -- \
  --sample-count 100 --min-time 0.5
```

### Repeated medians

Each cell is median latency from two independent process runs. The scaled numeric rows contain 64 operators plus the input/output knots; the Delay row contains 32 Delay knots.

| Workload | f32 run 1 / run 2 | i32 run 1 / run 2 | What it establishes |
| --- | ---: | ---: | --- |
| structural Not, 64 | 193.0 / 193.0 ns | 192.3 / 193.0 ns | fixed dispatch/gather cost is equivalent here |
| Delay, 32, length 4 | 150.1 / 149.5 ns | 150.1 / 149.5 ns | delay-ring traffic is equivalent here |
| Map, 64, identity range | 446.9 / 439.9 ns | 198.2 / 201.6 ns | existing i32 Map plan is effective for this special case |
| Map, 64, i32 power-of-two plan | n/a | 204.7 / 205.4 ns | shift plan matches the identity fast path closely |
| Map, 64, i32 general division | n/a | 366.2 / 366.9 ns | general division is about 1.8x slower than shift here |
| Digitize, 64, upper endpoint | 582.4 / 557.1 ns | 540.6 / 536.2 ns | paths are close; operand coverage is too narrow |
| Div, 64, divisor `ONE` | 226.9 / 228.9 ns | 223.0 / 223.7 ns | identity specialization, not general division |
| Sqrt, 64, input `ONE` | 193.0 / 193.7 ns | 1.228 / 1.239 us | exact restoring i32 sqrt is about 6.4x slower on M4 |

Additional context:

- `settle_and_door` measured 12.14 ns median, while the full headless Bevy `App::update()` measured 1.832-1.853 us median. These timed regions are deliberately different, and the direct microbenchmark is extremely small, so this is not a speedup ratio. It does show that host/ECS work dominates this tiny integration case.
- `tick_once_not_chain(64)` measured 196.4 ns versus 193.0 ns for the direct 64-Not settle, so the `NullHost` wrapper adds little in that synthetic workload.
- Divan's observed maxima include scheduler interruption and are not WCET measurements.
- The repeated medians were close, but raw sample artifacts were not retained. This report is a baseline, not a regression gate.

## Baseline evidence blockers and resolution

Commit `c3d0db9` repairs the benchmark defects discovered during the baseline audit:

- Numerically equivalent f32/Q16 inputs now use `ONE` or domain-specific constructors consistently ([catalog benches](../../crates/wyrd-for-games/benches/settle_catalog.rs)).
- Identity controls remain, while the portable matrix adds midpoint Digitize, non-identity Mul/Div, non-perfect Sqrt, Map, and Count cases ([foundation matrix](../../crates/wyrd-for-games/benches/runtime_foundation.rs)).
- Random, Emit, and other multi-loom samples count the actual number of loom operations; the Bevy benchmark reports update latency rather than cross-family knot throughput ([stateful benches](../../crates/wyrd-for-games/benches/settle_stateful.rs), [host bench](../../crates/wyrd-for-games-bevy/benches/host_tick.rs)).
- Bind measurements now distinguish precloned bind, clone-plus-bind, clone-only, and build/include-plus-bind work ([bind benches](../../crates/wyrd-for-games/benches/bind.rs)).
- OnStart has separate first-loom and steady-state cases so harness warm-up cannot erase the firing path ([catalog benches](../../crates/wyrd-for-games/benches/settle_catalog.rs)).
- Benchmark outputs and representative host commands are observed through `black_box`, preventing dead-result measurements.

The remaining evidence boundaries are deliberate:

- `zero_alloc_loom` proves stable capacities and delay sizing, not zero allocator calls, bounded bytes, fragmentation, or RSS ([test](../../crates/wyrd-for-games/tests/zero_alloc_loom.rs)).
- [The tracked performance model](../concepts/performance-model.md) is the behavioral contract; this dated report records local evidence and implementation decisions.
- CodSpeed measures default-feature f32 only and floats both the `stable` toolchain and installed `cargo-codspeed` version ([workflow](../../.github/workflows/codspeed.yml)). Its signal is distinct from local wall time and physical-device timing.
- The baseline medians above were not rerun as a matched before/after experiment after every portable commit. They must not be presented as speedups.

## Ranked candidates

| Rank | Candidate | Expected scope | Confidence | Risk / semantic cost |
| ---: | --- | --- | --- | --- |
| 1 | Repair benchmark semantics and counters | decision quality across all later work | very high | low |
| 2 | Cap Emit outbox reservation by policy | bind-time heap and fragmentation for emit-heavy graphs | very high | low |
| 3 | Cache fingerprint and reuse snapshot buffers | snapshot/rollback latency and allocation | high | low-medium |
| 4 | Remove unused i32 `KindTag` payload, then measure side plans | bound footprint and hot tag locality | high for dead fields; medium for side plans | low, then medium |
| 5 | Compact sparse state and cold retained bind artifacts | heap, snapshot bandwidth, possibly locality | medium-high | medium-high |
| 6 | Test a 5-signal port stride against stride 8 | exactly 12 payload bytes/knot, potentially 3 KiB at 256 knots | high for bytes; unknown for speed | medium |
| 7 | Precompute exact i32 Digitize plans | i32 numeric latency on division-heavy targets | medium | medium |
| 8 | Compact sense metadata and use eval-only topo | sense-heavy sampling and fixed scan overhead | medium | low-medium |
| 9 | Sparse/dense dirty evaluator experiment | large, mostly stable combinational graphs only | low-medium | high |

## Candidate reports

### 1. Repair benchmark semantics and counters

- **Hotspot:** Performance decisions themselves.
- **Evidence:** Invalid Q16 input, identity operands, two-loom counter mismatches, clone-inclusive bind rows, and inactive OnStart measurement are listed above.
- **Candidate and mechanism:** Feed semantically identical values; add midpoint, boundary, non-perfect-square, non-identity Mul/Div, Count, and conversion cases; report forced-event latency for two-loom benches; split clone-only, owned-bind, and build-plus-bind; construct or reset state when first-frame behavior is the target.
- **Expected scope (not promised speedup):** No runtime speedup; prevents false rankings and makes later changes disprovable.
- **Semantic and operational risks:** Larger suite and longer CI. Keep a small per-change matrix and a scheduled full/device matrix.
- **Benchmark plan:** Use the measurement matrix below and two alternating process runs at `--sample-count 300 --min-time 1`.
- **Result:** Implemented in `c3d0db9`; the new portable matrix landed in `457743f`. The corrected benchmark targets compile and execute for the workspace, and identity cases remain explicitly named controls.
- **Decision and fallback:** Keep the corrected suite as the basis for later host/device decisions. Do not compare old and corrected throughput counters as a regression series.

### 2. Cap Emit outbox reservation by policy

- **Hotspot:** Bind-time allocation and retained capacity for emit-heavy graphs.
- **Baseline evidence at `adc35ce`:** Bind counted every Emit knot and reserved `act_emits`, while `push_emit` could retain at most `max_emits_per_tick`. The current bounded reservation is in [bind](../../crates/wyrd-for-games/src/runtime_impl/bind.rs), and the default cap remains eight.
- **Candidate and mechanism:** Reserve `min(act_emits, usize::from(opts.max_emits_per_tick))`.
- **Expected scope (not promised speedup):** Lower heap request/retained capacity when Emit knot count exceeds the cap; no steady-state loom latency claim.
- **Semantic and operational risks:** Low. Snapshot restore already rejects outbox shapes above capacity, and valid snapshots cannot contain more retained emits than the cap.
- **Benchmark plan:** Bind 8/32/256 Emit graphs at caps 0/8/all; record requested and rounded bytes, peak live heap, largest free block, bind latency, and restore behavior.
- **Result:** Implemented in `5bd453d`. Reservation is exactly `min(act_emits, max_emits_per_tick)`; tests cover a zero cap, a smaller cap, and a cap larger than the graph. Short host bind probes were noisy and do not support a latency claim.
- **Decision and fallback:** Keep the capacity bound because it matches the maximum representable retained outbox state. Revisit only if allocator/device evidence demonstrates a regression.

### 3. Cache fingerprint and reuse snapshot buffers

- **Hotspot:** Rollback, save-every-frame, and repeated restore workloads.
- **Baseline evidence at `adc35ce`:** Every snapshot hashed immutable knots, threads, names, feature policy, and bind options, then cloned dense state vectors. Restore recomputed the same hash before reusing vector storage. The current implementation is in [runtime state](../../crates/wyrd-for-games/src/runtime_impl/runtime_state.rs).
- **Candidate and mechanism:** Compute/store the immutable fingerprint once at bind. Add `snapshot_into(&mut RuntimeState)` or an equivalent refresh API using `clone_from`, `clear`, and `extend`; retain allocating `snapshot()` as convenience.
- **Expected scope (not promised speedup):** Remove repeated O(knots + threads + name bytes) hashing and allocator calls after warm-up. State-copy bandwidth remains.
- **Semantic and operational risks:** Cached input must be truly immutable; rejected restore must remain atomic; reusable buffers can retain burst capacity.
- **Benchmark plan:** 16/64/256 knots; stateless/stateful/delay-heavy; empty/full outboxes; fresh versus reused snapshot; restore; allocator calls/bytes and latency for both numeric paths.
- **Result:** Implemented in `fc666d1` and `ca50ee6`, after adding the matrix in `457743f`. A short final-code smoke probe measured a reused 64-Not snapshot near 40-41 ns versus roughly 243-246 ns for a fresh snapshot on this M4 host. This is an allocating-versus-reused API comparison on one final revision, not a matched change-level speedup. Golden fingerprints, refreshed/fresh equivalence, continuation, cross-owner restore, incompatible-buffer overwrite, and atomic rejection pass for both numeric paths.
- **Decision and fallback:** Keep `snapshot_into` for repeated capture and allocating `snapshot()` for convenience. Callers choose retention behavior explicitly.

### 4. Shrink `KindTag` only with layout evidence

- **Hotspot:** One copied/matched dispatch tag per evaluated knot.
- **Evidence:** Every knot stores a `KindTag`, and the largest variant determines enum size ([storage](../../crates/wyrd-for-games/src/runtime_impl/bind.rs#L92-L100), [dispatch](../../crates/wyrd-for-games/src/runtime_impl/loom.rs#L102-L105)). Under i32, `Digitize` still carries `bin_scale`, `out_scale`, and `last_f`, initializes them to zero, and discards them during eval ([tag](../../crates/wyrd-for-games/src/runtime_impl/kind_tag.rs#L258-L277), [initialization](../../crates/wyrd-for-games/src/runtime_impl/kind_tag.rs#L519-L536), [eval](../../crates/wyrd-for-games/src/runtime_impl/loom.rs#L889-L899)).
- **Candidate and mechanism:** First cfg-gate the three f32-only fields. Then, only if type-size and cache evidence remains material, compare inline Map/Digitize plans with compact plan indices into side arrays.
- **Expected scope (not promised speedup):** Potential bound-memory and cache-density reduction; exact effect depends on enum layout.
- **Semantic and operational risks:** Side-plan indirection can lose on Map/Digitize-heavy graphs and adds storage complexity.
- **Benchmark plan:** Capture `size_of::<KindTag>()` for both features, type layouts, heap bytes, `.text`, and device cycles for Not/mixed/Map/Digitize graphs at 64/256 knots.
- **Result:** Three fields are source-proven dead under i32; enum-size and final-target effects remain unmeasured. No layout change was kept on the portable branch.
- **Decision and fallback:** Hardware-gated. Remove dead feature payload only after recording host and device layout/ELF evidence; do not move plans out of line unless the mixed-workload crossover supports it.

### 5. Compact sparse mutable state and cold retained artifacts

- **Hotspot:** Bound-runtime heap footprint and snapshot bandwidth.
- **Baseline evidence at `adc35ce`:** Runtime allocated knot-count-sized arrays for senses and every state family, plus three delay metadata arrays, regardless of actual rune mix. It also retained full `KnotKind`, threads, name vectors, and per-knot input-slot vectors after producing compact runtime plans. Current dense state and runtime layout remain visible in [bind](../../crates/wyrd-for-games/src/runtime_impl/bind.rs); the cold artifacts removed by this branch are listed in the result below.
- **Candidate and mechanism:** Assign compact state slots by rune category; cache the fingerprint; replace cold retained authoring data with the minimum metadata required for checked handles/tooling; drop artifacts with no production reads.
- **Expected scope (not promised speedup):** Lower heap and snapshot copy volume on mostly stateless graphs; possible locality improvement.
- **Semantic and operational risks:** Snapshot format/shape, exact fingerprints, checked port/domain errors, debug surfaces, and handle behavior all constrain the redesign. Extra state-slot indirection may slow stateful-heavy graphs.
- **Benchmark plan:** Stateless, mixed, all-Counter, all-Delay, and all-Random graphs at 64/256 knots; live/peak bytes, snapshot bytes/latency, settle p50/p99, and differential continuation tests.
- **Result:** The low-risk cold subset landed separately: `id_to_name` (`96f409d`), per-knot `input_slots` (`bd548d3`), and the temporary thread list (`1b54111`) are no longer retained after bind. Dense mutable state and snapshot layout were not compacted.
- **Decision and fallback:** Keep the proven cold-storage removals. Hardware-gate the dense-state/SenseId contract redesign as a dedicated memory project, with the current dense arrays as fallback and differential oracle.

### 6. Test port stride 5 versus 8

- **Hotspot:** `port_vals` occupies `knots * MAX_PORTS`, and every port access computes that stride.
- **Evidence:** `MAX_PORTS` is eight and the buffer is allocated at eight Signals per knot ([layout](../../crates/wyrd-for-games/src/runtime_impl/bind.rs#L128-L129), [allocation](../../crates/wyrd-for-games/src/runtime_impl/bind.rs#L480-L483)). The shipped catalog currently exposes at most slots 0-4 through four-input And/Or ([ports](../../crates/wyrd-for-games/src/foundation/ports.rs#L111-L125), [arity validation](../../crates/wyrd-for-games/src/foundation/ports.rs#L250-L257)).
- **Candidate and mechanism:** Compare a validated fixed stride of five with the existing power-of-two stride eight.
- **Expected scope (not promised speedup):** Three fewer four-byte Signals per knot: 12 payload bytes/knot, or 3 KiB at the 256-knot default hard budget.
- **Semantic and operational risks:** Multiply-by-five addressing may cost more than shift-by-eight on a 32-bit target, and stride five reduces future arity headroom. Packed per-knot offsets save more memory but add another hot load.
- **Benchmark plan:** Exact allocation bytes plus device cycles/jitter for Not, fan-out, mixed, Map, and max-fan-in graphs at 64/256 knots.
- **Result:** Payload saving is arithmetic; final-target runtime and allocator effects are unmeasured. No stride change was kept.
- **Decision and fallback:** Hardware-gated. Accept only on a RAM/cycles Pareto comparison. Keep stride eight if device latency or catalog evolution outweighs 3 KiB.

### 7. Precompute exact i32 Digitize plans

- **Hotspot:** i32 Digitize performs two signed i64 divisions per evaluated knot ([eval](../../crates/wyrd-for-games/src/runtime_impl/loom.rs#L889-L899)).
- **Evidence:** Existing i32 Map already gcd-reduces and selects Unit/Scale/Shift/Divide plans at bind ([plan](../../crates/wyrd-for-games/src/runtime_impl/kind_tag.rs#L11-L18), [selection](../../crates/wyrd-for-games/src/runtime_impl/kind_tag.rs#L55-L104)). Current endpoint-only Digitize medians were only slightly better than f32 on M4, but that workload does not exercise value distributions or a 32-bit helper-call cost.
- **Candidate and mechanism:** Apply exact ratio reduction and power-of-two shift selection to both Digitize divisions without changing truncation, endpoints, descending ranges, or Count semantics.
- **Expected scope (not promised speedup):** Division-heavy constrained targets and favorable authored ranges; neutral or worse where plan size/cache cost dominates.
- **Semantic and operational risks:** Exact rounding and full-domain overflow behavior are contractual. Larger tags can erase the arithmetic win.
- **Benchmark plan:** General/power-of-two denominators, endpoints/midpoints/random values, ascending/descending/full range; bit-exact differential corpus; device ELF inspection for division helpers; cycles and flash.
- **Result:** The portable matrix now exercises midpoint Digitize and non-trivial numeric operands, but no physical-target division/helper evidence exists. No arithmetic-plan change was kept.
- **Decision and fallback:** Hardware-gated. Prototype beside the current exact implementation and keep the generic division plan as fallback.

### 8. Compact sense metadata and lean the fixed scan

- **Hotspot:** Many host sense writes and graphs with a high ratio of sense knots.
- **Evidence:** `PortWriter::set_sense` reaches into the retained full `KnotKind` to recover a domain on every write ([source](../../crates/wyrd-for-games/src/runtime_impl/outbox.rs#L68-L89)). Senses are separately seeded, but the topo loop still loads a tag and branches over them ([loom](../../crates/wyrd-for-games/src/runtime_impl/loom.rs#L30-L62)).
- **Candidate and mechanism:** Bind compact sense-domain metadata, and compare an eval-only topo with the current all-knot topo. Encode the fixed port stride as a constant only if assembly/counters show it is not already folded.
- **Expected scope (not promised speedup):** Sense-heavy sampling and high-sense-ratio graphs; little benefit for one/two-sense or eval-heavy rooms.
- **Semantic and operational risks:** Preserve foreign-runtime/invalid-sense/domain errors and deterministic topo order. Extra metadata consumes memory.
- **Benchmark plan:** 1/16/64/256 writes across Bool/Level/Count and both numeric paths; 0/25/75% sense ratios; isolated sample and full tick; instructions/branches on host and device.
- **Result:** Focused sense-write and sense-density cases now exist in `runtime_foundation`; no physical device result exists. No metadata/topology change was kept.
- **Decision and fallback:** Hardware-gated. Retain the current path if LLVM or tiny graph size makes the change neutral.

### 9. Sparse/dense dirty evaluation is research only

- **Hotspot:** The full evaluator clears, seeds, and scans the whole bound DAG every tick ([loom](../../crates/wyrd-for-games/src/runtime_impl/loom.rs#L19-L63)).
- **Evidence:** Dirty propagation can remove work only when changed inputs and downstream cones stay sparse. Wyrd also has always-active stateful knots, shared RNG ordering, every-frame SignalOut, capped ordered emits, and small default budgets.
- **Candidate and mechanism:** Experimental deterministic dirty frontier plus an activity threshold that falls back to the current full scan; retain full evaluation as correctness oracle.
- **Expected scope (not promised speedup):** Potentially large only for large, mostly stable combinational graphs with sparse observed outputs.
- **Semantic and operational risks:** High: timers, delays, RNG sequence, emit order/drop counts, SignalOut refresh, duplicate frontier work, snapshots, and threshold hysteresis. It likely loses on tiny/default-sized, dense-change, high-fan-out, or stateful graphs.
- **Benchmark plan:** Chains, stars, diamonds, layered DAGs, and captured rooms; 0/1/5/25/100% changes; state/output density; evaluated nodes, queue pushes, bytes, p50/p99; randomized differential replay against full evaluation.
- **Result:** No sparse-change workload or profile currently justifies implementation. No dirty evaluator was implemented.
- **Decision and fallback:** Deferred research. The current full scan remains the production design and oracle even on the hardware branch until representative traces show a sparse crossover.

## Do not re-propose without new evidence

- Keep bind-time dense IDs, CSR inbound tables, dispatch tags, and arithmetic plans.
- Keep unwired-input-only clearing and the 0/1/2-edge gather fast paths.
- Keep the existing i32 Map specialization; measured identity and shift paths are already strong.
- Keep exact restoring i32 sqrt unless device evidence supports a bounded LUT or approximation with an explicit error/range contract. The M4 result alone does not justify changing semantics.
- Do not presume fixed point beats float on a Cortex-M7-class device; compare numerically equivalent kernels in the final consuming target.
- Do not replace random range mapping with modulo or masks without an explicit distribution contract.
- Do not add setup-only hash maps for host path/command lookup; hosts resolve handles once.
- Do not add unsafe indexing. Hot port access is already direct, bind-validated indexing with debug assertions.
- Do not force heapless maximum-capacity arrays or power-of-two Delay lengths without allocator and game-rule evidence.
- Do not add unconditional parallelism. Default graphs are small, deterministic, and dependency-bound.

## Minimum next measurement matrix

| Layer | Cases | Target/features | Primary metrics |
| --- | --- | --- | --- |
| Structural loom | Not 16/64/128; fan-out 32/64; sense ratios | f32 + i32 host | ns/settle, instructions, branches |
| Numeric | general Map; Digitize midpoint/boundary; non-identity Mul/Div; non-perfect Sqrt; Convert | f32 + i32, identical semantics | ns/64-op chain; exact output |
| Stateful/effects | Delay lengths 3/4; gated Random; Emit 8/32 at caps 0/8/all | f32 + i32 | ns/settle/event; heap bytes |
| Snapshot | fresh/reused snapshot and restore at 16/64/256 | f32 + i32 | latency, allocations, bytes copied |
| Bind | clone-only; owned bind; build-plus-bind; invalid cases | f32 + i32 | us/graph, allocations, peak heap |
| Representative | tier-D chamber Sample -> Loom -> Apply | f32 + i32 host | tick latency, output correctness |
| Engine | scripted headless Bevy update | f32 | ns/update, not knots/s |
| Physical | same representative graphs in consuming game | final device build | cycles/distribution, heap/stack, ELF sections |

For physical Playdate-class measurement, record device/SDK/rustc/target/profile/logging, calibrate the counter bracket, and separate first-after-reset from warmed runs. Retain min/median/p95/p99/max and interrupt policy; the maximum observed sample is not WCET. Instrument allocations/frees/requested and rounded bytes/peak live/largest free block/recovery, stack watermark, and final `.text`/`.rodata`/`.data`/`.bss`. Inspect the final image for wide arithmetic and math helper calls. Simulator results are functional and triage evidence only, never device-performance claims.

## Hardware-gated continuation

Target-sensitive work belongs on `perf/playdate-hardware-validation`, based on the validated portable tip. The local Playdate SDK and `pdc` compiler are present, but this checkout currently lacks `cargo-playdate`, an installed ARM Rust target, and a confirmed connected physical-device runner. A simulator cannot satisfy the performance gate.

Do not keep candidates 4, 6, 7, or 8—or the dense-state/SenseId portion of candidate 5—without all of the following:

1. a representative device workload and stable measurement harness;
2. matched before/after device distributions with correctness checks;
3. heap/stack and final ELF section evidence where memory/layout is the claim;
4. rollback of any candidate that regresses the primary metric, violates semantics, or only wins in the simulator.

Candidate 9 remains research-only even after hardware becomes available; it additionally requires captured sparse-change workloads and differential replay against the full evaluator.

## Validation completed

- `just quality`: formatting, Clippy with warnings denied, workspace tests, UI tests, examples, and all bench targets passed.
- `just coverage`: f32, i32, serde codec variants, and Bevy coverage passed; both source trees report 100% source-line coverage.
- `just msrv-no-std`: Rust 1.75 checks passed for alloc-only f32 and i32 configurations.
- `just publish-readiness`: package file lists and warning-free workspace documentation passed.
- `cargo test --manifest-path integrations/wyrd-moirai/Cargo.toml --locked`: four integration and restore-continuation tests passed.

The portable implementation is complete. Host smoke measurements support the reusable-snapshot mechanism, but no commit is claimed as a matched end-to-end loom speedup. Hardware-dependent optimization completion remains blocked on physical-device evidence.
