# Wyrd documentation

Use Wyrd to author deterministic game behaviour as a validated signal graph. This documentation
starts with the reader's job, while rustdoc and the source remain the exact API reference.

## Choose a path

- Start with the repository [quickstart](../README.md#quickstart) when you want the smallest
  complete `SignalIn` → `SignalOut` path.
- Read [vision and scope](concepts/vision-and-scope.md) when you need to decide what belongs in a
  Weave, what remains host-owned, and how local puzzle machines participate in a larger game.
- Use [choose a puzzle shape](examples/README.md) to find an executable recipe for a latch, timer,
  counter, trigger, mover target, or chamber-scale combination.
- Read the [performance model](concepts/performance-model.md) before setting graph budgets or
  optimizing a per-frame integration.

## Documentation boundaries

The tracked pages in this directory describe the released 0.2 code. Local `docs/` research and
planning material remains intentionally ignored: it may contain candidate APIs, raw investigation,
or historical measurements. Do not treat it as a public or current contract.

For exact catalog ports, runtime errors, and public signatures, start from the crate READMEs and
their linked source:

- [`wyrd-core`](../crates/wyrd-core/README.md)
- [`wyrd-graph`](../crates/wyrd-graph/README.md)
- [`wyrd-runtime`](../crates/wyrd-runtime/README.md)
- [`wyrd-bevy`](../crates/wyrd-bevy/README.md)
