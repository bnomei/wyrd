//! Tier B — first five Weaves (GBG middle core).

#![allow(clippy::result_large_err)]

use super::helpers::{bind_default, signal_out_truthy, tick_senses};
use super::Result;
use crate::authoring::{
    BuildError, KnotDef, Pattern, PatternDef, PatternExportDef, PortRefDef, ThreadDef, Weave,
    WeaveDef,
};
use crate::foundation::{
    from_count, CompareOp, FlagPriority, KnotKind, SignalDomain, TimerMode, ONE, ZERO,
};
use crate::runtime_impl::host::ScriptedHost;
use crate::weave;

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
    let pat = Pattern::try_from(PatternDef {
        id: "pat.mono".into(),
        inner: WeaveDef {
            id: "pat.mono.inner".into(),
            numeric: crate::foundation::NumericPath::compiled(),
            knots: alloc::vec![
                KnotDef {
                    id: "edge".into(),
                    kind: KnotKind::rising_from_zero()
                },
                KnotDef {
                    id: "t".into(),
                    kind: KnotKind::timer(TimerMode::PulseHold, 2)
                }
            ],
            threads: alloc::vec![ThreadDef {
                from: PortRefDef::new("edge", "out"),
                to: PortRefDef::new("t", "start")
            }],
        },
        inputs: alloc::vec![PatternExportDef::new("start", "edge", "in")],
        outputs: alloc::vec![PatternExportDef::new("active", "t", "active")],
    })?;
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

/// B02: Two-plate door (And) over ScriptedHost frames.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_b::run_b02_two_plate_door().unwrap();
/// ```
pub fn run_b02_two_plate_door() -> Result<()> {
    let weave = b02_two_plate_door_weave()?;
    let mut rt = bind_default(&weave)?;
    let a = rt.sense_id("plate_a").expect("a");
    let b = rt.sense_id("plate_b").expect("b");
    let mut host = ScriptedHost::new();
    tick_senses(&mut host, &mut rt, &[(a, ONE), (b, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "door.open"));
    tick_senses(&mut host, &mut rt, &[(a, ONE), (b, ONE)])?;
    assert!(signal_out_truthy(&rt, "door.open"));
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
