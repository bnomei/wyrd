# Design Decisions

- Fixed topology uses the existing `weave!` macro. A second general-purpose static DSL is rejected because it would fragment docs, agent output, and validation behavior.
- Reusable fragments use a dedicated `pattern!` macro, rather than changing `weave!` to return two unrelated value types.
- A public engine-neutral `Recipe` trait owns `Weave` construction and typed runtime-port resolution. It preserves direct runtime APIs and makes generic host and Bevy setup share a contract.
- Cookbook execution gets a typed closure-based Scenario harness. Closures scope frame writes and output expectations; topology stays declarative.
- Generated graphs use `Weave::compose` with typed Bool/Level/Count wires plus a low-level full-catalog escape hatch. Global DAG correctness remains fallible final validation rather than typestate.
- `RecipeManifest` is derived deterministically from a validated weave and supplements, rather than replaces, `WeaveDef` as the full tooling IR.
- JSON Schema is opt-in through a `schema` feature that enables serde, std, and schemars. Default and no_std builds stay free of schema dependencies.
- Bevy receives `WyrdRecipePlugin<R>` and typed recipe-instance resources, but game systems remain explicitly responsible for Sample and Apply.
- v0.3.0 is release-ready only. The workspace, internal dependency requirement, docs, changelog, and publish workflow move to 0.3.0; no publication/tag is performed.
- Every implementation commit is reviewed independently. Verified fixes are separate commits, and `cargo clean` occurs after every commit.
