//! Tier C — GBG / Zelda literacy (softlock-aware compositions).

#![allow(clippy::result_large_err)]

use super::helpers::{bind_default, emit_count, signal_out_truthy, signal_out_value, tick_senses};
use super::Result;
use crate::authoring::{BuildError, Weave};
use crate::foundation::{
    from_count, CompareOp, FlagPriority, KnotKind, SignalDomain, TimerMode, ONE, ZERO,
};
use crate::runtime_impl::host::ScriptedHost;
use crate::weave;

/// Topology for C01: multi-switch latch.
pub fn c01_multi_switch_latch_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "c01"; knots { a = KnotKind::signal_in(SignalDomain::Bool); b = KnotKind::signal_in(SignalDomain::Bool); both = KnotKind::and2(); edge = KnotKind::rising_from_zero(); rst = KnotKind::signal_in(SignalDomain::Bool); flag = KnotKind::flag(FlagPriority::ResetWins, false); out = KnotKind::signal_out("door.open", SignalDomain::Bool); }
        threads { a.out -> both.in_0; b.out -> both.in_1; both.out -> edge.in; edge.out -> flag.set; rst.out -> flag.reset; flag.out -> out.in; }
    }
}

/// C01: MultiSwitchLatch — both plates once together → Flag until reset.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c01_multi_switch_latch().unwrap();
/// ```
pub fn run_c01_multi_switch_latch() -> Result<()> {
    let weave = c01_multi_switch_latch_weave()?;
    let mut rt = bind_default(&weave)?;
    let a = rt.sense_id("a").unwrap();
    let b = rt.sense_id("b").unwrap();
    let r = rt.sense_id("rst").unwrap();
    let mut h = ScriptedHost::new();
    tick_senses(&mut h, &mut rt, &[(a, ONE), (b, ZERO), (r, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "door.open"));
    tick_senses(&mut h, &mut rt, &[(a, ONE), (b, ONE), (r, ZERO)])?;
    assert!(signal_out_truthy(&rt, "door.open"));
    tick_senses(&mut h, &mut rt, &[(a, ZERO), (b, ZERO), (r, ZERO)])?;
    assert!(signal_out_truthy(&rt, "door.open"));
    tick_senses(&mut h, &mut rt, &[(a, ZERO), (b, ZERO), (r, ONE)])?;
    assert!(!signal_out_truthy(&rt, "door.open"));
    Ok(())
}

/// Topology for C02: fed countdown hold.
pub fn c02_timed_hold_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "c02"; knots { plate = KnotKind::signal_in(SignalDomain::Bool); t = KnotKind::timer(TimerMode::FedCountdown, 2); out = KnotKind::signal_out("unlocked", SignalDomain::Bool); } threads { plate.out -> t.feed; t.active -> out.in; }
    }
}

/// C02: TimedHold — FedCountdown while plate held.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c02_timed_hold().unwrap();
/// ```
pub fn run_c02_timed_hold() -> Result<()> {
    let w = c02_timed_hold_weave()?;
    let mut rt = bind_default(&w)?;
    let p = rt.sense_id("plate").unwrap();
    let mut h = ScriptedHost::new();
    tick_senses(&mut h, &mut rt, &[(p, ONE)])?;
    assert!(!signal_out_truthy(&rt, "unlocked"));
    tick_senses(&mut h, &mut rt, &[(p, ONE)])?;
    assert!(signal_out_truthy(&rt, "unlocked"));
    tick_senses(&mut h, &mut rt, &[(p, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "unlocked"));
    Ok(())
}

/// Topology for C03: counter threshold reward window.
pub fn c03_press_n_then_window_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "c03"; knots { inc = KnotKind::signal_in(SignalDomain::Bool); cnt = KnotKind::counter(); cmp = KnotKind::compare(CompareOp::Gte, Some(from_count(2)), SignalDomain::Count); rise = KnotKind::rising_from_zero(); hold = KnotKind::timer(TimerMode::PulseHold, 2); out = KnotKind::signal_out("reward", SignalDomain::Bool); } threads { inc.out -> cnt.inc; cnt.count -> cmp.lhs; cmp.out -> rise.in; rise.out -> hold.start; hold.active -> out.in; }
    }
}

/// C03: Press N times → edge Compare → monostable reward window.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c03_press_n_then_window().unwrap();
/// ```
pub fn run_c03_press_n_then_window() -> Result<()> {
    let w = c03_press_n_then_window_weave()?;
    let mut rt = bind_default(&w)?;
    let i = rt.sense_id("inc").unwrap();
    let mut h = ScriptedHost::new();
    tick_senses(&mut h, &mut rt, &[(i, ONE)])?;
    assert!(!signal_out_truthy(&rt, "reward"));
    tick_senses(&mut h, &mut rt, &[(i, ZERO)])?;
    tick_senses(&mut h, &mut rt, &[(i, ONE)])?;
    assert!(signal_out_truthy(&rt, "reward"));
    tick_senses(&mut h, &mut rt, &[(i, ZERO)])?;
    assert!(signal_out_truthy(&rt, "reward"));
    tick_senses(&mut h, &mut rt, &[(i, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "reward"));
    Ok(())
}

/// Topology for C04: edge shot and cooling cue.
pub fn c04_button_cooldown_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "c04"; knots { btn = KnotKind::signal_in(SignalDomain::Bool); edge = KnotKind::rising_from_zero(); hold = KnotKind::timer(TimerMode::PulseHold, 2); shot = KnotKind::signal_out("shot", SignalDomain::Bool); cool = KnotKind::signal_out("cooling", SignalDomain::Bool); } threads { btn.out -> edge.in; edge.out -> hold.start; edge.out -> shot.in; hold.active -> cool.in; }
    }
}

/// C04: Button edge shot + monostable cooling cue.
///
/// Wyrd Weaves are **DAGs** — you cannot gate Timer `start` on the same Timer's
/// `active` (cycle). **RisingFromZero** removes hold-spam; **PulseHold** is the
/// visible cooldown lamp. The host may suppress input while `cooling`.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c04_button_cooldown().unwrap();
/// ```
pub fn run_c04_button_cooldown() -> Result<()> {
    let w = c04_button_cooldown_weave()?;
    let mut rt = bind_default(&w)?;
    let b = rt.sense_id("btn").unwrap();
    let mut h = ScriptedHost::new();
    tick_senses(&mut h, &mut rt, &[(b, ONE)])?;
    assert!(signal_out_truthy(&rt, "shot"));
    assert!(signal_out_truthy(&rt, "cooling"));
    tick_senses(&mut h, &mut rt, &[(b, ONE)])?;
    assert!(!signal_out_truthy(&rt, "shot"));
    assert!(signal_out_truthy(&rt, "cooling"));
    tick_senses(&mut h, &mut rt, &[(b, ZERO)])?;
    tick_senses(&mut h, &mut rt, &[(b, ONE)])?;
    assert!(signal_out_truthy(&rt, "shot"));
    Ok(())
}

/// Topology for C05: threshold pressed and crossed-up output.
pub fn c05_axis_digital_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "c05"; knots { axis = KnotKind::signal_in(SignalDomain::Level); th = KnotKind::Threshold { domain: SignalDomain::Level, high: from_count(5), low: from_count(0), use_hysteresis: false }; pressed = KnotKind::signal_out("pressed", SignalDomain::Bool); just = KnotKind::signal_out("just_pressed", SignalDomain::Bool); } threads { axis.out -> th.in; th.out -> pressed.in; th.crossed_up -> just.in; }
    }
}

/// C05: AxisDigital — Threshold pressed + `crossed_up` pulse.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c05_axis_digital().unwrap();
/// ```
pub fn run_c05_axis_digital() -> Result<()> {
    let w = c05_axis_digital_weave()?;
    let mut rt = bind_default(&w)?;
    let a = rt.sense_id("axis").unwrap();
    let mut h = ScriptedHost::new();
    tick_senses(&mut h, &mut rt, &[(a, from_count(4))])?;
    assert!(!signal_out_truthy(&rt, "pressed"));
    assert!(!signal_out_truthy(&rt, "just_pressed"));
    tick_senses(&mut h, &mut rt, &[(a, from_count(5))])?;
    assert!(signal_out_truthy(&rt, "pressed"));
    assert!(signal_out_truthy(&rt, "just_pressed"));
    tick_senses(&mut h, &mut rt, &[(a, from_count(5))])?;
    assert!(signal_out_truthy(&rt, "pressed"));
    assert!(!signal_out_truthy(&rt, "just_pressed"));
    Ok(())
}

/// Topology for C06: count remapping.
pub fn c06_map_remap_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "c06"; knots { input as "in" = KnotKind::signal_in(SignalDomain::Count); map = KnotKind::Map { domain: SignalDomain::Count, in_min: ZERO, in_max: ONE, out_min: from_count(0), out_max: from_count(10) }; out = KnotKind::signal_out("scaled", SignalDomain::Count); } threads { input.out -> map.in; map.out -> out.in; }
    }
}

/// C06: Map remap ZERO..ONE → 0..10 counts.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c06_map_remap().unwrap();
/// ```
pub fn run_c06_map_remap() -> Result<()> {
    let w = c06_map_remap_weave()?;
    let mut rt = bind_default(&w)?;
    let i = rt.sense_id("in").unwrap();
    let mut h = ScriptedHost::new();
    tick_senses(&mut h, &mut rt, &[(i, ZERO)])?;
    assert_eq!(signal_out_value(&rt, "scaled"), from_count(0));
    tick_senses(&mut h, &mut rt, &[(i, ONE)])?;
    assert_eq!(signal_out_value(&rt, "scaled"), from_count(10));
    Ok(())
}

/// Topology for C07: digitized levels.
pub fn c07_digitize_steps_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "c07"; knots { input as "in" = KnotKind::signal_in(SignalDomain::Level); dig = KnotKind::Digitize { domain: SignalDomain::Level, steps: 2, in_min: ZERO, in_max: ONE, out_min: from_count(0), out_max: from_count(1) }; out = KnotKind::signal_out("bin", SignalDomain::Level); } threads { input.out -> dig.in; dig.out -> out.in; }
    }
}

/// C07: Digitize into steps over ZERO..ONE.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c07_digitize_steps().unwrap();
/// ```
pub fn run_c07_digitize_steps() -> Result<()> {
    let w = c07_digitize_steps_weave()?;
    let mut rt = bind_default(&w)?;
    let i = rt.sense_id("in").unwrap();
    let mut h = ScriptedHost::new();
    tick_senses(&mut h, &mut rt, &[(i, ZERO)])?;
    assert_eq!(signal_out_value(&rt, "bin"), from_count(0));
    tick_senses(&mut h, &mut rt, &[(i, ONE)])?;
    assert_eq!(signal_out_value(&rt, "bin"), from_count(1));
    Ok(())
}

/// Topology for C08: OnStart latches boot state.
pub fn c08_on_start_once_weave() -> core::result::Result<Weave, BuildError> {
    weave! { id: "c08"; knots { start = KnotKind::OnStart; flag = KnotKind::flag(FlagPriority::SetWins, false); out = KnotKind::signal_out("booted", SignalDomain::Bool); } threads { start.out -> flag.set; flag.out -> out.in; } }
}

/// C08: OnStart latches Flag once (second tick OnStart is falsey).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c08_on_start_once().unwrap();
/// ```
pub fn run_c08_on_start_once() -> Result<()> {
    let w = c08_on_start_once_weave()?;
    let mut rt = bind_default(&w)?;
    let mut h = ScriptedHost::new();
    tick_senses(&mut h, &mut rt, &[])?;
    assert!(signal_out_truthy(&rt, "booted"));
    tick_senses(&mut h, &mut rt, &[])?;
    assert!(signal_out_truthy(&rt, "booted"));
    Ok(())
}

/// Topology for C09: one-shot command emission.
pub fn c09_emit_once_weave() -> core::result::Result<Weave, BuildError> {
    weave! { id: "c09"; knots { ok = KnotKind::signal_in(SignalDomain::Bool); edge = KnotKind::rising_from_zero(); em = KnotKind::emit_command("sfx.ping"); } threads { ok.out -> edge.in; edge.out -> em.trigger; } }
}

/// C09: Emit once — level → Rising → Emit.trigger (never level→Emit forever).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c09_emit_once().unwrap();
/// ```
pub fn run_c09_emit_once() -> Result<()> {
    let w = c09_emit_once_weave()?;
    let mut rt = bind_default(&w)?;
    let o = rt.sense_id("ok").unwrap();
    let mut h = ScriptedHost::new();
    tick_senses(&mut h, &mut rt, &[(o, ONE)])?;
    assert_eq!(emit_count(&rt, "sfx.ping"), 1);
    tick_senses(&mut h, &mut rt, &[(o, ONE)])?;
    assert_eq!(emit_count(&rt, "sfx.ping"), 0);
    Ok(())
}

/// Topology for C10: either key opens the output.
pub fn c10_or_any_of_keys_weave() -> core::result::Result<Weave, BuildError> {
    weave! { id: "c10"; knots { key_a = KnotKind::signal_in(SignalDomain::Bool); key_b = KnotKind::signal_in(SignalDomain::Bool); any = KnotKind::or2(); out = KnotKind::signal_out("open", SignalDomain::Bool); } threads { key_a.out -> any.in_0; key_b.out -> any.in_1; any.out -> out.in; } }
}

/// C10: Or any-of keys.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c10_or_any_of_keys().unwrap();
/// ```
pub fn run_c10_or_any_of_keys() -> Result<()> {
    let w = c10_or_any_of_keys_weave()?;
    let mut rt = bind_default(&w)?;
    let a = rt.sense_id("key_a").unwrap();
    let b = rt.sense_id("key_b").unwrap();
    let mut h = ScriptedHost::new();
    tick_senses(&mut h, &mut rt, &[(a, ZERO), (b, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "open"));
    tick_senses(&mut h, &mut rt, &[(a, ONE), (b, ZERO)])?;
    assert!(signal_out_truthy(&rt, "open"));
    tick_senses(&mut h, &mut rt, &[(a, ZERO), (b, ONE)])?;
    assert!(signal_out_truthy(&rt, "open"));
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
    fn c03_c05_c06_and_c07_keep_duplicate_knot_diagnostics() {
        assert_duplicate_knot_rejected(&c03_press_n_then_window_weave().unwrap(), "cmp");
        let c05 = c05_axis_digital_weave().unwrap();
        for id in ["th", "pressed", "just"] {
            assert_duplicate_knot_rejected(&c05, id);
        }
        assert_duplicate_knot_rejected(&c06_map_remap_weave().unwrap(), "map");
        assert_duplicate_knot_rejected(&c07_digitize_steps_weave().unwrap(), "dig");
    }
}
