//! Bevy headless host tick (sample → loom → apply) — f32 only.

use bevy::prelude::*;
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd::core::{KnotKind, SignalDomain};
use wyrd::graph::Weave;
use wyrd_bevy::{
    apply_signal_bool, set_sense_bool, AndDoorBinding, Door, WyrdInstance, WyrdPlugin, WyrdSet,
    WyrdWorld,
};

fn and_door_weave() -> Weave {
    let mut b = Weave::builder("door").unwrap();
    let pa = b
        .knot("plate_a", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let pb = b
        .knot("plate_b", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let both = b.knot("both", KnotKind::and2()).unwrap();
    let door = b
        .knot(
            "door",
            KnotKind::signal_out("door.open", SignalDomain::Bool),
        )
        .unwrap();
    let from = b.output(&pa, "out").unwrap();
    let to = b.input(&both, "in_0").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&pb, "out").unwrap();
    let to = b.input(&both, "in_1").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&both, "out").unwrap();
    let to = b.input(&door, "in").unwrap();
    b.connect(from, to).unwrap();
    b.build().unwrap()
}

#[derive(Resource, Clone, Copy)]
struct PlateState {
    a: bool,
    b: bool,
}

fn sample_plates(
    plates: Option<Res<PlateState>>,
    binding: Res<AndDoorBinding>,
    mut world: ResMut<WyrdWorld>,
) {
    let Some(plates) = plates else {
        return;
    };
    let Some(inst) = world.get_mut(binding.instance) else {
        return;
    };
    set_sense_bool(inst, binding.plate_a, plates.a).expect("bound plate_a handle");
    set_sense_bool(inst, binding.plate_b, plates.b).expect("bound plate_b handle");
}

fn apply_door_component(
    binding: Res<AndDoorBinding>,
    world: Res<WyrdWorld>,
    mut q: Query<&mut Door>,
) {
    let Some(inst) = world.get(binding.instance) else {
        return;
    };
    for mut door in &mut q {
        let _ =
            apply_signal_bool(inst, binding.door_path, &mut door.open).expect("bound door path");
    }
}

fn setup_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(WyrdPlugin)
        .add_systems(Update, sample_plates.in_set(WyrdSet::Sample))
        .add_systems(Update, apply_door_component.in_set(WyrdSet::Apply));

    let weave = and_door_weave();
    let inst = WyrdInstance::new("demo", weave).unwrap();
    let plate_a = inst.sense_id("plate_a").unwrap();
    let plate_b = inst.sense_id("plate_b").unwrap();
    let door_path = inst.path_id("door.open").unwrap();
    let instance = app.world_mut().resource_mut::<WyrdWorld>().insert(inst);
    let binding = AndDoorBinding {
        plate_a,
        plate_b,
        door_path,
        instance,
    };
    app.insert_resource(binding);
    app.world_mut().spawn(Door { open: false });
    app.insert_resource(PlateState { a: false, b: false });
    app
}

/// One Bevy `update()` with both plates high (door open path).
#[divan::bench]
fn bevy_door_tick_both(bencher: Bencher) {
    let mut app = setup_app();
    app.world_mut().resource_mut::<PlateState>().a = true;
    app.world_mut().resource_mut::<PlateState>().b = true;
    app.update();
    assert!(app
        .world_mut()
        .query::<&Door>()
        .single(app.world())
        .map(|d| d.open)
        .unwrap_or(false));
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        app.world_mut().resource_mut::<PlateState>().a = true;
        app.world_mut().resource_mut::<PlateState>().b = true;
        app.update();
        let open = app
            .world_mut()
            .query::<&Door>()
            .single(app.world())
            .map(|d| d.open)
            .unwrap_or(false);
        black_box(open);
    });
}

/// A complete closed/open/closed plate cycle over four Bevy updates.
#[divan::bench]
fn bevy_door_scripted_cycle(bencher: Bencher) {
    let mut app = setup_app();
    app.update();
    bencher.counter(ItemsCount::new(4u64)).bench_local(|| {
        let mut observed = [false; 4];
        for (slot, (a, b)) in
            observed
                .iter_mut()
                .zip([(false, false), (true, false), (true, true), (false, true)])
        {
            app.world_mut().insert_resource(PlateState { a, b });
            app.update();
            *slot = app
                .world_mut()
                .query::<&Door>()
                .single(app.world())
                .map(|d| d.open)
                .unwrap_or(false);
        }
        black_box(observed);
    });
}

fn main() {
    divan::main();
}
