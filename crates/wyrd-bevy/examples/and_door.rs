//! Headless and-door demo: two plates → And → door.open.
//!
//! ```bash
//! cargo run -p wyrd-bevy --example and_door
//! ```

use bevy::prelude::*;
use wyrd_bevy::{
    set_sense_bool, signal_truthy, AndDoorBinding, WyrdInstance, WyrdPlugin, WyrdSet, WyrdWorld,
};
use wyrd_core::KnotKind;
use wyrd_graph::Weave;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_millis(1),
        )))
        .add_plugins(WyrdPlugin)
        .init_resource::<DoorOpen>()
        .insert_resource(PlateState {
            a: false,
            b: false,
            frame: 0,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, drive_plates.in_set(WyrdSet::Sample))
        .add_systems(Update, apply_door.in_set(WyrdSet::Apply))
        .add_systems(Update, quit_after.in_set(WyrdSet::Apply))
        .run();
}

#[derive(Resource, Default)]
struct DoorOpen(bool);

#[derive(Resource)]
struct PlateState {
    a: bool,
    b: bool,
    frame: u32,
}

fn setup(mut world: ResMut<WyrdWorld>, mut commands: Commands) {
    let weave = and_door_weave();
    let inst = WyrdInstance::new("and_door", weave).expect("bind weave");
    let binding = AndDoorBinding {
        plate_a: inst.sense_id("plate_a").expect("plate_a"),
        plate_b: inst.sense_id("plate_b").expect("plate_b"),
        door_path: inst.path_id("door.open").expect("door.open"),
        instance: 0,
    };
    world.instances.push(inst);
    commands.insert_resource(binding);
    eprintln!("wyrd-bevy and_door: frames 1–2 A only, 3–4 both plates");
}

fn and_door_weave() -> Weave {
    let (b, pa) = Weave::builder("door")
        .knot("plate_a", KnotKind::signal_in())
        .unwrap();
    let (b, pb) = b.knot("plate_b", KnotKind::signal_in()).unwrap();
    let (b, _) = b.and2("both", pa, pb).unwrap();
    let (b, _) = b.knot("door", KnotKind::signal_out("door.open")).unwrap();
    b.wire_named("both", "out", "door", "in").build().unwrap()
}

fn drive_plates(
    mut plates: ResMut<PlateState>,
    binding: Res<AndDoorBinding>,
    mut world: ResMut<WyrdWorld>,
) {
    plates.frame = plates.frame.wrapping_add(1);
    plates.a = plates.frame >= 1;
    plates.b = plates.frame >= 3;

    let Some(inst) = world.instances.get_mut(binding.instance) else {
        return;
    };
    set_sense_bool(inst, binding.plate_a, plates.a);
    set_sense_bool(inst, binding.plate_b, plates.b);
}

fn apply_door(binding: Res<AndDoorBinding>, world: Res<WyrdWorld>, mut door: ResMut<DoorOpen>) {
    let Some(inst) = world.instances.get(binding.instance) else {
        return;
    };
    let open = signal_truthy(inst, binding.door_path);
    if open != door.0 {
        door.0 = open;
        eprintln!("door.open = {open}");
    }
}

fn quit_after(plates: Res<PlateState>, mut exit: MessageWriter<AppExit>) {
    if plates.frame >= 5 {
        exit.write(AppExit::Success);
    }
}
