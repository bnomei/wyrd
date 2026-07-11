# Performance model: bind once, settle predictably

Use this guide when a Wyrd integration runs every frame or targets a constrained host. It explains
the architectural guarantees and how to measure them; it does not promise a frame time for an
entire game.

## The contract

Authoring uses names, validation, and flexible topology. `Runtime::bind` turns a validated `Weave`
into dense runtime state: host paths and commands are interned, inbound edges become compact tables,
topological order and dispatch tags are prepared, and state/outbox/delay storage is allocated for
the bound graph.

Each later `Runtime::loom()` performs one deterministic pass over that bound DAG:

```text
clear unwired inputs → seed constants / senses / OnStart → gather inbound values
→ evaluate topological order → append SignalOut and EmitCommand results
```

The per-tick work therefore grows with the active graph and its effects, rather than with string
lookups or topology construction. It is a full settle, not a dirty-node incremental evaluator.
Bind at setup or room load; do not rebuild a Weave every frame.

## Why that matters for game rules

Puzzles often run continuously: overlap facts change, doors need their current state, movers need
a current target, and trigger requests must not duplicate while a player remains in a volume.
Dense runtime handles and precomputed topology keep this rule layer predictable while the host does
the heavier work—physics, rendering, collision queries, asset loading, and animation.

That is a reason to keep a Weave focused on an active room or puzzle island, not a claim that Wyrd
alone makes a whole game fast. Profile the complete Sample → Loom → Apply path in your engine.

## Bounds are part of the design

The default graph budget rejects more than 256 knots or 512 threads and reports soft warnings past
64 knots or 128 threads. It also limits chain depth, fan-out, and cumulative delay-path length.
`BindOpts::budget` lets a host choose stricter or measured limits, while
`BindOpts::max_emits_per_tick` defaults to eight emitted commands and reports dropped emits.

Treat those limits as game safety and frame-budget guardrails. Raise them only after measuring a
representative loaded graph on the target hardware.

## What “no allocation on the hot path” means

After bind, loom does not allocate graph topology. Delay storage and outbox capacity are prepared
from the graph, and the runtime's `zero_alloc_loom` test checks that repeated settles do not grow
those buffers.

This is deliberately narrower than a global no-allocation promise. Your host can still allocate
while sampling, applying effects, logging, building command vectors, running Bevy systems, or doing
any other game work. On the hottest path, iterate the runtime outbox directly; the convenient
`outbox_to_commands` helper creates a new `Vec`.

## Numeric paths and constrained hosts

The core, graph, and runtime support either `signal-f32` or Q16 `signal-i32`. The latter is useful
when the host wants predictable fixed-point signals; binding prepares specialized map plans and the
runtime uses bounded integer implementations where appropriate. This is not a universal performance
ranking: profile the actual knot mix on the actual device. `wyrd-bevy` is f32-only because Bevy
types are float-native.

For a Playdate-class host, use `wyrd-runtime` directly, bind on room load, resolve handles once,
and profile a representative graph on physical hardware.

## Measure the right thing

Run the checked behaviour first:

```bash
cargo test -p wyrd-runtime --test zero_alloc_loom
cargo test -p wyrd-runtime --test tutorial_ladder d01_shrine_chamber
```

Then run the benchmark families that match the question:

```bash
# Runtime chains, catalog operations, stateful knots, bind, and isolated evaluation paths
cargo bench -p wyrd-runtime

# One family while iterating
cargo bench -p wyrd-runtime --bench settle_chain
cargo bench -p wyrd-runtime --bench bind

# Q16 behaviour on the target toolchain
cargo bench -p wyrd-runtime --no-default-features --features "std,signal-i32"
```

The repository includes runtime chain, catalog, stateful, bind, and isolated evaluation benchmarks,
plus a headless Bevy Sample → Loom → Apply benchmark. CodSpeed tracks the workspace’s default
feature benchmark build; correctness CI covers the integer and `no_std` paths, but that is not an
i32 performance measurement.

When a game is slow, measure host sampling and application beside `loom()`. A graph may be cheap
while a spatial query, animation update, or command allocation dominates the actual frame.

## Source anchors

- Bind-time layout and handle interning: [`crates/wyrd-runtime/src/bind.rs`](../../crates/wyrd-runtime/src/bind.rs)
- Single-pass settle: [`crates/wyrd-runtime/src/loom.rs`](../../crates/wyrd-runtime/src/loom.rs)
- Budget and warnings: [`crates/wyrd-graph/src/validate.rs`](../../crates/wyrd-graph/src/validate.rs)
- Buffer-stability test: [`crates/wyrd-runtime/tests/zero_alloc_loom.rs`](../../crates/wyrd-runtime/tests/zero_alloc_loom.rs)
- Benchmark targets: [`crates/wyrd-runtime/benches`](../../crates/wyrd-runtime/benches) and [`crates/wyrd-bevy/benches/host_tick.rs`](../../crates/wyrd-bevy/benches/host_tick.rs)
