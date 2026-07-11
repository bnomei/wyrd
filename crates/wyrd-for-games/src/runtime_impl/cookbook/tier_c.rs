//! Tier C — GBG / Zelda literacy (softlock-aware compositions).
//!
//! Full Weave listings under each function’s **Examples** in rustdoc.

#![allow(clippy::result_large_err)] // CookbookError intentionally preserves context.

use super::helpers::{bind_default, emit_count, signal_out_truthy, signal_out_value, tick_senses};
use super::Result;
use crate::authoring::Weave;
use crate::foundation::{
    from_count, CompareOp, FlagPriority, KnotKind, SignalDomain, TimerMode, ONE, ZERO,
};
use crate::runtime_impl::host::ScriptedHost;

fn duplicate_knot_id<'a>(
    failure_at: Option<&str>,
    target: &str,
    default: &'a str,
    duplicate: &'a str,
) -> &'a str {
    if failure_at == Some(target) {
        duplicate
    } else {
        default
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
    let mut b = Weave::builder("c01")?;
    let pa = b.knot("a", KnotKind::signal_in(SignalDomain::Bool))?;
    let pb = b.knot("b", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_both = b.knot("both", KnotKind::and2())?;
    let from = b.output(&pa, "out")?;
    let to = b.input(&k_both, "in_0")?;
    b.connect(from, to)?;
    let from = b.output(&pb, "out")?;
    let to = b.input(&k_both, "in_1")?;
    b.connect(from, to)?;
    let k_edge = b.knot("edge", KnotKind::rising_from_zero())?;
    let k_rst = b.knot("rst", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_flag = b.knot("flag", KnotKind::flag(FlagPriority::ResetWins, false))?;
    let k_out = b.knot("out", KnotKind::signal_out("door.open", SignalDomain::Bool))?;
    let from = b.output(&k_both, "out")?;
    let to = b.input(&k_edge, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_edge, "out")?;
    let to = b.input(&k_flag, "set")?;
    b.connect(from, to)?;
    let from = b.output(&k_rst, "out")?;
    let to = b.input(&k_flag, "reset")?;
    b.connect(from, to)?;
    let from = b.output(&k_flag, "out")?;
    let to = b.input(&k_out, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

    let mut rt = bind_default(&weave)?;
    let a = rt.sense_id("a").expect("a");
    let b_id = rt.sense_id("b").expect("b");
    let rst = rt.sense_id("rst").expect("rst");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &[(a, ONE), (b_id, ZERO), (rst, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "door.open"));
    tick_senses(&mut host, &mut rt, &[(a, ONE), (b_id, ONE), (rst, ZERO)])?;
    assert!(signal_out_truthy(&rt, "door.open"));
    tick_senses(&mut host, &mut rt, &[(a, ZERO), (b_id, ZERO), (rst, ZERO)])?;
    assert!(signal_out_truthy(&rt, "door.open"));
    tick_senses(&mut host, &mut rt, &[(a, ZERO), (b_id, ZERO), (rst, ONE)])?;
    assert!(!signal_out_truthy(&rt, "door.open"));
    Ok(())
}

/// C02: TimedHold — FedCountdown while plate held.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c02_timed_hold().unwrap();
/// ```
pub fn run_c02_timed_hold() -> Result<()> {
    let mut b = Weave::builder("c02")?;
    let k_plate = b.knot("plate", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_t = b.knot("t", KnotKind::timer(TimerMode::FedCountdown, 2))?;
    let k_out = b.knot("out", KnotKind::signal_out("unlocked", SignalDomain::Bool))?;
    let from = b.output(&k_plate, "out")?;
    let to = b.input(&k_t, "feed")?;
    b.connect(from, to)?;
    let from = b.output(&k_t, "active")?;
    let to = b.input(&k_out, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

    let mut rt = bind_default(&weave)?;
    let plate = rt.sense_id("plate").expect("plate");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &[(plate, ONE)])?;
    assert!(!signal_out_truthy(&rt, "unlocked"));
    tick_senses(&mut host, &mut rt, &[(plate, ONE)])?;
    assert!(signal_out_truthy(&rt, "unlocked"));
    tick_senses(&mut host, &mut rt, &[(plate, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "unlocked"));
    Ok(())
}

/// C03: Press N times → edge Compare → monostable reward window.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c03_press_n_then_window().unwrap();
/// ```
pub fn run_c03_press_n_then_window() -> Result<()> {
    run_c03_press_n_then_window_with(None)
}

fn run_c03_press_n_then_window_with(failure_at: Option<&str>) -> Result<()> {
    let mut b = Weave::builder("c03")?;
    let k_inc = b.knot("inc", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_cnt = b.knot("cnt", KnotKind::counter())?;
    let k_cmp = b.knot(
        duplicate_knot_id(failure_at, "c03.compare", "cmp", "inc"),
        KnotKind::compare(CompareOp::Gte, Some(from_count(2)), SignalDomain::Count),
    )?;
    let k_rise = b.knot("rise", KnotKind::rising_from_zero())?;
    let k_hold = b.knot("hold", KnotKind::timer(TimerMode::PulseHold, 2))?;
    let k_out = b.knot("out", KnotKind::signal_out("reward", SignalDomain::Bool))?;
    let from = b.output(&k_inc, "out")?;
    let to = b.input(&k_cnt, "inc")?;
    b.connect(from, to)?;
    let from = b.output(&k_cnt, "count")?;
    let to = b.input(&k_cmp, "lhs")?;
    b.connect(from, to)?;
    let from = b.output(&k_cmp, "out")?;
    let to = b.input(&k_rise, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_rise, "out")?;
    let to = b.input(&k_hold, "start")?;
    b.connect(from, to)?;
    let from = b.output(&k_hold, "active")?;
    let to = b.input(&k_out, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

    let mut rt = bind_default(&weave)?;
    let inc = rt.sense_id("inc").expect("inc");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &[(inc, ONE)])?;
    assert!(!signal_out_truthy(&rt, "reward"));
    tick_senses(&mut host, &mut rt, &[(inc, ZERO)])?;
    tick_senses(&mut host, &mut rt, &[(inc, ONE)])?;
    assert!(signal_out_truthy(&rt, "reward"));
    tick_senses(&mut host, &mut rt, &[(inc, ZERO)])?;
    assert!(signal_out_truthy(&rt, "reward"));
    tick_senses(&mut host, &mut rt, &[(inc, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "reward"));
    Ok(())
}

/// C04: Button edge shot + monostable cooling cue.
///
/// Wyrd Weaves are **DAGs** — you cannot gate Timer `start` on the same Timer’s
/// `active` (cycle). Teach: **RisingFromZero** kills hold-spam; **PulseHold**
/// is the visible cooldown lamp. Host may suppress input while `cooling`.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c04_button_cooldown().unwrap();
/// ```
pub fn run_c04_button_cooldown() -> Result<()> {
    let mut b = Weave::builder("c04")?;
    let k_btn = b.knot("btn", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_edge = b.knot("edge", KnotKind::rising_from_zero())?;
    let k_hold = b.knot("hold", KnotKind::timer(TimerMode::PulseHold, 2))?;
    let k_shot = b.knot("shot", KnotKind::signal_out("shot", SignalDomain::Bool))?;
    let k_cool = b.knot("cool", KnotKind::signal_out("cooling", SignalDomain::Bool))?;
    let from = b.output(&k_btn, "out")?;
    let to = b.input(&k_edge, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_edge, "out")?;
    let to = b.input(&k_hold, "start")?;
    b.connect(from, to)?;
    let from = b.output(&k_edge, "out")?;
    let to = b.input(&k_shot, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_hold, "active")?;
    let to = b.input(&k_cool, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

    let mut rt = bind_default(&weave)?;
    let btn = rt.sense_id("btn").expect("btn");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &[(btn, ONE)])?;
    assert!(signal_out_truthy(&rt, "shot"));
    assert!(signal_out_truthy(&rt, "cooling"));
    tick_senses(&mut host, &mut rt, &[(btn, ONE)])?;
    assert!(!signal_out_truthy(&rt, "shot"));
    assert!(signal_out_truthy(&rt, "cooling"));
    tick_senses(&mut host, &mut rt, &[(btn, ZERO)])?;
    tick_senses(&mut host, &mut rt, &[(btn, ONE)])?;
    assert!(signal_out_truthy(&rt, "shot"));
    Ok(())
}

/// C05: AxisDigital — Threshold pressed + `crossed_up` pulse.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c05_axis_digital().unwrap();
/// ```
pub fn run_c05_axis_digital() -> Result<()> {
    run_c05_axis_digital_with(None)
}

fn run_c05_axis_digital_with(failure_at: Option<&str>) -> Result<()> {
    let mut b = Weave::builder("c05")?;
    let k_axis = b.knot("axis", KnotKind::signal_in(SignalDomain::Level))?;
    let k_th = b.knot(
        duplicate_knot_id(failure_at, "c05.threshold", "th", "axis"),
        KnotKind::Threshold {
            domain: SignalDomain::Level,
            high: from_count(5),
            low: from_count(0),
            use_hysteresis: false,
        },
    )?;
    let k_pressed = b.knot(
        duplicate_knot_id(failure_at, "c05.pressed", "pressed", "axis"),
        KnotKind::signal_out("pressed", SignalDomain::Bool),
    )?;
    let k_just = b.knot(
        duplicate_knot_id(failure_at, "c05.just_pressed", "just", "axis"),
        KnotKind::signal_out("just_pressed", SignalDomain::Bool),
    )?;
    let from = b.output(&k_axis, "out")?;
    let to = b.input(&k_th, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_th, "out")?;
    let to = b.input(&k_pressed, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_th, "crossed_up")?;
    let to = b.input(&k_just, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

    let mut rt = bind_default(&weave)?;
    let axis = rt.sense_id("axis").expect("axis");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &[(axis, from_count(4))])?;
    assert!(!signal_out_truthy(&rt, "pressed"));
    assert!(!signal_out_truthy(&rt, "just_pressed"));
    tick_senses(&mut host, &mut rt, &[(axis, from_count(5))])?;
    assert!(signal_out_truthy(&rt, "pressed"));
    assert!(signal_out_truthy(&rt, "just_pressed"));
    tick_senses(&mut host, &mut rt, &[(axis, from_count(5))])?;
    assert!(signal_out_truthy(&rt, "pressed"));
    assert!(!signal_out_truthy(&rt, "just_pressed"));
    Ok(())
}

/// C06: Map remap ZERO..ONE → 0..10 counts.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c06_map_remap().unwrap();
/// ```
pub fn run_c06_map_remap() -> Result<()> {
    run_c06_map_remap_with(None)
}

fn run_c06_map_remap_with(failure_at: Option<&str>) -> Result<()> {
    let mut b = Weave::builder("c06")?;
    let k_in = b.knot("in", KnotKind::signal_in(SignalDomain::Count))?;
    let k_map = b.knot(
        duplicate_knot_id(failure_at, "c06.map", "map", "in"),
        KnotKind::Map {
            domain: SignalDomain::Count,
            in_min: ZERO,
            in_max: ONE,
            out_min: from_count(0),
            out_max: from_count(10),
        },
    )?;
    let k_out = b.knot("out", KnotKind::signal_out("scaled", SignalDomain::Count))?;
    let from = b.output(&k_in, "out")?;
    let to = b.input(&k_map, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_map, "out")?;
    let to = b.input(&k_out, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

    let mut rt = bind_default(&weave)?;
    let id = rt.sense_id("in").expect("in");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &[(id, ZERO)])?;
    assert_eq!(signal_out_value(&rt, "scaled"), from_count(0));
    tick_senses(&mut host, &mut rt, &[(id, ONE)])?;
    assert_eq!(signal_out_value(&rt, "scaled"), from_count(10));
    Ok(())
}

/// C07: Digitize into steps over ZERO..ONE.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c07_digitize_steps().unwrap();
/// ```
pub fn run_c07_digitize_steps() -> Result<()> {
    run_c07_digitize_steps_with(None)
}

fn run_c07_digitize_steps_with(failure_at: Option<&str>) -> Result<()> {
    let mut b = Weave::builder("c07")?;
    let k_in = b.knot("in", KnotKind::signal_in(SignalDomain::Level))?;
    let k_dig = b.knot(
        duplicate_knot_id(failure_at, "c07.digitize", "dig", "in"),
        KnotKind::Digitize {
            domain: SignalDomain::Level,
            steps: 2,
            in_min: ZERO,
            in_max: ONE,
            out_min: from_count(0),
            out_max: from_count(1),
        },
    )?;
    let k_out = b.knot("out", KnotKind::signal_out("bin", SignalDomain::Level))?;
    let from = b.output(&k_in, "out")?;
    let to = b.input(&k_dig, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_dig, "out")?;
    let to = b.input(&k_out, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

    let mut rt = bind_default(&weave)?;
    let id = rt.sense_id("in").expect("in");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &[(id, ZERO)])?;
    assert_eq!(signal_out_value(&rt, "bin"), from_count(0));
    tick_senses(&mut host, &mut rt, &[(id, ONE)])?;
    assert_eq!(signal_out_value(&rt, "bin"), from_count(1));
    Ok(())
}

/// C08: OnStart latches Flag once (second tick OnStart is falsey).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c08_on_start_once().unwrap();
/// ```
pub fn run_c08_on_start_once() -> Result<()> {
    let mut b = Weave::builder("c08")?;
    let k_start = b.knot("start", KnotKind::OnStart)?;
    let k_flag = b.knot("flag", KnotKind::flag(FlagPriority::SetWins, false))?;
    let k_out = b.knot("out", KnotKind::signal_out("booted", SignalDomain::Bool))?;
    let from = b.output(&k_start, "out")?;
    let to = b.input(&k_flag, "set")?;
    b.connect(from, to)?;
    let from = b.output(&k_flag, "out")?;
    let to = b.input(&k_out, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

    let mut rt = bind_default(&weave)?;
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &[])?;
    assert!(signal_out_truthy(&rt, "booted"));
    tick_senses(&mut host, &mut rt, &[])?;
    assert!(signal_out_truthy(&rt, "booted"));
    Ok(())
}

/// C09: Emit once — level → Rising → Emit.trigger (never level→Emit forever).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c09_emit_once().unwrap();
/// ```
pub fn run_c09_emit_once() -> Result<()> {
    let mut b = Weave::builder("c09")?;
    let k_ok = b.knot("ok", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_edge = b.knot("edge", KnotKind::rising_from_zero())?;
    let k_em = b.knot("em", KnotKind::emit_command("sfx.ping"))?;
    let from = b.output(&k_ok, "out")?;
    let to = b.input(&k_edge, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_edge, "out")?;
    let to = b.input(&k_em, "trigger")?;
    b.connect(from, to)?;
    let weave = b.build()?;

    let mut rt = bind_default(&weave)?;
    let ok = rt.sense_id("ok").expect("ok");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &[(ok, ONE)])?;
    assert_eq!(emit_count(&rt, "sfx.ping"), 1);
    tick_senses(&mut host, &mut rt, &[(ok, ONE)])?;
    assert_eq!(emit_count(&rt, "sfx.ping"), 0);
    Ok(())
}

/// C10: Or any-of keys.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_c::run_c10_or_any_of_keys().unwrap();
/// ```
pub fn run_c10_or_any_of_keys() -> Result<()> {
    let mut b = Weave::builder("c10")?;
    let ka = b.knot("key_a", KnotKind::signal_in(SignalDomain::Bool))?;
    let kb = b.knot("key_b", KnotKind::signal_in(SignalDomain::Bool))?;
    let or_id = b.knot("any", KnotKind::or2())?;
    let k_out = b.knot("out", KnotKind::signal_out("open", SignalDomain::Bool))?;
    let from = b.output(&ka, "out")?;
    let to = b.input(&or_id, "in_0")?;
    b.connect(from, to)?;
    let from = b.output(&kb, "out")?;
    let to = b.input(&or_id, "in_1")?;
    b.connect(from, to)?;
    let from = b.output(&or_id, "out")?;
    let to = b.input(&k_out, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

    let mut rt = bind_default(&weave)?;
    let a = rt.sense_id("key_a").expect("a");
    let b_id = rt.sense_id("key_b").expect("b");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &[(a, ZERO), (b_id, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "open"));
    tick_senses(&mut host, &mut rt, &[(a, ONE), (b_id, ZERO)])?;
    assert!(signal_out_truthy(&rt, "open"));
    tick_senses(&mut host, &mut rt, &[(a, ZERO), (b_id, ONE)])?;
    assert!(signal_out_truthy(&rt, "open"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c03_duplicate_compare_propagates_the_real_builder_error() {
        assert!(run_c03_press_n_then_window_with(Some("c03.compare")).is_err());
    }

    #[test]
    fn c05_duplicate_knots_propagate_the_real_builder_error() {
        for failure_at in ["c05.threshold", "c05.pressed", "c05.just_pressed"] {
            assert!(run_c05_axis_digital_with(Some(failure_at)).is_err());
        }
    }

    #[test]
    fn c06_duplicate_map_propagates_the_real_builder_error() {
        assert!(run_c06_map_remap_with(Some("c06.map")).is_err());
    }

    #[test]
    fn c07_duplicate_digitize_propagates_the_real_builder_error() {
        assert!(run_c07_digitize_steps_with(Some("c07.digitize")).is_err());
    }
}
