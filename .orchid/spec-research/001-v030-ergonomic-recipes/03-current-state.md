# Current State

- `crates/wyrd-for-games/src/authoring/macros.rs` exports `weave!`; it lowers to `WeaveBuilder`, evaluates expressions once, rejects duplicate macro bindings at compile time, and preserves runtime builder validation.
- `crates/wyrd-for-games/src/authoring/builder.rs` owns validated incremental graph construction with owner-scoped knot and endpoint handles, domain checks, Pattern inclusion, and final `Weave` validation.
- `crates/wyrd-for-games/src/authoring/weave.rs` defines serializable `WeaveDef` and immutable validated `Weave`; `pattern.rs` defines equivalent Pattern data and validation.
- `crates/wyrd-for-games/src/runtime_impl/cookbook/tier_a.rs` through `tier_d.rs` manually construct every static graph, then mix binding, string handle lookup, tick driving, and assertions in `run_*` functions.
- `crates/wyrd-for-games-bevy/src/lib.rs` provides `WyrdPlugin`, `WyrdWorld`, `WyrdInstance`, and ordered `WyrdSet::Sample`, `Loom`, `Apply`; it intentionally keeps graph topology outside Entities.
- `crates/wyrd-for-games-bevy/examples/and_door.rs` duplicates the manual two-plate door graph and resolves names into an application binding resource.
- `Cargo.toml` defines workspace version `0.2.0`, core MSRV 1.75, core optional serde/RON/JSON features, and a Bevy 0.19 companion crate. Core is `#![no_std]` with alloc-backed collections.
- `.github/workflows/ci.yml` validates workspace warnings/tests, f32/i32 and codec feature matrices, Rust 1.75 no_std builds, Bevy, coverage, package contents, and warning-free rustdoc.
