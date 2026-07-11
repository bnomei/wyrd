# Design

Static topology uses the existing `weave!` lowering. `pattern!` lowers to the same Pattern data model. `Recipe` wraps a weave plus typed resolved runtime ports; `RecipeManifest` derives endpoint contracts from a validated weave. Scenario is an execution helper, and Composer is the generated-topology layer with typed domains plus raw escape hatches.

The Bevy crate consumes `Recipe` through a generic plugin/resource but keeps systems explicit around its existing Sample/Loom/Apply schedule. The core remains no_std-friendly; schema is an optional std/serde/schemars feature.

All tasks run serially because they share public contracts and Cargo outputs. Each completed task receives a fresh independent validator before the main commit. After that commit a separate source-visible bug review runs; any verified repair is a separate fix commit followed by another independent validator. Run `cargo clean` after every commit.
