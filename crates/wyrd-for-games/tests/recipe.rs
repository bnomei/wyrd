use wyrd::{
    weave, BindOpts, BuildError, CmdId, HostPathId, KnotKind, Recipe, RecipeEndpoint, RecipeError,
    RecipeManifest, RecipeResolveError, Runtime, SenseId, SignalDomain, Weave,
};

struct DoorPorts {
    plate: SenseId,
    door: HostPathId,
    chime: CmdId,
}

struct DoorRecipe;

impl Recipe for DoorRecipe {
    type Ports = DoorPorts;

    fn weave() -> Result<Weave, BuildError> {
        weave! {
            id: "recipe-door";
            knots {
                plate = KnotKind::signal_in(SignalDomain::Bool);
                door = KnotKind::signal_out("door.open", SignalDomain::Bool);
                chime = KnotKind::emit_command("door-chime");
            }
            threads {
                plate.out -> door.in;
                plate.out -> chime.trigger;
            }
        }
    }

    fn resolve_ports(runtime: &Runtime) -> Result<Self::Ports, RecipeResolveError> {
        Ok(DoorPorts {
            plate: runtime.required_sense("plate")?,
            door: runtime.required_path("door.open")?,
            chime: runtime.required_command("door-chime")?,
        })
    }
}

struct InvalidRecipe;

impl Recipe for InvalidRecipe {
    type Ports = SenseId;

    fn weave() -> Result<Weave, BuildError> {
        DoorRecipe::weave()
    }

    fn resolve_ports(runtime: &Runtime) -> Result<Self::Ports, RecipeResolveError> {
        runtime.required_sense("door")
    }
}

#[test]
fn recipe_binds_and_retains_typed_ports() {
    let _: Option<wyrd::graph::RecipeManifest> = None;
    let _: Option<wyrd::runtime::RecipeInstance<DoorRecipe>> = None;

    let instance = DoorRecipe::bind_with(BindOpts::default()).expect("recipe should bind");

    assert!(instance.runtime().sense_id("plate").is_some());
    assert_eq!(
        instance.ports().plate,
        instance.runtime().sense_id("plate").unwrap()
    );
    assert_eq!(
        instance.ports().door,
        instance.runtime().path_id("door.open").unwrap()
    );
    assert_eq!(
        instance.ports().chime,
        instance.runtime().cmd_id("door-chime").unwrap()
    );
}

#[test]
fn recipe_resolution_error_names_the_invalid_required_endpoint() {
    assert!(matches!(
        InvalidRecipe::bind(),
        Err(RecipeError::Resolve(RecipeResolveError::Invalid {
            endpoint: RecipeEndpoint::SignalIn,
            name,
            reason: "the knot is not a SignalIn",
        })) if name == "door"
    ));
}

#[test]
fn manifest_is_derived_from_topology_in_stable_order() {
    let first = DoorRecipe::weave().unwrap();
    let second = weave! {
        id: "recipe-door";
        knots {
            chime = KnotKind::emit_command("door-chime");
            door = KnotKind::signal_out("door.open", SignalDomain::Bool);
            plate = KnotKind::signal_in(SignalDomain::Bool);
        }
        threads {
            plate.out -> door.in;
            plate.out -> chime.trigger;
        }
    }
    .unwrap();

    let manifest = RecipeManifest::from_weave(&first);
    assert_eq!(manifest, RecipeManifest::from_weave(&second));
    assert_eq!(manifest.weave_id, "recipe-door");
    assert_eq!(manifest.signal_inputs.len(), 1);
    assert_eq!(manifest.signal_inputs[0].knot, "plate");
    assert_eq!(manifest.signal_outputs[0].path, "door.open");
    assert_eq!(manifest.emit_commands[0].name, "door-chime");
    assert_eq!(DoorRecipe::manifest().unwrap(), manifest);
}
