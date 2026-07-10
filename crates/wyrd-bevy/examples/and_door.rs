//! And-door demo: two plates → And → SignalOut; **host** applies to a Door component.
//!
//! The door is **not** a Wyrd Knot — only a host effect. Bevy Messages confirm
//! apply (VFX/UI), they are never Weave Threads.
//!
//! ```bash
//! cargo run -p wyrd-bevy --example and_door
//! ```

use bevy::prelude::*;
use wyrd_bevy::{
    apply_signal_bool, set_sense_bool, AndDoorBinding, Door, WyrdInstance, WyrdPlugin,
    WyrdSet, WyrdSignalConfirm, WyrdWorld,
};
use wyrd_core::KnotKind;
use wyrd_graph::Weave;

fn main() {
    App::new()
        .add_plugins(MinimalPlugins.set(bevy::app::ScheduleRunnerPlugin::run_loop(
            std::time::Duration::from_millis(1),
        )))
        .add_plugins(WyrdPlugin)
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
    // Host-owned door entity (not a Weave knot).
    commands.spawn(Door { open: false });
    eprintln!("wyrd-bevy and_door: host Door component; frames 1–2 A only, 3–4 both plates");
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

fn apply_door(
    binding: Res<AndDoorBinding>,
    world: Res<WyrdWorld>,
    mut q: Query<&mut Door>,
    mut confirms: MessageWriter<WyrdSignalConfirm>,
) {
    let Some(inst) = world.instances.get(binding.instance) else {
        return;
    };
    for mut door in &mut q {
        if apply_signal_bool(inst, binding.door_path, &mut door.open) {
            confirms.write(WyrdSignalConfirm {
                path: binding.door_path,
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
