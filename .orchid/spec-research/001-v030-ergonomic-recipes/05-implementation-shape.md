# Implementation Shape

## Ownership

- `crates/wyrd-for-games/src/authoring/` owns `pattern!`, `Weave::compose`, typed composer handles, manifests, and feature-gated schema exports.
- `crates/wyrd-for-games/src/runtime_impl/` owns `Recipe`, bound recipe instances, recipe resolution errors, and cookbook Scenario support.
- `crates/wyrd-for-games/src/runtime_impl/cookbook/` owns declarative recipe topology, typed recipe ports, and executable teaching scenarios.
- `crates/wyrd-for-games-bevy/src/lib.rs` owns recipe plugin/resource insertion while retaining the existing Sample/Loom/Apply sets.

## Public contracts

- `pattern!` has `id`, `knots`, `exports { input name = knot.port; output name = knot.port; }`, and `threads` blocks and returns `Result<Pattern, BuildError>`.
- `Recipe` has associated `Ports`, `weave()`, `resolve(&Runtime)`, a default bind path returning `RecipeInstance<Self>`, and deterministic `manifest()`.
- `RecipeInstance<R>` owns `Runtime` and `R::Ports`; resolution errors identify the recipe and missing/invalid author endpoint.
- `RecipeManifest` lists weave ID, numeric path, input endpoints, signal outputs, and command outputs. It derives serde and derives `JsonSchema` only under `schema`.
- `Weave::compose` receives a scoped `Composer`; its typed wires prevent fixed-domain connections from crossing Bool/Level/Count. Generic knot/thread operations preserve full catalog access.
- `WyrdRecipePlugin<R>` binds a recipe during startup and inserts `WyrdRecipeInstance<R> { instance, ports }`; it does not schedule host sampling or effect application.

## Slices

1. Declare cookbook and Bevy static topology with `weave!`; extract reusable `*_weave` functions.
2. Add `pattern!` and convert the monostable Pattern recipe.
3. Add Recipe ports, binding, errors, and manifests; convert cookbook contracts.
4. Add Scenario and migrate runnable recipes.
5. Add Composer and typed wires with parity/error tests.
6. Add schema feature and CI feature coverage.
7. Add Bevy recipe registration and refactor `and_door`.
8. Prepare v0.3.0 docs, metadata, workflow, and package validation.

## Tests and constraints

- Keep existing behavior and public APIs intact; add macro parity/UI tests, recipe resolution failures, scenario behavior, composer domains/fan-out/pattern inclusion, deterministic manifest/schema tests, and Bevy lifecycle tests.
- Run narrow validation per slice after `cargo clean`; final gates mirror `.github/workflows/ci.yml`, package dry-runs, and warning-free rustdoc.
- Core additions must compile with both signal paths and `alloc`/no_std where their dependencies permit. Bevy remains signal-f32 only.
