# Intake

Implement a release-ready Wyrd v0.3.0. The release must make the cookbook tiers and Bevy integration more ergonomic and composable through declarative static graphs, reusable patterns, typed recipes and bindings, dynamic composition, agent-readable manifests/schemas, and a thin Bevy registration layer.

Success means all requested layers are shipped additively, each implementation and review autofix lands in a separate commit, independent review is run, `cargo clean` runs between commits, and final validation is release-ready without publishing or tagging.

Non-goals: changing v0.2 public API behavior, changing serialized `WeaveDef`/`PatternDef` compatibility, coupling the core crate to Bevy, publishing crates, or tagging a release.
