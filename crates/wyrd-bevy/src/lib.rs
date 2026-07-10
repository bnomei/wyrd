//! Thin Bevy bridge for Wyrd — no graph topology on Entities.
//!
//! Host tick order: [`WyrdSet::Sample`] → [`WyrdSet::Loom`] → [`WyrdSet::Apply`].
//! Bevy **Messages** are confirmations of host effects only — never Weave Threads.

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

/// Host-owned door state on an Entity (not a Wyrd Knot).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Door {
    pub open: bool,
}

/// Confirmation that a SignalOut level was applied by the host (VFX/UI only).
///
/// **Not** a Thread. Topology lives only in the Weave.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WyrdSignalConfirm {
    pub path: HostPathId,
    pub truthy: bool,
}

pub struct WyrdPlugin;

impl Plugin for WyrdPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WyrdWorld>()
            .add_message::<WyrdSignalConfirm>()
            .configure_sets(
                Update,
                (WyrdSet::Sample, WyrdSet::Loom, WyrdSet::Apply).chain(),
            )
            .add_systems(Update, loom_all.in_set(WyrdSet::Loom));
    }
}

/// Advance tick, clear outbox, loom every instance.
/// Host systems write senses in `WyrdSet::Sample` and read outbox in `WyrdSet::Apply`.
///
/// Loom is infallible after a successful bind (validate already ran).
pub fn loom_all(mut world: ResMut<WyrdWorld>) {
    for inst in world.instances.iter_mut() {
        inst.tick = inst.tick.wrapping_add(1);
        inst.runtime.begin_frame(HostTime { tick: inst.tick });
        let _ = inst.runtime.loom(&inst.weave);
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

/// Apply a SignalOut level into a host `bool`, returning `true` if the value changed.
pub fn apply_signal_bool(inst: &WyrdInstance, path: HostPathId, slot: &mut bool) -> bool {
    let next = signal_truthy(inst, path);
    let changed = *slot != next;
    *slot = next;
    changed
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

    fn apply_door_component(
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
            }
        }
    }

    #[test]
    fn headless_app_and_door() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(WyrdPlugin)
            .init_resource::<DoorOpen>();

        let weave = and_door_weave();
        let inst = WyrdInstance::new("demo", weave).unwrap();
        assert!(inst.sense_id("nope").is_none());
        assert!(inst.path_id("nope").is_none());
        let binding = AndDoorBinding {
            plate_a: inst.sense_id("plate_a").unwrap(),
            plate_b: inst.sense_id("plate_b").unwrap(),
            door_path: inst.path_id("door.open").unwrap(),
            instance: 0,
        };
        // Missing path → not truthy.
        assert!(!signal_truthy(&inst, HostPathId(99)));

        app.world_mut()
            .resource_mut::<WyrdWorld>()
            .instances
            .push(inst);
        app.insert_resource(binding);
        app.add_systems(Update, sample_plates.in_set(WyrdSet::Sample));
        app.add_systems(Update, apply_door.in_set(WyrdSet::Apply));

        // No PlateState yet — sample is a no-op; apply still runs.
        app.update();
        assert!(!app.world().resource::<DoorOpen>().0);

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

        // Bad instance index on binding → sample/apply early-return arms.
        app.insert_resource(AndDoorBinding {
            plate_a: binding.plate_a,
            plate_b: binding.plate_b,
            door_path: binding.door_path,
            instance: 99,
        });
        app.update();
    }

    #[derive(Resource, Default)]
    struct ConfirmLog(Vec<WyrdSignalConfirm>);

    fn log_confirms(
        mut reader: MessageReader<WyrdSignalConfirm>,
        mut log: ResMut<ConfirmLog>,
    ) {
        for m in reader.read() {
            log.0.push(*m);
        }
    }

    #[test]
    fn door_component_and_confirmation_message() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(WyrdPlugin)
            .init_resource::<ConfirmLog>();

        let weave = and_door_weave();
        let inst = WyrdInstance::new("demo", weave).unwrap();
        let binding = AndDoorBinding {
            plate_a: inst.sense_id("plate_a").unwrap(),
            plate_b: inst.sense_id("plate_b").unwrap(),
            door_path: inst.path_id("door.open").unwrap(),
            instance: 0,
        };
        let door_path = binding.door_path;

        app.world_mut()
            .resource_mut::<WyrdWorld>()
            .instances
            .push(inst);
        app.insert_resource(binding);
        app.world_mut().spawn(Door { open: false });
        app.add_systems(Update, sample_plates.in_set(WyrdSet::Sample));
        app.add_systems(Update, apply_door_component.in_set(WyrdSet::Apply));
        app.add_systems(Update, log_confirms.after(WyrdSet::Apply));

        app.world_mut().insert_resource(PlateState {
            a: true,
            b: true,
        });
        app.update();

        let door = app
            .world_mut()
            .query::<&Door>()
            .single(app.world())
            .expect("door entity");
        assert!(door.open);

        let log = app.world().resource::<ConfirmLog>();
        assert!(
            log.0.iter().any(|c| c.path == door_path && c.truthy),
            "expected WyrdSignalConfirm for door.open"
        );
    }

    #[test]
    fn bind_failure_on_empty_weave() {
        let empty = Weave {
            id: "e".into(),
            knots: vec![],
            threads: vec![],
            numeric: wyrd_core::NumericPath::compiled(),
        };
        assert!(WyrdInstance::new("bad", empty).is_err());
    }
}
