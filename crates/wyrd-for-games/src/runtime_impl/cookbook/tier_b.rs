//! Tier B — reusable declarative patterns and compact typed recipes.
//!
//! These examples remain authored with [`crate::weave!`]; B01 introduces
//! [`crate::pattern!`] for a validated reusable fragment. B02 carries that
//! declarative topology across the host boundary with a [`crate::Recipe`] and
//! [`crate::Scenario`], so its runner never re-resolves string handles per frame.

#![allow(clippy::result_large_err)]

use super::helpers::{bind_default, signal_out_truthy, tick_senses};
use super::Result;
use crate::authoring::{BuildError, Pattern, Weave};
use crate::foundation::{
    from_count, CompareOp, FlagPriority, KnotKind, SignalDomain, TimerMode, ONE, ZERO,
};
use crate::runtime_impl::host::ScriptedHost;
use crate::{pattern, weave, HostPathId, Recipe, RecipeResolveError, Scenario, SenseId};

/// Typed ports for the B02 two-plate door recipe.
pub struct B02TwoPlateDoorPorts {
    pub plate_a: SenseId,
    pub plate_b: SenseId,
    pub door: HostPathId,
}

/// Typed host boundary for B02's declarative door topology.
pub struct B02TwoPlateDoorRecipe;

impl Recipe for B02TwoPlateDoorRecipe {
    type Ports = B02TwoPlateDoorPorts;

    fn weave() -> core::result::Result<Weave, BuildError> {
        b02_two_plate_door_weave()
    }

    fn resolve_ports(
        runtime: &crate::Runtime,
    ) -> core::result::Result<Self::Ports, RecipeResolveError> {
        Ok(B02TwoPlateDoorPorts {
            plate_a: runtime.required_sense("plate_a")?,
            plate_b: runtime.required_sense("plate_b")?,
            door: runtime.required_path("door.open")?,
        })
    }
}

/// Topology for B01, with the monostable pattern supplied by the caller.
pub fn b01_monostable_pattern_weave(pat: &Pattern) -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "lvl";
        knots { btn = KnotKind::signal_in(SignalDomain::Bool); out = KnotKind::signal_out("lamp", SignalDomain::Bool); }
        patterns { hold = ("hold1", pat); }
        threads { btn.out -> hold.in("start"); hold.out("active") -> out.in; }
    }
}

/// B01: Monostable Pattern — RisingFromZero → PulseHold (expand-at-load).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_b::run_b01_monostable_pattern().unwrap();
/// ```
pub fn run_b01_monostable_pattern() -> Result<()> {
    run_b01_monostable_pattern_with(false)
}

fn run_b01_monostable_pattern_with(invalid_pattern_id: bool) -> Result<()> {
    let pat = pattern! {
        id: if invalid_pattern_id { "pat/mono" } else { "pat.mono" };
        knots {
            edge = KnotKind::rising_from_zero();
            t = KnotKind::timer(TimerMode::PulseHold, 2);
        }
        exports {
            input start = edge.in;
            output active = t.active;
        }
        threads {
            edge.out -> t.start;
        }
    }?;
    let weave = b01_monostable_pattern_weave(&pat)?;
    let mut rt = bind_default(&weave)?;
    let btn = rt.sense_id("btn").expect("btn");
    let mut host = ScriptedHost::new();
    tick_senses(&mut host, &mut rt, &[(btn, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "lamp"));
    tick_senses(&mut host, &mut rt, &[(btn, ONE)])?;
    assert!(signal_out_truthy(&rt, "lamp"));
    tick_senses(&mut host, &mut rt, &[(btn, ZERO)])?;
    assert!(signal_out_truthy(&rt, "lamp"));
    tick_senses(&mut host, &mut rt, &[(btn, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "lamp"));
    Ok(())
}

/// Topology for B02: two plates → And → door request.
pub fn b02_two_plate_door_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "door";
        knots { plate_a = KnotKind::signal_in(SignalDomain::Bool); plate_b = KnotKind::signal_in(SignalDomain::Bool); both = KnotKind::and2(); door = KnotKind::signal_out("door.open", SignalDomain::Bool); }
        threads { plate_a.out -> both.in_0; plate_b.out -> both.in_1; both.out -> door.in; }
    }
}

/// B02: Two-plate door (And) through typed [`Recipe`] + [`Scenario`] frames.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_b::run_b02_two_plate_door().unwrap();
/// ```
pub fn run_b02_two_plate_door() -> Result<()> {
    Scenario::<B02TwoPlateDoorRecipe>::run(|scenario| {
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

/// Topology for B03: toggle/reset Flag → lamp.
pub fn b03_flag_toggle_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "f";
        knots { tog = KnotKind::signal_in(SignalDomain::Bool); rst = KnotKind::signal_in(SignalDomain::Bool); flag = KnotKind::flag(FlagPriority::ResetWins, true); out = KnotKind::signal_out("lamp", SignalDomain::Bool); }
        threads { tog.out -> flag.toggle; rst.out -> flag.reset; flag.out -> out.in; }
    }
}

/// B03: Flag toggle on rising `toggle` port + `reset` (ResetWins).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_b::run_b03_flag_toggle().unwrap();
/// ```
pub fn run_b03_flag_toggle() -> Result<()> {
    let weave = b03_flag_toggle_weave()?;
    let mut rt = bind_default(&weave)?;
    let tog = rt.sense_id("tog").expect("tog");
    let rst = rt.sense_id("rst").expect("rst");
    let mut host = ScriptedHost::new();
    tick_senses(&mut host, &mut rt, &[(tog, ONE), (rst, ZERO)])?;
    assert!(signal_out_truthy(&rt, "lamp"));
    tick_senses(&mut host, &mut rt, &[(tog, ONE), (rst, ZERO)])?;
    assert!(signal_out_truthy(&rt, "lamp"));
    tick_senses(&mut host, &mut rt, &[(tog, ZERO), (rst, ONE)])?;
    assert!(!signal_out_truthy(&rt, "lamp"));
    Ok(())
}

/// Topology for B04: Counter → Compare(Gte) → ready.
pub fn b04_counter_threshold_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "c";
        knots { inc = KnotKind::signal_in(SignalDomain::Bool); cnt = KnotKind::counter(); cmp = KnotKind::compare(CompareOp::Gte, Some(from_count(2)), SignalDomain::Count); out = KnotKind::signal_out("ready", SignalDomain::Bool); }
        threads { inc.out -> cnt.inc; cnt.count -> cmp.lhs; cmp.out -> out.in; }
    }
}

/// B04: Counter → Compare(Gte) — Counter owns rising-edge on `inc`.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_b::run_b04_counter_threshold().unwrap();
/// ```
pub fn run_b04_counter_threshold() -> Result<()> {
    let weave = b04_counter_threshold_weave()?;
    let mut rt = bind_default(&weave)?;
    let inc = rt.sense_id("inc").expect("inc");
    let mut host = ScriptedHost::new();
    tick_senses(&mut host, &mut rt, &[(inc, ONE)])?;
    assert!(!signal_out_truthy(&rt, "ready"));
    tick_senses(&mut host, &mut rt, &[(inc, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "ready"));
    tick_senses(&mut host, &mut rt, &[(inc, ONE)])?;
    assert!(signal_out_truthy(&rt, "ready"));
    Ok(())
}

/// Topology for B05: Delay → output.
pub fn b05_delayed_pulse_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "d";
        knots { input as "in" = KnotKind::signal_in(SignalDomain::Level); del = KnotKind::Delay { ticks: 2 }; out = KnotKind::signal_out("y", SignalDomain::Level); }
        threads { input.out -> del.in; del.out -> out.in; }
    }
}

/// B05: Delay Rune (2 ticks) passes level through.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_b::run_b05_delayed_pulse().unwrap();
/// ```
pub fn run_b05_delayed_pulse() -> Result<()> {
    let weave = b05_delayed_pulse_weave()?;
    let mut rt = bind_default(&weave)?;
    let id = rt.sense_id("in").expect("in");
    let mut host = ScriptedHost::new();
    tick_senses(&mut host, &mut rt, &[(id, ONE)])?;
    assert!(!signal_out_truthy(&rt, "y"));
    tick_senses(&mut host, &mut rt, &[(id, ONE)])?;
    assert!(!signal_out_truthy(&rt, "y"));
    tick_senses(&mut host, &mut rt, &[(id, ONE)])?;
    assert!(signal_out_truthy(&rt, "y"));
    Ok(())
}

#[cfg(test)]
mod scenario_tests {
    use super::*;

    #[test]
    fn b01_pattern_recipe_runs() {
        run_b01_monostable_pattern().unwrap();
    }

    #[test]
    fn b01_pattern_validation_error_propagates() {
        assert!(run_b01_monostable_pattern_with(true).is_err());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_duplicate_knot_rejected(weave: &Weave, id: &str) {
        let kind = weave
            .knots()
            .iter()
            .find(|knot| knot.id == id)
            .expect("recipe owns the requested knot")
            .kind
            .clone();
        let mut builder = Weave::builder("cookbook-duplicate").expect("valid builder id");
        builder.knot(id, kind.clone()).expect("first knot is valid");
        assert!(matches!(
            builder.knot(id, kind),
            Err(BuildError::DuplicateKnotId { .. })
        ));
    }

    #[test]
    fn b02_and_b04_keep_duplicate_knot_diagnostics() {
        assert_duplicate_knot_rejected(&b02_two_plate_door_weave().unwrap(), "door");
        assert_duplicate_knot_rejected(&b04_counter_threshold_weave().unwrap(), "cmp");
    }
}
