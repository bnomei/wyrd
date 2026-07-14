//! Typed, closure-scoped authoring for dynamically generated [`Weave`](crate::Weave)s.
//!
//! [`Weave::compose`](crate::Weave::compose) is deliberately a thin layer over
//! [`WeaveBuilder`](crate::WeaveBuilder): it gives common graph operations
//! domain-typed wires while retaining [`Composer::knot`] and
//! [`Composer::thread`] for every catalog operation.

use core::marker::PhantomData;

use crate::authoring::{
    BuildError, InputPort, KnotHandle, OutputPort, Pattern, PatternInstance, ValidationError,
    Weave, WeaveBuilder,
};
use crate::foundation::{
    CalcOp, CompareOp, FlagPriority, KnotKind, Signal, SignalDomain, TimerMode,
};

/// Error produced while composing a generated weave.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ComposeError {
    /// A scoped composer operation was rejected before final validation.
    Build(BuildError),
    /// The complete generated graph failed normal weave validation.
    Validation(ValidationError),
}

impl From<BuildError> for ComposeError {
    fn from(value: BuildError) -> Self {
        Self::Build(value)
    }
}

impl From<ValidationError> for ComposeError {
    fn from(value: ValidationError) -> Self {
        Self::Validation(value)
    }
}

impl core::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Build(error) => error.fmt(f),
            Self::Validation(error) => error.fmt(f),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ComposeError {}

/// A signal domain that may be represented by a typed [`Wire`].
pub trait WireDomain {
    /// Runtime domain enforced by helpers that accept this wire.
    const DOMAIN: SignalDomain;
}

/// Marker for boolean graph wires.
#[derive(Clone, Copy, Debug)]
pub struct Bool;

/// Marker for continuous level graph wires.
#[derive(Clone, Copy, Debug)]
pub struct Level;

/// Marker for whole-count graph wires.
#[derive(Clone, Copy, Debug)]
pub struct Count;

impl WireDomain for Bool {
    const DOMAIN: SignalDomain = SignalDomain::Bool;
}

impl WireDomain for Level {
    const DOMAIN: SignalDomain = SignalDomain::Level;
}

impl WireDomain for Count {
    const DOMAIN: SignalDomain = SignalDomain::Count;
}

/// A typed wire domain accepted by numeric catalog operations.
pub trait NumericWireDomain: WireDomain {}

impl NumericWireDomain for Level {}
impl NumericWireDomain for Count {}

/// A typed output wire owned by the active [`Composer`].
///
/// It can be cloned to intentionally fan out to several downstream knots.
#[derive(Clone, Debug)]
pub struct Wire<D: WireDomain> {
    port: OutputPort,
    marker: PhantomData<D>,
}

impl<D: WireDomain> Wire<D> {
    /// Underlying catalog-checked output port for raw escape-hatch operations.
    pub fn port(&self) -> &OutputPort {
        &self.port
    }
}

/// Boolean wire alias used by composition examples and helpers.
pub type BoolWire = Wire<Bool>;
/// Level wire alias used by composition examples and helpers.
pub type LevelWire = Wire<Level>;
/// Count wire alias used by composition examples and helpers.
pub type CountWire = Wire<Count>;

/// Dynamic graph composer scoped to one generated [`Weave`].
///
/// Use typed helpers for common semantic operations. For an operation that has
/// no helper yet, use [`Self::knot`], [`Self::input`], [`Self::output`], and
/// [`Self::thread`]; these delegate directly to `WeaveBuilder` and therefore
/// retain the complete catalog and its diagnostics.
pub struct Composer {
    builder: WeaveBuilder,
}

impl Composer {
    pub(crate) fn new(id: impl Into<std::string::String>) -> Result<Self, BuildError> {
        Ok(Self {
            builder: WeaveBuilder::new(id)?,
        })
    }

    pub(crate) fn build(self) -> Result<Weave, ValidationError> {
        self.builder.build()
    }

    /// Add any catalog knot under an explicit author id.
    pub fn knot(
        &mut self,
        id: impl Into<std::string::String>,
        kind: KnotKind,
    ) -> Result<KnotHandle, BuildError> {
        self.builder.knot(id, kind)
    }

    /// Resolve any catalog input port for [`Self::thread`].
    pub fn input(&self, knot: &KnotHandle, name: &str) -> Result<InputPort, BuildError> {
        self.builder.input(knot, name)
    }

    /// Resolve any catalog output port for [`Self::thread`].
    pub fn output(&self, knot: &KnotHandle, name: &str) -> Result<OutputPort, BuildError> {
        self.builder.output(knot, name)
    }

    /// Connect raw catalog ports, retaining `WeaveBuilder` ownership and
    /// fixed-domain diagnostics.
    pub fn thread(&mut self, from: &OutputPort, to: &InputPort) -> Result<(), BuildError> {
        self.builder.connect(from.clone(), to.clone())?;
        Ok(())
    }

    /// Include a reusable pattern through the authoritative builder expansion.
    pub fn include(
        &mut self,
        instance_id: impl Into<std::string::String>,
        pattern: &Pattern,
    ) -> Result<PatternInstance, BuildError> {
        self.builder.include(instance_id, pattern)
    }

    /// Host boolean sense source.
    pub fn bool_input(
        &mut self,
        id: impl Into<std::string::String>,
    ) -> Result<BoolWire, BuildError> {
        self.source(id, KnotKind::signal_in(SignalDomain::Bool))
    }

    /// Host continuous-level sense source.
    pub fn level_input(
        &mut self,
        id: impl Into<std::string::String>,
    ) -> Result<LevelWire, BuildError> {
        self.source(id, KnotKind::signal_in(SignalDomain::Level))
    }

    /// Host whole-count sense source.
    pub fn count_input(
        &mut self,
        id: impl Into<std::string::String>,
    ) -> Result<CountWire, BuildError> {
        self.source(id, KnotKind::signal_in(SignalDomain::Count))
    }

    /// Boolean literal source.
    pub fn bool_constant(
        &mut self,
        id: impl Into<std::string::String>,
        value: bool,
    ) -> Result<BoolWire, BuildError> {
        self.source(id, KnotKind::constant_bool(value))
    }

    /// One-shot boolean source emitted on the first settle after bind.
    pub fn on_start(&mut self, id: impl Into<std::string::String>) -> Result<BoolWire, BuildError> {
        self.source(id, KnotKind::OnStart)
    }

    /// Level literal source.
    pub fn level_constant(
        &mut self,
        id: impl Into<std::string::String>,
        value: f32,
    ) -> Result<LevelWire, BuildError> {
        self.source(id, KnotKind::constant_level(value))
    }

    /// Count literal source.
    pub fn count_constant(
        &mut self,
        id: impl Into<std::string::String>,
        value: i32,
    ) -> Result<CountWire, BuildError> {
        self.source(id, KnotKind::constant_count(value))
    }

    /// Bind a typed wire to a host output path under an explicit knot id.
    pub fn signal_out<D: WireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        path: impl Into<std::string::String>,
        wire: &Wire<D>,
    ) -> Result<(), BuildError> {
        let knot = self.knot(id, KnotKind::signal_out(path, D::DOMAIN))?;
        let input = self.input(&knot, "in")?;
        self.thread(wire.port(), &input)
    }

    /// Boolean inversion.
    pub fn not(
        &mut self,
        id: impl Into<std::string::String>,
        input: &BoolWire,
    ) -> Result<BoolWire, BuildError> {
        self.unary(id, KnotKind::not(), input)
    }

    /// Boolean conjunction with two inputs.
    pub fn and(
        &mut self,
        id: impl Into<std::string::String>,
        a: &BoolWire,
        b: &BoolWire,
    ) -> Result<BoolWire, BuildError> {
        self.binary(id, KnotKind::and2(), a, b, "in_0", "in_1")
    }

    /// Boolean disjunction with two inputs.
    pub fn or(
        &mut self,
        id: impl Into<std::string::String>,
        a: &BoolWire,
        b: &BoolWire,
    ) -> Result<BoolWire, BuildError> {
        self.binary(id, KnotKind::or2(), a, b, "in_0", "in_1")
    }

    /// Boolean exclusive-or.
    pub fn xor(
        &mut self,
        id: impl Into<std::string::String>,
        a: &BoolWire,
        b: &BoolWire,
    ) -> Result<BoolWire, BuildError> {
        self.binary(id, KnotKind::xor(), a, b, "a", "b")
    }

    /// Rising truthiness edge, preserving the source domain only at the input.
    pub fn rising<D: WireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        input: &Wire<D>,
    ) -> Result<BoolWire, BuildError> {
        self.unary(id, KnotKind::rising_from_zero(), input)
    }

    /// Falling truthiness edge.
    pub fn falling<D: WireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        input: &Wire<D>,
    ) -> Result<BoolWire, BuildError> {
        self.unary(id, KnotKind::falling_to_zero(), input)
    }

    /// Any truthiness-change edge.
    pub fn change<D: WireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        input: &Wire<D>,
    ) -> Result<BoolWire, BuildError> {
        self.unary(id, KnotKind::change(), input)
    }

    /// State latch. Optional inputs map directly to the catalog's optional ports.
    pub fn flag(
        &mut self,
        id: impl Into<std::string::String>,
        priority: FlagPriority,
        set: Option<&BoolWire>,
        reset: Option<&BoolWire>,
        toggle: Option<&BoolWire>,
    ) -> Result<BoolWire, BuildError> {
        let knot = self.knot(id, KnotKind::flag(priority, toggle.is_some()))?;
        self.optional_thread(set, &knot, "set")?;
        self.optional_thread(reset, &knot, "reset")?;
        self.optional_thread(toggle, &knot, "toggle")?;
        self.typed_output(&knot, "out")
    }

    /// Counter with optional increment, decrement, and reset controls.
    pub fn counter(
        &mut self,
        id: impl Into<std::string::String>,
        increment: Option<&BoolWire>,
        decrement: Option<&BoolWire>,
        reset: Option<&BoolWire>,
    ) -> Result<CountWire, BuildError> {
        let knot = self.knot(id, KnotKind::counter())?;
        self.optional_thread(increment, &knot, "inc")?;
        self.optional_thread(decrement, &knot, "dec")?;
        self.optional_thread(reset, &knot, "reset")?;
        self.typed_output(&knot, "count")
    }

    /// Pulse-hold timer started by a boolean edge.
    pub fn pulse_hold(
        &mut self,
        id: impl Into<std::string::String>,
        ticks: u16,
        start: &BoolWire,
    ) -> Result<BoolWire, BuildError> {
        let knot = self.knot(id, KnotKind::timer(TimerMode::PulseHold, ticks))?;
        let input = self.input(&knot, "start")?;
        self.thread(start.port(), &input)?;
        self.typed_output(&knot, "active")
    }

    /// Countdown timer fed by a boolean level.
    pub fn fed_countdown(
        &mut self,
        id: impl Into<std::string::String>,
        ticks: u16,
        feed: &BoolWire,
    ) -> Result<BoolWire, BuildError> {
        let knot = self.knot(id, KnotKind::timer(TimerMode::FedCountdown, ticks))?;
        let input = self.input(&knot, "feed")?;
        self.thread(feed.port(), &input)?;
        self.typed_output(&knot, "active")
    }

    /// Compare two wires in the same typed domain.
    pub fn compare<D: WireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        op: CompareOp,
        lhs: &Wire<D>,
        rhs: &Wire<D>,
    ) -> Result<BoolWire, BuildError> {
        self.binary(
            id,
            KnotKind::compare(op, None, D::DOMAIN),
            lhs,
            rhs,
            "lhs",
            "rhs",
        )
    }

    /// Compare a wire to a catalog literal in the same typed domain.
    pub fn compare_constant<D: WireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        op: CompareOp,
        lhs: &Wire<D>,
        rhs: Signal,
    ) -> Result<BoolWire, BuildError> {
        let knot = self.knot(id, KnotKind::compare(op, Some(rhs), D::DOMAIN))?;
        let input = self.input(&knot, "lhs")?;
        self.thread(lhs.port(), &input)?;
        self.typed_output(&knot, "out")
    }

    /// Binary arithmetic in one typed numeric domain.
    pub fn calc<D: NumericWireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        op: CalcOp,
        a: &Wire<D>,
        b: &Wire<D>,
    ) -> Result<Wire<D>, BuildError> {
        self.binary(id, KnotKind::calc(op, D::DOMAIN), a, b, "a", "b")
    }

    /// Linear mapping in one typed numeric domain.
    pub fn map<D: NumericWireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        input: &Wire<D>,
        in_min: Signal,
        in_max: Signal,
        out_min: Signal,
        out_max: Signal,
    ) -> Result<Wire<D>, BuildError> {
        self.unary(
            id,
            KnotKind::map(in_min, in_max, out_min, out_max, D::DOMAIN),
            input,
        )
    }

    /// Threshold a typed numeric wire into boolean state and edge outputs.
    pub fn threshold<D: NumericWireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        input: &Wire<D>,
        high: Signal,
        low: Signal,
        use_hysteresis: bool,
    ) -> Result<ThresholdWires, BuildError> {
        let knot = self.knot(
            id,
            KnotKind::Threshold {
                domain: D::DOMAIN,
                high,
                low,
                use_hysteresis,
            },
        )?;
        let target = self.input(&knot, "in")?;
        self.thread(input.port(), &target)?;
        Ok(ThresholdWires {
            out: self.typed_output(&knot, "out")?,
            crossed_up: self.typed_output(&knot, "crossed_up")?,
            crossed_down: self.typed_output(&knot, "crossed_down")?,
        })
    }

    /// Delay a typed wire without changing its domain.
    pub fn delay<D: WireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        ticks: u16,
        input: &Wire<D>,
    ) -> Result<Wire<D>, BuildError> {
        self.unary(id, KnotKind::Delay { ticks }, input)
    }

    /// Absolute value in a typed numeric domain.
    pub fn abs<D: NumericWireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        input: &Wire<D>,
    ) -> Result<Wire<D>, BuildError> {
        self.unary(id, KnotKind::abs(D::DOMAIN), input)
    }

    /// Negation in a typed numeric domain.
    pub fn neg<D: NumericWireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        input: &Wire<D>,
    ) -> Result<Wire<D>, BuildError> {
        self.unary(id, KnotKind::neg(D::DOMAIN), input)
    }

    /// Square root in a typed numeric domain.
    pub fn sqrt<D: NumericWireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        input: &Wire<D>,
    ) -> Result<Wire<D>, BuildError> {
        self.unary(id, KnotKind::sqrt(D::DOMAIN), input)
    }

    /// Clamp in a typed numeric domain.
    pub fn clamp<D: NumericWireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        input: &Wire<D>,
        min: Signal,
        max: Signal,
    ) -> Result<Wire<D>, BuildError> {
        self.unary(id, KnotKind::clamp(min, max, D::DOMAIN), input)
    }

    /// Digitize a typed numeric wire into a fixed number of bins.
    pub fn digitize<D: NumericWireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        input: &Wire<D>,
        steps: u16,
    ) -> Result<Wire<D>, BuildError> {
        self.unary(id, KnotKind::digitize(steps, D::DOMAIN), input)
    }

    /// Random source, optionally bounded by typed wires and gated by a boolean.
    pub fn random<D: NumericWireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        require_gate: bool,
        min: Option<&Wire<D>>,
        max: Option<&Wire<D>>,
        gate: Option<&BoolWire>,
    ) -> Result<Wire<D>, BuildError> {
        let knot = self.knot(id, KnotKind::random(require_gate, D::DOMAIN))?;
        self.optional_thread(min, &knot, "min")?;
        self.optional_thread(max, &knot, "max")?;
        self.optional_thread(gate, &knot, "gate")?;
        self.typed_output(&knot, "out")
    }

    /// Select between two wires of the same typed domain.
    pub fn select<D: WireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        select: &BoolWire,
        a: &Wire<D>,
        b: &Wire<D>,
    ) -> Result<Wire<D>, BuildError> {
        let knot = self.knot(id, KnotKind::select())?;
        for (source, name) in [(select.port(), "sel"), (a.port(), "a"), (b.port(), "b")] {
            let input = self.input(&knot, name)?;
            self.thread(source, &input)?;
        }
        self.typed_output(&knot, "out")
    }

    /// Explicitly convert one typed domain to another.
    pub fn convert<F: WireDomain, T: WireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        input: &Wire<F>,
    ) -> Result<Wire<T>, BuildError> {
        self.unary(id, KnotKind::convert(F::DOMAIN, T::DOMAIN), input)
    }

    /// Emit a named host command from a boolean trigger.
    pub fn emit(
        &mut self,
        id: impl Into<std::string::String>,
        name: impl Into<std::string::String>,
        trigger: &BoolWire,
    ) -> Result<(), BuildError> {
        let knot = self.knot(id, KnotKind::emit_command(name))?;
        let input = self.input(&knot, "trigger")?;
        self.thread(trigger.port(), &input)
    }

    fn source<D: WireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        kind: KnotKind,
    ) -> Result<Wire<D>, BuildError> {
        let knot = self.knot(id, kind)?;
        self.typed_output(&knot, "out")
    }

    fn unary<D: WireDomain, R: WireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        kind: KnotKind,
        input: &Wire<D>,
    ) -> Result<Wire<R>, BuildError> {
        let knot = self.knot(id, kind)?;
        let target = self.input(&knot, "in")?;
        self.thread(input.port(), &target)?;
        self.typed_output(&knot, "out")
    }

    fn binary<D: WireDomain, E: WireDomain, R: WireDomain>(
        &mut self,
        id: impl Into<std::string::String>,
        kind: KnotKind,
        a: &Wire<D>,
        b: &Wire<E>,
        a_port: &str,
        b_port: &str,
    ) -> Result<Wire<R>, BuildError> {
        let knot = self.knot(id, kind)?;
        let a_target = self.input(&knot, a_port)?;
        let b_target = self.input(&knot, b_port)?;
        self.thread(a.port(), &a_target)?;
        self.thread(b.port(), &b_target)?;
        self.typed_output(&knot, "out")
    }

    fn optional_thread<D: WireDomain>(
        &mut self,
        wire: Option<&Wire<D>>,
        knot: &KnotHandle,
        port: &str,
    ) -> Result<(), BuildError> {
        if let Some(wire) = wire {
            let input = self.input(knot, port)?;
            self.thread(wire.port(), &input)?;
        }
        Ok(())
    }

    fn typed_output<D: WireDomain>(
        &self,
        knot: &KnotHandle,
        port: &str,
    ) -> Result<Wire<D>, BuildError> {
        Ok(Wire {
            port: self.output(knot, port)?,
            marker: PhantomData,
        })
    }
}

/// Boolean outputs of a threshold knot.
#[derive(Clone, Debug)]
pub struct ThresholdWires {
    /// Current threshold state.
    pub out: BoolWire,
    /// One-frame rising threshold crossing.
    pub crossed_up: BoolWire,
    /// One-frame falling threshold crossing.
    pub crossed_down: BoolWire,
}
