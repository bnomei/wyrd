# Make Specs Handoff: 001-v030-ergonomic-recipes

## Status

- research_id: 001-v030-ergonomic-recipes
- status: frozen
- intended_spec_slug: v030-ergonomic-recipes
- shape_review: GREEN
- cheap_worker_ready: yes

## Objective

Release Wyrd v0.3.0 as an additive ergonomic authoring and host-composition upgrade, with validated static and dynamic graph construction, typed generic/Bevy recipe bindings, agent-readable tool data, and release-ready documentation.

## Requirements Seed

- R001: WHEN a fixed graph is authored, THE SYSTEM SHALL support the existing validated `weave!` topology syntax across cookbook and Bevy examples.
- R002: WHEN a reusable fragment is authored, THE SYSTEM SHALL support validated `pattern!` declarations with named exports.
- R003: WHEN a host binds a recipe, THE SYSTEM SHALL resolve and retain typed ports without repeated ad-hoc lookup strings.
- R004: WHEN examples run scripted frames, THE SYSTEM SHALL provide typed scenario setup and expectations without changing current recipe behavior.
- R005: WHEN code generates topology, THE SYSTEM SHALL provide scoped typed composition while retaining full-catalog builder access.
- R006: WHEN tooling consumes a recipe, THE SYSTEM SHALL provide deterministic manifests and optional JSON Schema.
- R007: WHEN Bevy loads a recipe, THE SYSTEM SHALL register a typed instance without taking ownership of Sample or Apply systems.
- R008: WHEN v0.3.0 is prepared, THE SYSTEM SHALL preserve v0.2 compatibility and pass the repository release gates without publishing.

## Scope

In scope:
- Core authoring, recipe, cookbook, schema, Bevy, test, CI, docs, and release metadata changes described in the frozen implementation shape.

Out of scope:
- crates.io publish/tag, breaking v0.2 API/schema changes, editor UI, Bevy graph topology Entities, and core Bevy dependencies.

## Current-State Facts

- `crates/wyrd-for-games/src/authoring/macros.rs` already exposes validated `weave!`.
- `crates/wyrd-for-games/src/runtime_impl/cookbook/tier_a.rs` through `tier_d.rs` mix static topology with execution.
- `crates/wyrd-for-games-bevy/src/lib.rs` owns the explicit Sample/Loom/Apply boundary.
- `.github/workflows/ci.yml` defines the compatibility and release validation matrix.

## Decisions

- Use `weave!` for fixed topology, dedicated `pattern!` for reusable fragments, `Recipe` for host contracts, Scenario for teaching execution, Composer for generated topology, optional schema for tools, and a thin generic Bevy recipe plugin.
- Use serial Orchid tasks with independent review. Review fixes are separate commits and every commit is followed by `cargo clean`.

Rejected:
- A second static graph DSL, macro-only host bindings, mandatory schema dependencies, and a Bevy-owned topology model.

Open:
- None.

## Implementation Shape Excerpts

- Authoring changes live in `crates/wyrd-for-games/src/authoring/`; runtime recipe/scenario changes live in `crates/wyrd-for-games/src/runtime_impl/`; Bevy registration lives in `crates/wyrd-for-games-bevy/src/lib.rs`.
- Maintain owner-scoped Builder validation and use source-backed tests in `crates/wyrd-for-games/tests/`, the Bevy crate tests, and existing CI workflow.

## Suggested Spec Shape

- spec_kind: feature
- fanout_policy: serial
- execution_policy: auto-continue
- task_slices: T001 through T008, ordered exactly as the implementation shape.

## Validation

- Run focused tests after each clean boundary and the final CI-equivalent matrix, package dry-runs, and warning-free rustdoc. Request approved Cargo network access if dependencies are absent.

## Worker Context Policy

- Workers may read: authoring modules, runtime/cookbook modules, Bevy lib/example, target tests, manifests, CI, and README files named in their task context.
- Workers must not be sent to: raw/, broad current-state research, decision history, or rejected alternatives.
