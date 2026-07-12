//! Tier A — declarative foundations and the typed host boundary.
//!
//! A01–A04 pair [`crate::weave!`] topology with [`crate::Recipe`] port
//! resolution and closure-scoped [`crate::Scenario`] frames. This is the
//! default shape for a static recipe: graph names stay at authoring time while
//! a generic host receives dense typed handles after bind. A05 intentionally
//! remains a direct validation lesson.

#![allow(clippy::result_large_err)]

use super::Result;
use crate::authoring::{BuildError, ValidationError, Weave};
use crate::foundation::{from_count, KnotKind, SignalDomain, ONE, ZERO};
use crate::weave;
use crate::{HostPathId, Recipe, RecipeResolveError, Scenario, SenseId};

/// Typed ports for the A01 tutorial recipe.
pub struct A01HelloInvertPorts {
    pub inverted: HostPathId,
}

/// Typed recipe binding for A01.
pub struct A01HelloInvertRecipe;

impl Recipe for A01HelloInvertRecipe {
    type Ports = A01HelloInvertPorts;

    fn weave() -> core::result::Result<Weave, BuildError> {
        a01_hello_invert_weave()
    }

    fn resolve_ports(
        runtime: &crate::Runtime,
    ) -> core::result::Result<Self::Ports, RecipeResolveError> {
        Ok(A01HelloInvertPorts {
            inverted: runtime.required_path("debug.inverted")?,
        })
    }
}

/// Typed ports for the A02 tutorial recipe.
pub struct A02TwoPlateAndPorts {
    pub plate_a: SenseId,
    pub plate_b: SenseId,
    pub door: HostPathId,
}

/// Typed recipe binding for A02.
pub struct A02TwoPlateAndRecipe;

impl Recipe for A02TwoPlateAndRecipe {
    type Ports = A02TwoPlateAndPorts;

    fn weave() -> core::result::Result<Weave, BuildError> {
        a02_two_plate_and_weave()
    }

    fn resolve_ports(
        runtime: &crate::Runtime,
    ) -> core::result::Result<Self::Ports, RecipeResolveError> {
        Ok(A02TwoPlateAndPorts {
            plate_a: runtime.required_sense("plate_a")?,
            plate_b: runtime.required_sense("plate_b")?,
            door: runtime.required_path("door.open")?,
        })
    }
}

/// Typed ports for the A03 tutorial recipe.
pub struct A03BindSampleLoomPorts {
    pub input: SenseId,
    pub output: HostPathId,
}

/// Typed recipe binding for A03.
pub struct A03BindSampleLoomRecipe;

impl Recipe for A03BindSampleLoomRecipe {
    type Ports = A03BindSampleLoomPorts;

    fn weave() -> core::result::Result<Weave, BuildError> {
        a03_bind_sample_loom_weave()
    }

    fn resolve_ports(
        runtime: &crate::Runtime,
    ) -> core::result::Result<Self::Ports, RecipeResolveError> {
        Ok(A03BindSampleLoomPorts {
            input: runtime.required_sense("in")?,
            output: runtime.required_path("y")?,
        })
    }
}

/// Typed ports for the A04 tutorial recipe.
pub struct A04HostTickOncePorts {
    pub input: SenseId,
    pub lamp: HostPathId,
}

/// Typed recipe binding for A04.
pub struct A04HostTickOnceRecipe;

impl Recipe for A04HostTickOnceRecipe {
    type Ports = A04HostTickOncePorts;

    fn weave() -> core::result::Result<Weave, BuildError> {
        a04_host_tick_once_weave()
    }

    fn resolve_ports(
        runtime: &crate::Runtime,
    ) -> core::result::Result<Self::Ports, RecipeResolveError> {
        Ok(A04HostTickOncePorts {
            input: runtime.required_sense("in")?,
            lamp: runtime.required_path("lamp")?,
        })
    }
}

/// Topology for A01: Constant(ONE) → Not → SignalOut.
pub fn a01_hello_invert_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "a01";
        knots {
            c = KnotKind::constant(ONE, SignalDomain::Bool);
            n = KnotKind::not();
            o = KnotKind::signal_out("debug.inverted", SignalDomain::Bool);
        }
        threads { c.out -> n.in; n.out -> o.in; }
    }
}

/// A01: Constant(ONE) → Not → SignalOut (falsey).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a01_hello_invert().unwrap();
/// ```
pub fn run_a01_hello_invert() -> Result<()> {
    Scenario::<A01HelloInvertRecipe>::run(|scenario| {
        scenario.frame(|_| Ok(()))?;
        scenario.expect_value(|ports| ports.inverted, ZERO)
    })?;
    Ok(())
}

/// Topology for A02: two plates → And → `door.open`.
pub fn a02_two_plate_and_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "a02";
        knots {
            plate_a = KnotKind::signal_in(SignalDomain::Bool);
            plate_b = KnotKind::signal_in(SignalDomain::Bool);
            both = KnotKind::and2();
            door = KnotKind::signal_out("door.open", SignalDomain::Bool);
        }
        threads { plate_a.out -> both.in_0; plate_b.out -> both.in_1; both.out -> door.in; }
    }
}

/// A02: Two plates → And → `door.open` request (host owns the door).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a02_two_plate_and().unwrap();
/// ```
pub fn run_a02_two_plate_and() -> Result<()> {
    Scenario::<A02TwoPlateAndRecipe>::run(|scenario| {
        scenario.frame(|frame| {
            frame.set(|ports| ports.plate_a, ONE)?;
            frame.set(|ports| ports.plate_b, ZERO)
        })?;
        scenario.expect_value(|ports| ports.door, ZERO)?;
        scenario.frame(|frame| {
            frame.set(|ports| ports.plate_a, ONE)?;
            frame.set(|ports| ports.plate_b, ONE)
        })?;
        scenario.expect_truthy(|ports| ports.door)
    })?;
    Ok(())
}

/// Topology for A03: input → Not → output.
pub fn a03_bind_sample_loom_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "a03";
        knots {
            input as "in" = KnotKind::signal_in(SignalDomain::Bool);
            n = KnotKind::not();
            o = KnotKind::signal_out("y", SignalDomain::Bool);
        }
        threads { input.out -> n.in; n.out -> o.in; }
    }
}

/// A03: Dense bind path — `set_sense` + loom + outbox (no Host trait).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a03_bind_sample_loom().unwrap();
/// ```
pub fn run_a03_bind_sample_loom() -> Result<()> {
    Scenario::<A03BindSampleLoomRecipe>::run(|scenario| {
        scenario.frame(|frame| frame.set(|ports| ports.input, ZERO))?;
        scenario.expect_truthy(|ports| ports.output)?;
        scenario.frame(|frame| frame.set(|ports| ports.input, ONE))?;
        scenario.expect_value(|ports| ports.output, ZERO)
    })?;
    Ok(())
}

/// Topology for A04: input → `lamp` output.
pub fn a04_host_tick_once_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "a04";
        knots { input as "in" = KnotKind::signal_in(SignalDomain::Bool); o = KnotKind::signal_out("lamp", SignalDomain::Bool); }
        threads { input.out -> o.in; }
    }
}

/// A04: [`ScriptedHost`](crate::ScriptedHost) + [`tick_once`](crate::tick_once) over two frames.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a04_host_tick_once().unwrap();
/// ```
pub fn run_a04_host_tick_once() -> Result<()> {
    Scenario::<A04HostTickOnceRecipe>::run(|scenario| {
        scenario.frame(|frame| frame.set(|ports| ports.input, ZERO))?;
        scenario.expect_value(|ports| ports.lamp, ZERO)?;
        scenario.frame(|frame| frame.set(|ports| ports.input, ONE))?;
        scenario.expect_truthy(|ports| ports.lamp)
    })?;
    Ok(())
}

/// Topology for A05, deliberately invalid because its Map range is inverted.
pub fn a05_validate_fails_weave() -> core::result::Result<Weave, BuildError> {
    a05_validate_fails_weave_with(false)
}

fn a05_validate_fails_weave_with(valid_map: bool) -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "a05";
        knots {
            c = KnotKind::constant(ONE, SignalDomain::Level);
            map = KnotKind::Map { domain: SignalDomain::Level, in_min: from_count(if valid_map { 0 } else { 5 }), in_max: from_count(1), out_min: ZERO, out_max: ONE };
            o = KnotKind::signal_out("y", SignalDomain::Level);
        }
        threads { c.out -> map.in; map.out -> o.in; }
    }
}

/// A05: Validate rejects inverted Map in-range (`InvalidParameter`).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a05_validate_fails().unwrap();
/// ```
pub fn run_a05_validate_fails() -> Result<()> {
    run_a05_validate_fails_with(false)
}

fn run_a05_validate_fails_with(valid_map: bool) -> Result<()> {
    match a05_validate_fails_weave_with(valid_map) {
        Err(BuildError::Validation(ValidationError::InvalidParameter { .. })) => Ok(()),
        other => panic!("expected invalid Map range, got {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a05_invalid_map_preserves_the_validation_error() {
        assert!(matches!(
            a05_validate_fails_weave(),
            Err(BuildError::Validation(
                ValidationError::InvalidParameter { .. }
            ))
        ));
    }

    #[test]
    #[should_panic(expected = "expected invalid Map range")]
    fn a05_valid_map_reaches_the_diagnostic_branch() {
        let _ = run_a05_validate_fails_with(true);
    }
}
