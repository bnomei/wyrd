//! Feature-gated JSON Schema contracts for recipe tooling.

#![cfg(feature = "schema")]

use wyrd::{schema_for, PatternDef, RecipeManifest, WeaveDef};

#[test]
fn recipe_manifest_schema_describes_stable_endpoint_contracts() {
    let schema = schema_for!(RecipeManifest);
    let object = schema
        .schema
        .object
        .as_ref()
        .expect("manifest schema must describe an object");

    assert!(object.properties.contains_key("weave_id"));
    assert!(object.properties.contains_key("numeric"));
    assert!(object.properties.contains_key("signal_inputs"));
    assert!(object.properties.contains_key("signal_outputs"));
    assert!(object.properties.contains_key("emit_commands"));
    assert!(schema.definitions.contains_key("SignalInManifest"));
    assert!(schema.definitions.contains_key("SignalOutManifest"));
    assert!(schema.definitions.contains_key("EmitCommandManifest"));
}

#[test]
fn authoring_definition_schemas_expose_graph_and_pattern_shape() {
    let weave = schema_for!(WeaveDef);
    let weave_object = weave
        .schema
        .object
        .as_ref()
        .expect("weave schema must describe an object");
    assert!(weave_object.properties.contains_key("id"));
    assert!(weave_object.properties.contains_key("knots"));
    assert!(weave_object.properties.contains_key("threads"));
    assert!(weave.definitions.contains_key("KnotDef"));
    assert!(weave.definitions.contains_key("ThreadDef"));

    let pattern = schema_for!(PatternDef);
    let pattern_object = pattern
        .schema
        .object
        .as_ref()
        .expect("pattern schema must describe an object");
    assert!(pattern_object.properties.contains_key("inner"));
    assert!(pattern_object.properties.contains_key("inputs"));
    assert!(pattern_object.properties.contains_key("outputs"));
    assert!(pattern.definitions.contains_key("PatternExportDef"));
}
