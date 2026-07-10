# Coverage (100% line target)

Use `./scripts/coverage.sh` (`cargo-llvm-cov`). CI fails under **100% lines** for:

| Suite | Command |
| --- | --- |
| f32 core/graph/runtime | `cargo llvm-cov --workspace --exclude wyrd-bevy --fail-under-lines 100` |
| i32 core/graph/runtime | `… --no-default-features --features std,signal-i32` |
| serde-ron (graph) | `… -p wyrd-graph --features std,signal-f32,serde-ron` |
| serde-json (graph) | `… -p wyrd-graph --features std,signal-f32,serde-json` |
| bevy | `cargo llvm-cov -p wyrd-bevy --fail-under-lines 100` |

```bash
./scripts/coverage.sh          # f32
./scripts/coverage.sh --all    # f32 + i32 + serde + bevy
./scripts/coverage.sh --open
```

## Policy

- Prefer **behavior tests** (builder → bind → loom → outbox) over coverage-only stubs.
- Dual-check critical math under both `signal-f32` and `signal-i32`.
- Region coverage may stay below 100% where match arms or defensive OOB paths are partial; **line** coverage is the gate.
