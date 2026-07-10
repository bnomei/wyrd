//! Bevy headless host tick (sample → loom → apply) — f32 only.

use bevy::prelude::*;
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd_bevy::{
    apply_signal_bool, set_sense_bool, AndDoorBinding, Door, WyrdInstance, WyrdPlugin, WyrdSet,
    WyrdWorld,
};
use wyrd_core::KnotKind;
use wyrd_graph::Weave;

fn and_door_weave() -> Weave {
    let (b, pa) = Weave::builder("door")
        .knot("plate_a", KnotKind::signal_in())
        .unwrap();
    let (b, pb) = b.knot("plate_b", KnotKind::signal_in()).unwrap();
    let (b, _) = b.and2("both", pa, pb).unwrap();
    let (b, _) = b.knot("door", KnotKind::signal_out("door.open")).unwrap();
    b.wire_named("both", "out", "door", "in").build().unwrap()
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
    let Some(inst) = world.instances.get_mut(binding.instance) else {
        return;
    };
    set_sense_bool(inst, binding.plate_a, plates.a);
    set_sense_bool(inst, binding.plate_b, plates.b);
}

fn apply_door_component(
    binding: Res<AndDoorBinding>,
    world: Res<WyrdWorld>,
    mut q: Query<&mut Door>,
) {
    let Some(inst) = world.instances.get(binding.instance) else {
        return;
    };
    for mut door in &mut q {
        let _ = apply_signal_bool(inst, binding.door_path, &mut door.open);
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
    let binding = AndDoorBinding {
        plate_a: inst.sense_id("plate_a").unwrap(),
        plate_b: inst.sense_id("plate_b").unwrap(),
        door_path: inst.path_id("door.open").unwrap(),
        instance: 0,
    };
    app.world_mut()
        .resource_mut::<WyrdWorld>()
        .instances
        .push(inst);
    app.insert_resource(binding);
    app.world_mut().spawn(Door { open: false });
    app.insert_resource(PlateState { a: false, b: false });
    app
}

/// One Bevy `update()` with both plates high (door open path).
#[divan::bench]
fn bevy_door_tick_both(bencher: Bencher) {
    let mut app = setup_app();
    // Warmup one frame so schedules settle.
    app.world_mut().resource_mut::<PlateState>().a = true;
    app.world_mut().resource_mut::<PlateState>().b = true;
    app.update();
    // Weave: plate_a, plate_b, both, door → 4 knots (ItemsCount labels work, not pure loom ns).
    let knots = 4u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
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

/// Alternating plate patterns over updates (sample churn).
#[divan::bench]
fn bevy_door_tick_scripted(bencher: Bencher) {
    let mut app = setup_app();
    app.update();
    let mut phase = 0u8;
    let knots = 4u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        let (a, b) = match phase % 4 {
            0 => (false, false),
            1 => (true, false),
            2 => (true, true),
            _ => (false, true),
        };
        phase = phase.wrapping_add(1);
        app.world_mut().resource_mut::<PlateState>().a = a;
        app.world_mut().resource_mut::<PlateState>().b = b;
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

fn main() {
    divan::main();
}
