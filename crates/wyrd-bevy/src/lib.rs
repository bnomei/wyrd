//! Thin Bevy 0.18 bridge for Wyrd — no graph topology on Entities.
//!
//! Host tick order: [`WyrdSet::Sample`] → [`WyrdSet::Loom`] → [`WyrdSet::Apply`].
//! The plugin only drives loom; games own sample/apply systems. Bevy
//! **Messages** (`WyrdSignalConfirm`) are post-apply VFX/UI confirmations —
//! never Weave Threads. f32 signal path only.

use bevy::prelude::*;
use core::sync::atomic::{AtomicUsize, Ordering};
use wyrd_core::{HostTime, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{BindError, HandleError, HostPathId, Outbox, Runtime, SenseId};

/// Ordered host integration sets. Sample → Loom → Apply.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum WyrdSet {
    /// Host writes dense senses from world state.
    Sample,
    /// Plugin-driven settle (`loom_all`).
    Loom,
    /// Host applies outbox to components / world effects.
    Apply,
}

/// One bound Runtime. Host owns sampling and applying outbox.
pub struct WyrdInstance {
    label: String,
    runtime: Runtime,
    tick: u64,
}

impl WyrdInstance {
    /// Bind `weave` with default opts under a host-visible label.
    pub fn new(label: impl Into<String>, weave: Weave) -> Result<Self, BindError> {
        let runtime = Runtime::bind(weave, Default::default())?;
        Ok(Self {
            label: label.into(),
            runtime,
            tick: 0,
        })
    }

    /// Host-visible instance label (not a weave id).
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Monotonic frame counter advanced by [`loom_all`].
    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    pub fn outbox(&self) -> Outbox<'_> {
        self.runtime.outbox()
    }

    /// Resolve a `SignalIn` name once at setup (not each sample).
    pub fn sense_id(&self, name: &str) -> Option<SenseId> {
        self.runtime.sense_id(name)
    }

    /// Resolve a `SignalOut` path once at setup (not each apply).
    pub fn path_id(&self, path: &str) -> Option<HostPathId> {
        self.runtime.path_id(path)
    }
}

/// Stable, opaque handle to an instance stored in [`WyrdWorld`].
///
/// Handles are generational: removing an instance permanently invalidates its
/// handle, even if a later insertion reuses the same storage slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WyrdInstanceId {
    owner: usize,
    index: usize,
    generation: u64,
}

struct WyrdInstanceSlot {
    generation: u64,
    instance: Option<WyrdInstance>,
    next_free: Option<usize>,
}

// Monotonic owner identity prevents handles from crossing WyrdWorld resources.
static NEXT_WORLD_OWNER: AtomicUsize = AtomicUsize::new(1);

fn next_world_owner() -> usize {
    NEXT_WORLD_OWNER
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |owner| {
            owner.checked_add(1)
        })
        .expect("WyrdWorld owner token space exhausted")
}

/// All active Wyrd instances, addressed through stable generational handles.
#[derive(Resource)]
pub struct WyrdWorld {
    owner: usize,
    slots: Vec<WyrdInstanceSlot>,
    free_head: Option<usize>,
    len: usize,
}

impl Default for WyrdWorld {
    fn default() -> Self {
        Self {
            owner: next_world_owner(),
            slots: Vec::new(),
            free_head: None,
            len: 0,
        }
    }
}

impl WyrdWorld {
    /// Insert an instance and return its stable handle.
    pub fn insert(&mut self, instance: WyrdInstance) -> WyrdInstanceId {
        let (index, generation) = if let Some(index) = self.free_head {
            let slot = &mut self.slots[index];
            self.free_head = slot.next_free.take();
            debug_assert!(slot.instance.is_none());
            slot.instance = Some(instance);
            (index, slot.generation)
        } else {
            let index = self.slots.len();
            self.slots.push(WyrdInstanceSlot {
                generation: 0,
                instance: Some(instance),
                next_free: None,
            });
            (index, 0)
        };
        self.len += 1;
        WyrdInstanceId {
            owner: self.owner,
            index,
            generation,
        }
    }

    /// Remove and return an instance when `id` is still current.
    ///
    /// A slot whose generation reaches [`u64::MAX`] is retired after removal
    /// instead of wrapping around and making an ancient handle valid again.
    pub fn remove(&mut self, id: WyrdInstanceId) -> Option<WyrdInstance> {
        if id.owner != self.owner {
            return None;
        }
        let slot = self.slots.get_mut(id.index)?;
        if slot.generation != id.generation {
            return None;
        }
        let instance = slot.instance.take()?;
        self.len -= 1;

        if let Some(generation) = slot.generation.checked_add(1) {
            slot.generation = generation;
            slot.next_free = self.free_head;
            self.free_head = Some(id.index);
        } else {
            slot.next_free = None;
        }

        Some(instance)
    }

    /// Borrow an instance when `id` is still current.
    pub fn get(&self, id: WyrdInstanceId) -> Option<&WyrdInstance> {
        if id.owner != self.owner {
            return None;
        }
        let slot = self.slots.get(id.index)?;
        if slot.generation == id.generation {
            slot.instance.as_ref()
        } else {
            None
        }
    }

    /// Mutably borrow an instance when `id` is still current.
    pub fn get_mut(&mut self, id: WyrdInstanceId) -> Option<&mut WyrdInstance> {
        if id.owner != self.owner {
            return None;
        }
        let slot = self.slots.get_mut(id.index)?;
        if slot.generation == id.generation {
            slot.instance.as_mut()
        } else {
            None
        }
    }

    /// Iterate over active instance handles and shared references.
    pub fn iter(&self) -> impl Iterator<Item = (WyrdInstanceId, &WyrdInstance)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            slot.instance.as_ref().map(|instance| {
                (
                    WyrdInstanceId {
                        owner: self.owner,
                        index,
                        generation: slot.generation,
                    },
                    instance,
                )
            })
        })
    }

    /// Iterate over active instance handles and mutable references.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (WyrdInstanceId, &mut WyrdInstance)> {
        let owner = self.owner;
        self.slots
            .iter_mut()
            .enumerate()
            .filter_map(move |(index, slot)| {
                let generation = slot.generation;
                slot.instance.as_mut().map(|instance| {
                    (
                        WyrdInstanceId {
                            owner,
                            index,
                            generation,
                        },
                        instance,
                    )
                })
            })
    }

    /// Number of active instances.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Whether there are no active instances.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

/// Demo/test binding for the and-door sample (not a general host primitive).
/// Resolve your own `SenseId` / `HostPathId` fields at setup for production games.
#[derive(Resource, Clone, Copy, Debug)]
pub struct AndDoorBinding {
    pub plate_a: SenseId,
    pub plate_b: SenseId,
    pub door_path: HostPathId,
    pub instance: WyrdInstanceId,
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

/// Registers [`WyrdWorld`], confirmation messages, ordered sets, and `loom_all`.
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
    for (_, inst) in world.iter_mut() {
        inst.tick = inst.tick.wrapping_add(1);
        inst.runtime.begin_frame(HostTime { tick: inst.tick });
        inst.runtime.loom();
    }
}

/// Write ONE/ZERO into a sense port through its dense [`SenseId`].
#[inline]
pub fn set_sense_bool(inst: &mut WyrdInstance, id: SenseId, on: bool) -> Result<(), HandleError> {
    let v = if on { ONE } else { ZERO };
    inst.runtime.port_writer().set_sense(id, v)
}

/// Read truthy SignalOut by HostPathId.
///
/// Returns [`HandleError::ForeignRuntime`] when `path` belongs to another
/// instance, and [`HandleError::InvalidHostPath`] for an invalid local path.
pub fn signal_truthy(inst: &WyrdInstance, path: HostPathId) -> Result<bool, HandleError> {
    inst.runtime.path_name(path)?;
    Ok(inst
        .runtime
        .outbox()
        .signals()
        .iter()
        .find(|s| s.path == path)
        .map(|s| wyrd_core::is_truthy(s.value))
        .unwrap_or(false))
}

/// Apply a SignalOut level into a host `bool`, returning `true` if the value changed.
///
/// Returns the same handle validation errors as [`signal_truthy`] and leaves
/// `slot` unchanged when validation fails.
pub fn apply_signal_bool(
    inst: &WyrdInstance,
    path: HostPathId,
    slot: &mut bool,
) -> Result<bool, HandleError> {
    let next = signal_truthy(inst, path)?;
    let changed = *slot != next;
    *slot = next;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wyrd_core::KnotKind;

    fn and_door_weave() -> Weave {
        let mut b = Weave::builder("door").unwrap();
        let pa = b.knot("plate_a", KnotKind::signal_in()).unwrap();
        let pb = b.knot("plate_b", KnotKind::signal_in()).unwrap();
        let both = b.knot("both", KnotKind::and2()).unwrap();
        let door = b.knot("door", KnotKind::signal_out("door.open")).unwrap();
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
        let Some(inst) = world.get_mut(binding.instance) else {
            return;
        };
        set_sense_bool(inst, binding.plate_a, plates.a).expect("bound plate_a handle");
        set_sense_bool(inst, binding.plate_b, plates.b).expect("bound plate_b handle");
    }

    fn apply_door(binding: Res<AndDoorBinding>, world: Res<WyrdWorld>, mut door: ResMut<DoorOpen>) {
        let Some(inst) = world.get(binding.instance) else {
            return;
        };
        door.0 = signal_truthy(inst, binding.door_path).expect("bound door path");
    }

    fn apply_door_component(
        binding: Res<AndDoorBinding>,
        world: Res<WyrdWorld>,
        mut q: Query<&mut Door>,
        mut confirms: MessageWriter<WyrdSignalConfirm>,
    ) {
        let Some(inst) = world.get(binding.instance) else {
            return;
        };
        for mut door in &mut q {
            if apply_signal_bool(inst, binding.door_path, &mut door.open).expect("bound door path")
            {
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
        let plate_a = inst.sense_id("plate_a").unwrap();
        let plate_b = inst.sense_id("plate_b").unwrap();
        let door_path = inst.path_id("door.open").unwrap();
        let foreign = WyrdInstance::new("foreign", and_door_weave()).unwrap();
        assert_eq!(
            signal_truthy(&inst, foreign.path_id("door.open").unwrap()),
            Err(HandleError::ForeignRuntime {
                handle: "host path"
            })
        );
        let mut open = true;
        assert_eq!(
            apply_signal_bool(&inst, foreign.path_id("door.open").unwrap(), &mut open),
            Err(HandleError::ForeignRuntime {
                handle: "host path"
            })
        );
        assert!(open, "a rejected handle must not mutate host state");

        let instance = app.world_mut().resource_mut::<WyrdWorld>().insert(inst);
        let binding = AndDoorBinding {
            plate_a,
            plate_b,
            door_path,
            instance,
        };
        app.insert_resource(binding);
        app.add_systems(Update, sample_plates.in_set(WyrdSet::Sample));
        app.add_systems(Update, apply_door.in_set(WyrdSet::Apply));

        app.update();
        assert!(!app.world().resource::<DoorOpen>().0);

        app.world_mut()
            .insert_resource(PlateState { a: true, b: false });
        app.update();
        assert!(!app.world().resource::<DoorOpen>().0);

        app.world_mut()
            .insert_resource(PlateState { a: true, b: true });
        app.update();
        assert!(app.world().resource::<DoorOpen>().0);

        let removed = app
            .world_mut()
            .resource_mut::<WyrdWorld>()
            .remove(binding.instance)
            .unwrap();
        let removed_tick = removed.tick();
        app.update();
        assert_eq!(removed.tick(), removed_tick);
    }

    #[derive(Resource, Default)]
    struct ConfirmLog(Vec<WyrdSignalConfirm>);

    fn log_confirms(mut reader: MessageReader<WyrdSignalConfirm>, mut log: ResMut<ConfirmLog>) {
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
        let door_path = binding.door_path;
        app.insert_resource(binding);
        app.world_mut().spawn(Door { open: false });
        app.add_systems(Update, sample_plates.in_set(WyrdSet::Sample));
        app.add_systems(Update, apply_door_component.in_set(WyrdSet::Apply));
        app.add_systems(Update, log_confirms.after(WyrdSet::Apply));

        app.world_mut()
            .insert_resource(PlateState { a: true, b: true });
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
    fn instance_state_is_exposed_through_accessors() {
        let inst = WyrdInstance::new("demo", and_door_weave()).unwrap();
        assert_eq!(inst.label(), "demo");
        assert_eq!(inst.tick(), 0);
        assert!(inst.runtime().sense_id("plate_a").is_some());
        assert!(inst.outbox().signals().is_empty());
    }

    #[test]
    fn generational_ids_survive_removal_and_reject_stale_handles() {
        let mut world = WyrdWorld::default();
        let first = world.insert(WyrdInstance::new("first", and_door_weave()).unwrap());
        let second = world.insert(WyrdInstance::new("second", and_door_weave()).unwrap());

        let removed = world.remove(first).unwrap();
        assert_eq!(removed.label(), "first");
        assert_eq!(world.get(second).unwrap().label(), "second");
        assert!(world.get(first).is_none());

        let replacement = world.insert(WyrdInstance::new("replacement", and_door_weave()).unwrap());
        assert_eq!(replacement.index, first.index);
        assert_ne!(replacement.generation, first.generation);
        assert!(world.get(first).is_none());
        assert_eq!(world.get(replacement).unwrap().label(), "replacement");
        assert_eq!(world.len(), 2);
        assert_eq!(world.iter().count(), 2);
        let iter_ids: Vec<_> = world.iter().map(|(id, _)| id).collect();
        assert!(iter_ids.iter().all(|id| world.get(*id).is_some()));
        let iter_mut_ids: Vec<_> = world.iter_mut().map(|(id, _)| id).collect();
        assert!(iter_mut_ids.iter().all(|id| world.get(*id).is_some()));
        assert!(!world.is_empty());
    }

    #[test]
    fn instance_ids_are_rejected_by_other_worlds() {
        let mut first_world = WyrdWorld::default();
        let mut second_world = WyrdWorld::default();
        let first_id = first_world.insert(WyrdInstance::new("first", and_door_weave()).unwrap());
        let second_id = second_world.insert(WyrdInstance::new("second", and_door_weave()).unwrap());

        assert!(second_world.get(first_id).is_none());
        assert!(second_world.get_mut(first_id).is_none());
        assert!(second_world.remove(first_id).is_none());
        assert_eq!(second_world.get(second_id).unwrap().label(), "second");
    }

    #[test]
    fn replacing_world_resource_invalidates_old_ids() {
        let mut app = App::new();
        app.init_resource::<WyrdWorld>();
        let old_id = app
            .world_mut()
            .resource_mut::<WyrdWorld>()
            .insert(WyrdInstance::new("old", and_door_weave()).unwrap());

        app.insert_resource(WyrdWorld::default());
        let new_id = app
            .world_mut()
            .resource_mut::<WyrdWorld>()
            .insert(WyrdInstance::new("new", and_door_weave()).unwrap());

        let world = app.world().resource::<WyrdWorld>();
        assert!(world.get(old_id).is_none());
        assert_eq!(world.get(new_id).unwrap().label(), "new");
    }

    #[test]
    fn generation_overflow_retires_slot() {
        let mut world = WyrdWorld::default();
        let original = world.insert(WyrdInstance::new("original", and_door_weave()).unwrap());
        world.slots[original.index].generation = u64::MAX;
        let final_id = WyrdInstanceId {
            owner: world.owner,
            index: original.index,
            generation: u64::MAX,
        };

        world.remove(final_id).unwrap();
        let replacement = world.insert(WyrdInstance::new("replacement", and_door_weave()).unwrap());

        assert_ne!(replacement.index, final_id.index);
        assert!(world.get(final_id).is_none());
        assert_eq!(world.get(replacement).unwrap().label(), "replacement");
    }
}
