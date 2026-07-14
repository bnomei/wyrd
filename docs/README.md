# Wyrd documentation

Use Wyrd to author deterministic game behaviour as a validated signal graph. This documentation
starts with the reader's job, while rustdoc and the source remain the exact API reference.

## Choose a path

- Start with the repository [quickstart](../README.md#quickstart) when you want the smallest
  complete `SignalIn` → `SignalOut` path.
- Read [vision and scope](concepts/vision-and-scope.md) when you need to decide what belongs in a
  Weave, what remains host-owned, and how local puzzle machines participate in a larger game.
- Follow the [`wyrd::examples`](https://docs.rs/wyrd-for-games/latest/wyrd/examples/) tiers for
  executable lessons covering latches, timers, counters, triggers, mover targets, and
  chamber-scale composition.
- Read the [performance model](concepts/performance-model.md) before setting graph budgets or
  optimizing a per-frame integration.

## Documentation boundaries

The tracked pages in this directory describe the 0.4 source line. Local `docs/` research and
planning material remains intentionally ignored: it may contain candidate APIs, raw investigation,
or historical measurements. Do not treat it as a public or current contract.

For exact catalog ports, runtime errors, and public signatures, start from the package READMEs and
their linked source:

- [`wyrd-for-games` (`wyrd`)](../crates/wyrd-for-games/README.md)
- [`wyrd-for-games-bevy` (`wyrd_bevy`)](../crates/wyrd-for-games-bevy/README.md)

## Documentation and example contract

Use one documentation surface for each job:

- Keep one complete first-success path in the root README. Its code must compile as a doctest or
  have an equivalent integration test.
- Put exact ownership, errors, panics, allocation behavior, and lifecycle guarantees on the public
  Rust item that owns the contract. Enable `# Errors`, `# Panics`, or `# Safety` sections only when
  they apply.
- Add a Rustdoc `# Examples` section when a public API is easier to understand through a short,
  isolated use. Prefer a runnable doctest; use hidden `#` setup lines to keep the rendered example
  focused, `no_run` only when a real engine or device loop cannot run in Rustdoc, and
  `compile_fail` for an intentionally rejected API shape.
- Put reusable engine-neutral lessons in `wyrd::examples`, grouped into ordered tiers with one
  descriptive module per lesson. Keep the full example in its Rustdoc and compile it as a doctest.
- Put complete host integrations that need setup, scheduling, or several systems in a crate's
  `examples/` directory and compile or run them in CI. Keep regression assertions in `tests/`, not
  in prose-only snippets.

Validate all public examples with:

```bash
cargo test -p wyrd-for-games --doc --no-default-features \
  --features "std,signal-f32,serde-ron,serde-json,schema" --locked
cargo build -p wyrd-for-games-bevy --example and_door
```
