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

#[test]
fn recipe_ports_resolve_all_named_endpoint_kinds_and_can_be_split() {
    let instance = DoorRecipe::bind().expect("recipe should bind");
    let (runtime, ports) = instance.into_parts();

    assert_eq!(runtime.required_sense("plate").unwrap(), ports.plate);
    assert_eq!(runtime.required_path("door.open").unwrap(), ports.door);
    assert_eq!(runtime.required_command("door-chime").unwrap(), ports.chime);
    assert_eq!(runtime.required_knot("plate").unwrap().get(), 0);

    assert!(matches!(
        runtime.required_sense("missing"),
        Err(RecipeResolveError::Missing { endpoint: RecipeEndpoint::SignalIn, name })
            if name == "missing"
    ));
    assert!(matches!(
        runtime.required_knot("missing"),
        Err(RecipeResolveError::Missing { endpoint: RecipeEndpoint::Knot, name })
            if name == "missing"
    ));
    assert!(matches!(
        runtime.required_path("missing"),
        Err(RecipeResolveError::Missing { endpoint: RecipeEndpoint::SignalOut, name })
            if name == "missing"
    ));
    assert!(matches!(
        runtime.required_command("missing"),
        Err(RecipeResolveError::Missing { endpoint: RecipeEndpoint::EmitCommand, name })
            if name == "missing"
    ));
}

#[test]
fn manifest_ignores_non_host_knots() {
    let mut builder = Weave::builder("recipe-manifest-constant").unwrap();
    builder
        .knot("constant", KnotKind::constant_bool(true))
        .unwrap();
    let manifest = RecipeManifest::from_weave(&builder.build().unwrap());

    assert!(manifest.signal_inputs.is_empty());
    assert!(manifest.signal_outputs.is_empty());
    assert!(manifest.emit_commands.is_empty());
}

#[test]
fn manifest_orders_duplicate_host_names_by_knot_id() {
    let weave = weave! {
        id: "recipe-manifest-order";
        knots {
            source = KnotKind::signal_in(SignalDomain::Bool);
            output_z = KnotKind::signal_out("z", SignalDomain::Bool);
            output_a_late = KnotKind::signal_out("a", SignalDomain::Bool);
            output_a_early = KnotKind::signal_out("a", SignalDomain::Bool);
            command_z = KnotKind::emit_command("z");
            command_a_late = KnotKind::emit_command("a");
            command_a_early = KnotKind::emit_command("a");
        }
        threads {
            source.out -> output_z.in;
            source.out -> output_a_late.in;
            source.out -> output_a_early.in;
            source.out -> command_z.trigger;
            source.out -> command_a_late.trigger;
            source.out -> command_a_early.trigger;
        }
    }
    .unwrap();
    let manifest = RecipeManifest::from_weave(&weave);

    assert_eq!(
        manifest
            .signal_outputs
            .iter()
            .map(|output| output.knot.as_str())
            .collect::<Vec<_>>(),
        ["output_a_early", "output_a_late", "output_z"]
    );
    assert_eq!(
        manifest
            .emit_commands
            .iter()
            .map(|command| command.knot.as_str())
            .collect::<Vec<_>>(),
        ["command_a_early", "command_a_late", "command_z"]
    );
}
