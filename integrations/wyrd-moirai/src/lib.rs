//! Wyrd-owned integration of a typed [`wyrd::Recipe`] with Moirai.
//!
//! This crate deliberately has a separate workspace root and sibling path
//! dependencies. It is a non-publishable integration seam: `wyrd-for-games`
//! remains engine-neutral and never depends on Moirai.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

use alloc::format;
use core::marker::PhantomData;

use moirai::schedule::{stage, System};
use moirai::world::World;
use wyrd::runtime::{Outbox, PortWriter, Recipe, RecipeError, Runtime, RuntimeState};
use wyrd::HostTime;

/// Host-facing failure from a sample or apply callback.
///
/// The adapter preserves this string verbatim inside the contextual Moirai
/// system error, so a host can report a game-specific failure without a panic.
pub type HostError = alloc::string::String;

/// Wyrd host hooks implemented by an integration consumer.
///
/// Implementations keep engine types at this outer boundary. Typed recipe
/// ports are passed by reference, so the host resolves dense ids once at bind
/// time and never needs string lookups on a frame path.
pub trait MoiraiHost<R: Recipe> {
    /// Sample host-owned world state into this frame's Wyrd ports.
    fn sample(
        &mut self,
        world: &mut World,
        ports: &R::Ports,
        writer: &mut PortWriter<'_>,
        tick: u64,
    ) -> Result<(), HostError>;

    /// Apply Wyrd's settled outbox into host-owned world state.
    ///
    /// Returning an error leaves the driver's tick unchanged, making a
    /// retry explicit to the host instead of silently advancing time.
    fn apply(
        &mut self,
        world: &mut World,
        ports: &R::Ports,
        outbox: Outbox<'_>,
        tick: u64,
    ) -> Result<(), HostError>;
}

/// Snapshot of all adapter-owned continuation state.
///
/// `RuntimeState` contains Wyrd's mutable graph state and compatibility
/// fingerprint; `tick` is the host-frame counter used by Sample and Apply.
#[derive(Clone, Debug)]
pub struct MoiraiDriverState {
    runtime: RuntimeState,
    tick: u64,
}

impl MoiraiDriverState {
    /// Wyrd runtime snapshot portion.
    pub fn runtime(&self) -> &RuntimeState {
        &self.runtime
    }

    /// Host frame counter captured with the runtime.
    pub const fn tick(&self) -> u64 {
        self.tick
    }
}

/// A bound, typed Wyrd recipe driven through three Moirai systems.
pub struct MoiraiDriver<R: Recipe, H: MoiraiHost<R>> {
    runtime: Runtime,
    ports: R::Ports,
    host: H,
    tick: u64,
    recipe: PhantomData<fn() -> R>,
}

impl<R: Recipe, H: MoiraiHost<R>> MoiraiDriver<R, H> {
    /// Bind `R` and retain its typed ports alongside the host adapter.
    pub fn bind(host: H) -> Result<Self, RecipeError> {
        let instance = R::bind()?;
        let (runtime, ports) = instance.into_parts();
        Ok(Self {
            runtime,
            ports,
            host,
            tick: 0,
            recipe: PhantomData,
        })
    }

    /// Create a driver from already-bound Wyrd parts.
    pub fn from_parts(runtime: Runtime, ports: R::Ports, host: H) -> Self {
        Self {
            runtime,
            ports,
            host,
            tick: 0,
            recipe: PhantomData,
        }
    }

    /// Current host tick. It advances only after a successful Apply callback.
    pub const fn tick(&self) -> u64 {
        self.tick
    }

    /// Borrow the bound runtime for read-only diagnostics.
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Borrow the recipe's typed, bind-time resolved ports.
    pub fn ports(&self) -> &R::Ports {
        &self.ports
    }

    /// Borrow the host adapter.
    pub fn host(&self) -> &H {
        &self.host
    }

    /// Mutably borrow the host adapter outside schedule execution.
    pub fn host_mut(&mut self) -> &mut H {
        &mut self.host
    }

    /// Snapshot both Wyrd continuation state and the adapter host tick.
    pub fn snapshot(&self) -> MoiraiDriverState {
        MoiraiDriverState {
            runtime: self.runtime.snapshot(),
            tick: self.tick,
        }
    }

    /// Restore runtime and tick atomically.
    ///
    /// Wyrd validates the entire `RuntimeState` before assignment. The tick is
    /// assigned only after that call succeeds, so any rejected snapshot leaves
    /// this driver unchanged as a whole.
    pub fn restore(&mut self, state: &MoiraiDriverState) -> Result<(), wyrd::RestoreError> {
        self.runtime.restore(&state.runtime)?;
        self.tick = state.tick;
        Ok(())
    }

    fn sample(&mut self, world: &mut World) -> Result<(), HostError> {
        self.runtime.begin_frame(HostTime { tick: self.tick });
        let mut writer = self.runtime.port_writer();
        self.host.sample(world, &self.ports, &mut writer, self.tick)
    }

    fn loom(&mut self) {
        self.runtime.loom();
    }

    fn apply(&mut self, world: &mut World) -> Result<(), HostError> {
        self.host
            .apply(world, &self.ports, self.runtime.outbox(), self.tick)?;
        self.tick = self.tick.wrapping_add(1);
        Ok(())
    }
}

/// Return deterministically ordered Sample, Loom, and Apply systems.
///
/// All systems use Moirai's checked [`World::resource_scope`] API. Their
/// declared resource requirement turns a missing driver into a schedule-build
/// failure instead of an optional runtime no-op.
pub fn systems<R, H>() -> [System; 3]
where
    R: Recipe + 'static,
    R::Ports: 'static,
    H: MoiraiHost<R> + 'static,
{
    let sample = System::try_new("wyrd_moirai.sample", stage::UPDATE, |world, _dt| {
        with_driver::<R, H, _>(world, "sample", |driver, world| driver.sample(world))
    })
    .requires_resource::<MoiraiDriver<R, H>>()
    .before("wyrd_moirai.loom");

    let loom = System::try_new("wyrd_moirai.loom", stage::UPDATE, |world, _dt| {
        with_driver::<R, H, _>(world, "loom", |driver, _world| {
            driver.loom();
            Ok(())
        })
    })
    .requires_resource::<MoiraiDriver<R, H>>()
    .after("wyrd_moirai.sample")
    .before("wyrd_moirai.apply");

    let apply = System::try_new("wyrd_moirai.apply", stage::UPDATE, |world, _dt| {
        with_driver::<R, H, _>(world, "apply", |driver, world| driver.apply(world))
    })
    .requires_resource::<MoiraiDriver<R, H>>()
    .after("wyrd_moirai.loom");

    [sample, loom, apply]
}

fn with_driver<R, H, T>(
    world: &mut World,
    phase: &'static str,
    f: impl FnOnce(&mut MoiraiDriver<R, H>, &mut World) -> Result<T, HostError>,
) -> Result<T, HostError>
where
    R: Recipe + 'static,
    R::Ports: 'static,
    H: MoiraiHost<R> + 'static,
{
    world
        .resource_scope::<MoiraiDriver<R, H>, _>(|driver, world| match driver {
            Some(driver) => f(driver, world),
            None => Err(format!("wyrd_moirai {phase}: driver resource is missing")),
        })
        .map_err(|error| format!("wyrd_moirai {phase}: world resource scope failed: {error:?}"))?
        .map_err(|error| format!("wyrd_moirai {phase}: host callback failed: {error}"))
}

#[cfg(test)]
mod tests {
    extern crate std;

    use alloc::vec;
    use std::vec::Vec;

    use super::*;
    use moirai::{AppBuilder, BuildError};
    use wyrd::{
        weave, BuildError as WyrdBuildError, CalcOp, HostPathId, KnotKind, RecipeResolveError,
        SenseId, Signal, SignalDomain, Weave,
    };

    struct GrassRecipe;

    struct GrassPorts {
        wind: SenseId,
        grass: SenseId,
        level: HostPathId,
    }

    impl Recipe for GrassRecipe {
        type Ports = GrassPorts;

        fn weave() -> Result<Weave, WyrdBuildError> {
            weave! {
                id: "sea-grass";
                knots {
                    wind = KnotKind::signal_in(SignalDomain::Level);
                    grass = KnotKind::signal_in(SignalDomain::Level);
                    mix = KnotKind::calc(CalcOp::Add, SignalDomain::Level);
                    level = KnotKind::signal_out("grass.level", SignalDomain::Level);
                }
                threads {
                    wind.out -> mix.a;
                    grass.out -> mix.b;
                    mix.out -> level.in;
                }
            }
        }

        fn resolve_ports(runtime: &Runtime) -> Result<Self::Ports, RecipeResolveError> {
            Ok(GrassPorts {
                wind: runtime.required_sense("wind")?,
                grass: runtime.required_sense("grass")?,
                level: runtime.required_path("grass.level")?,
            })
        }
    }

    #[derive(Default)]
    struct SeaHost {
        wind: Vec<Signal>,
        grass: Vec<Signal>,
        outputs: Vec<Signal>,
        fail_sample: bool,
        fail_apply: bool,
    }

    impl MoiraiHost<GrassRecipe> for SeaHost {
        fn sample(
            &mut self,
            _world: &mut World,
            ports: &GrassPorts,
            writer: &mut PortWriter<'_>,
            tick: u64,
        ) -> Result<(), HostError> {
            if self.fail_sample {
                return Err("sample boom".into());
            }
            let i = tick as usize;
            writer
                .set_sense(ports.wind, self.wind[i])
                .map_err(|error| format!("wind: {error:?}"))?;
            writer
                .set_sense(ports.grass, self.grass[i])
                .map_err(|error| format!("grass: {error:?}"))
        }

        fn apply(
            &mut self,
            _world: &mut World,
            ports: &GrassPorts,
            outbox: Outbox<'_>,
            _tick: u64,
        ) -> Result<(), HostError> {
            if self.fail_apply {
                return Err("apply boom".into());
            }
            let value = outbox
                .signals()
                .iter()
                .find(|sample| sample.path == ports.level)
                .ok_or_else(|| HostError::from("missing grass level"))?
                .value;
            self.outputs.push(value);
            Ok(())
        }
    }

    fn host() -> SeaHost {
        SeaHost {
            wind: vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0],
            grass: vec![9.0, 8.0, 7.0, 6.0, 5.0, 4.0],
            ..SeaHost::default()
        }
    }

    fn app(host: SeaHost) -> moirai::App {
        let mut builder = AppBuilder::new();
        builder.insert_resource(MoiraiDriver::<GrassRecipe, _>::bind(host).unwrap());
        for system in systems::<GrassRecipe, SeaHost>() {
            builder.add_system(system).unwrap();
        }
        builder.build().unwrap()
    }

    #[test]
    fn sea_shaped_continuation_matches_after_restore() {
        let mut uninterrupted = app(host());
        for _ in 0..3 {
            uninterrupted.update(1.0 / 60.0).unwrap();
        }
        let snapshot = uninterrupted
            .world()
            .resource::<MoiraiDriver<GrassRecipe, SeaHost>>()
            .unwrap()
            .unwrap()
            .snapshot();
        for _ in 0..3 {
            uninterrupted.update(1.0 / 60.0).unwrap();
        }
        let expected = uninterrupted
            .world()
            .resource::<MoiraiDriver<GrassRecipe, SeaHost>>()
            .unwrap()
            .unwrap()
            .host()
            .outputs
            .clone();

        let mut restored = app(host());
        for _ in 0..3 {
            restored.update(1.0 / 60.0).unwrap();
        }
        restored
            .world_mut()
            .resource_mut::<MoiraiDriver<GrassRecipe, SeaHost>>()
            .unwrap()
            .unwrap()
            .restore(&snapshot)
            .unwrap();
        for _ in 0..3 {
            restored.update(1.0 / 60.0).unwrap();
        }
        let actual = &restored
            .world()
            .resource::<MoiraiDriver<GrassRecipe, SeaHost>>()
            .unwrap()
            .unwrap()
            .host()
            .outputs;
        assert_eq!(actual, &expected);
    }

    #[test]
    fn missing_driver_is_a_build_failure() {
        let mut builder = AppBuilder::new();
        for system in systems::<GrassRecipe, SeaHost>() {
            builder.add_system(system).unwrap();
        }
        assert!(matches!(
            builder.build(),
            Err(BuildError::MissingRequiredResource { .. })
        ));
    }

    #[test]
    fn sample_and_apply_errors_propagate_and_apply_does_not_advance_tick() {
        let mut sample_host = host();
        sample_host.fail_sample = true;
        let mut sample_app = app(sample_host);
        let sample_error = sample_app.update(1.0 / 60.0).unwrap_err();
        assert!(matches!(
            sample_error,
            moirai::AppError::Fault(ref fault)
                if fault.detail.as_deref().is_some_and(|detail| detail.contains("sample boom"))
        ));

        let mut apply_host = host();
        apply_host.fail_apply = true;
        let mut apply_app = app(apply_host);
        let apply_error = apply_app.update(1.0 / 60.0).unwrap_err();
        assert!(matches!(
            apply_error,
            moirai::AppError::Fault(ref fault)
                if fault.detail.as_deref().is_some_and(|detail| detail.contains("apply boom"))
        ));
        assert_eq!(
            apply_app
                .world()
                .resource::<MoiraiDriver<GrassRecipe, SeaHost>>()
                .unwrap()
                .unwrap()
                .tick(),
            0
        );
    }

    #[test]
    fn fingerprint_rejection_keeps_driver_tick_unchanged() {
        let mut driver_app = app(host());
        driver_app.update(1.0 / 60.0).unwrap();
        let other = MoiraiDriver::<OtherRecipe, _>::bind(OtherHost).unwrap();
        let incompatible = other.snapshot();
        let before = driver_app
            .world()
            .resource::<MoiraiDriver<GrassRecipe, SeaHost>>()
            .unwrap()
            .unwrap()
            .snapshot();
        assert_eq!(before.tick(), 1);

        let driver = driver_app
            .world_mut()
            .resource_mut::<MoiraiDriver<GrassRecipe, SeaHost>>()
            .unwrap()
            .unwrap();
        assert!(driver.restore(&incompatible).is_err());
        let after = driver.snapshot();

        assert_eq!(after.tick(), before.tick());
        assert_eq!(
            format!("{:?}", after.runtime()),
            format!("{:?}", before.runtime())
        );
    }

    struct OtherRecipe;
    impl Recipe for OtherRecipe {
        type Ports = SenseId;
        fn weave() -> Result<Weave, WyrdBuildError> {
            weave! {
                id: "other";
                knots {
                    in_ = KnotKind::signal_in(SignalDomain::Bool);
                    out = KnotKind::signal_out("other.output", SignalDomain::Bool);
                }
                threads { in_.out -> out.in; }
            }
        }
        fn resolve_ports(runtime: &Runtime) -> Result<Self::Ports, RecipeResolveError> {
            runtime.required_sense("in_")
        }
    }
    struct OtherHost;
    impl MoiraiHost<OtherRecipe> for OtherHost {
        fn sample(
            &mut self,
            _: &mut World,
            _: &SenseId,
            _: &mut PortWriter<'_>,
            _: u64,
        ) -> Result<(), HostError> {
            Ok(())
        }
        fn apply(
            &mut self,
            _: &mut World,
            _: &SenseId,
            _: Outbox<'_>,
            _: u64,
        ) -> Result<(), HostError> {
            Ok(())
        }
    }
}
