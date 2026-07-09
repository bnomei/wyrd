//! Thin Bevy bridge for Wyrd — no graph topology on Entities.

use bevy::prelude::*;
use wyrd_core::{HostPathId, HostTime, KnotId, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::Runtime;

/// Ordered host integration sets. Sample → Loom → Apply.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum WyrdSet {
    Sample,
    Loom,
    Apply,
}

/// One bound Weave + Runtime. Host owns sampling and applying outbox.
pub struct WyrdInstance {
    pub label: String,
    pub weave: Weave,
    pub runtime: Runtime,
    pub tick: u64,
}

impl WyrdInstance {
    pub fn new(label: impl Into<String>, weave: Weave) -> Result<Self, wyrd_core::WyrdError> {
        let runtime = Runtime::bind(&weave, Default::default())?;
        Ok(Self {
            label: label.into(),
            weave,
            runtime,
            tick: 0,
        })
    }

    pub fn sense_id(&self, name: &str) -> Option<KnotId> {
        self.runtime.sense_id(name)
    }

    pub fn path_id(&self, path: &str) -> Option<HostPathId> {
        self.runtime.path_id(path)
    }
}

/// All active Wyrd instances (dense ids already bound).
#[derive(Resource, Default)]
pub struct WyrdWorld {
    pub instances: Vec<WyrdInstance>,
}

/// Demo/test binding for the and-door sample (not a general host primitive).
/// Resolve your own `KnotId` / `HostPathId` fields at setup for production games.
#[derive(Resource, Clone, Copy, Debug)]
pub struct AndDoorBinding {
    pub plate_a: KnotId,
    pub plate_b: KnotId,
    pub door_path: HostPathId,
    pub instance: usize,
}

pub struct WyrdPlugin;

impl Plugin for WyrdPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WyrdWorld>()
            .configure_sets(
                Update,
                (WyrdSet::Sample, WyrdSet::Loom, WyrdSet::Apply).chain(),
            )
            .add_systems(Update, loom_all.in_set(WyrdSet::Loom));
    }
}

/// Advance tick, clear outbox, loom every instance.
/// Host systems write senses in `WyrdSet::Sample` and read outbox in `WyrdSet::Apply`.
pub fn loom_all(mut world: ResMut<WyrdWorld>) {
    for inst in world.instances.iter_mut() {
        inst.tick = inst.tick.wrapping_add(1);
        inst.runtime.begin_frame(HostTime { tick: inst.tick });
        if let Err(e) = inst.runtime.loom(&inst.weave) {
            // Settle should not fail on a validated weave; surface if it does.
            bevy::log::error!("wyrd loom failed ({}): {e}", inst.label);
            debug_assert!(false, "wyrd loom failed: {e}");
        }
    }
}

/// Write ONE/ZERO into a sense port (dense KnotId).
#[inline]
pub fn set_sense_bool(inst: &mut WyrdInstance, id: KnotId, on: bool) {
    let v = if on { ONE } else { ZERO };
    inst.runtime.port_writer().set_sense(id, v);
}

/// Read truthy SignalOut by HostPathId.
pub fn signal_truthy(inst: &WyrdInstance, path: HostPathId) -> bool {
    inst.runtime
        .outbox()
        .signals()
        .iter()
        .find(|s| s.path == path)
        .map(|s| wyrd_core::is_truthy(s.value))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wyrd_core::KnotKind;

    fn and_door_weave() -> Weave {
        let (b, pa) = Weave::builder("door")
            .knot("plate_a", KnotKind::signal_in())
            .unwrap();
        let (b, pb) = b.knot("plate_b", KnotKind::signal_in()).unwrap();
        let (b, _) = b.and2("both", pa, pb).unwrap();
        let (b, _) = b.knot("door", KnotKind::signal_out("door.open")).unwrap();
        b.wire_named("both", "out", "door", "in").build().unwrap()
    }

    #[derive(Resource, Default)]
    struct DoorOpen(bool);

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

    fn apply_door(binding: Res<AndDoorBinding>, world: Res<WyrdWorld>, mut door: ResMut<DoorOpen>) {
        let Some(inst) = world.instances.get(binding.instance) else {
            return;
        };
        door.0 = signal_truthy(inst, binding.door_path);
    }

    #[test]
    fn headless_app_and_door() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(WyrdPlugin)
            .init_resource::<DoorOpen>();

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
        app.add_systems(Update, sample_plates.in_set(WyrdSet::Sample));
        app.add_systems(Update, apply_door.in_set(WyrdSet::Apply));

        app.world_mut().insert_resource(PlateState {
            a: true,
            b: false,
        });
        app.update();
        assert!(!app.world().resource::<DoorOpen>().0);

        app.world_mut().insert_resource(PlateState {
            a: true,
            b: true,
        });
        app.update();
        assert!(app.world().resource::<DoorOpen>().0);
    }
}
