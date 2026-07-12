//! And-door demo: two plates → And → SignalOut; **host** applies to a Door component.
//!
//! The door is **not** a Wyrd Knot — only a host effect. Bevy Messages confirm
//! apply (VFX/UI), they are never Weave Threads.
//!
//! ```bash
//! cargo run -p wyrd-for-games-bevy --example and_door
//! ```

use bevy::prelude::*;
use wyrd::core::{KnotKind, SignalDomain};
use wyrd::graph::Weave;
use wyrd::runtime::{Recipe, RecipeResolveError, Runtime};
use wyrd::{weave, BuildError, HostPathId, SenseId};
use wyrd_bevy::{
    apply_signal_bool, set_sense_bool, Door, WyrdPlugin, WyrdRecipeInstance, WyrdRecipePlugin,
    WyrdSet, WyrdSignalConfirm, WyrdWorld,
};

fn main() {
    App::new()
        .add_plugins(
            MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
                std::time::Duration::from_millis(1),
            )),
        )
        .add_plugins((WyrdPlugin, WyrdRecipePlugin::<AndDoorRecipe>::default()))
        .insert_resource(PlateState {
            a: false,
            b: false,
            frame: 0,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, drive_plates.in_set(WyrdSet::Sample))
        .add_systems(Update, apply_door.in_set(WyrdSet::Apply))
        .add_systems(Update, log_confirms.after(WyrdSet::Apply))
        .add_systems(Update, quit_after.in_set(WyrdSet::Apply))
        .run();
}

#[derive(Resource)]
struct PlateState {
    a: bool,
    b: bool,
    frame: u32,
}

fn setup(mut commands: Commands) {
    commands.spawn(Door { open: false });
    eprintln!(
        "wyrd-for-games-bevy and_door: host Door component; frames 1–2 A only, 3–4 both plates"
    );
}

struct AndDoorRecipe;

struct AndDoorPorts {
    plate_a: SenseId,
    plate_b: SenseId,
    door_path: HostPathId,
}

impl Recipe for AndDoorRecipe {
    type Ports = AndDoorPorts;

    fn weave() -> Result<Weave, BuildError> {
        and_door_weave()
    }

    fn resolve_ports(runtime: &Runtime) -> Result<Self::Ports, RecipeResolveError> {
        Ok(AndDoorPorts {
            plate_a: runtime.required_sense("plate_a")?,
            plate_b: runtime.required_sense("plate_b")?,
            door_path: runtime.required_path("door.open")?,
        })
    }
}

fn and_door_weave() -> Result<Weave, BuildError> {
    weave! {
        id: "door";
        knots {
            plate_a = KnotKind::signal_in(SignalDomain::Bool);
            plate_b = KnotKind::signal_in(SignalDomain::Bool);
            both = KnotKind::and2();
            door = KnotKind::signal_out("door.open", SignalDomain::Bool);
        }
        threads {
            plate_a.out -> both.in_0;
            plate_b.out -> both.in_1;
            both.out -> door.in;
        }
    }
}

fn drive_plates(
    mut plates: ResMut<PlateState>,
    recipe: Res<WyrdRecipeInstance<AndDoorRecipe>>,
    mut world: ResMut<WyrdWorld>,
) {
    plates.frame = plates.frame.wrapping_add(1);
    plates.a = plates.frame >= 1;
    plates.b = plates.frame >= 3;

    let Some((ports, inst)) = recipe.get_mut(&mut world) else {
        return;
    };
    set_sense_bool(inst, ports.plate_a, plates.a).expect("bound plate_a handle");
    set_sense_bool(inst, ports.plate_b, plates.b).expect("bound plate_b handle");
}

fn apply_door(
    recipe: Res<WyrdRecipeInstance<AndDoorRecipe>>,
    world: Res<WyrdWorld>,
    mut q: Query<&mut Door>,
    mut confirms: MessageWriter<WyrdSignalConfirm>,
) {
    let Some((ports, inst)) = recipe.get(&world) else {
        return;
    };
    for mut door in &mut q {
        if apply_signal_bool(inst, ports.door_path, &mut door.open).expect("bound door path") {
            confirms.write(WyrdSignalConfirm {
                path: ports.door_path,
                truthy: door.open,
            });
            eprintln!("host applied Door.open = {}", door.open);
        }
    }
}

fn log_confirms(mut reader: MessageReader<WyrdSignalConfirm>) {
    for c in reader.read() {
        eprintln!(
            "confirmation (not a Thread): path={:?} truthy={}",
            c.path, c.truthy
        );
    }
}

fn quit_after(plates: Res<PlateState>, mut exit: MessageWriter<AppExit>) {
    if plates.frame >= 6 {
        exit.write(AppExit::Success);
    }
}
