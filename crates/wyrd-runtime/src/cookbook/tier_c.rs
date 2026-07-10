//! Tier C — GBG / Zelda literacy (softlock-aware compositions).
//!
//! Full Weave listings under each function’s **Examples** in rustdoc.

use super::helpers::{bind_default, emit_count, signal_out_truthy, signal_out_value, tick_senses};
use super::Result;
use crate::host::ScriptedHost;
use wyrd_core::{from_count, CompareOp, FlagPriority, KnotKind, TimerMode, ONE, ZERO};
use wyrd_graph::Weave;

/// C01: MultiSwitchLatch — both plates once together → Flag until reset.
///
/// # Examples
///
/// ```
/// wyrd_runtime::cookbook::tier_c::run_c01_multi_switch_latch().unwrap();
/// ```
pub fn run_c01_multi_switch_latch() -> Result<()> {
    let mut b = Weave::builder("c01")?;
    let pa = b.knot("a", KnotKind::signal_in())?;
    let pb = b.knot("b", KnotKind::signal_in())?;
    let k_both = b.knot("both", KnotKind::and2())?;
    let from = b.output(&pa, "out")?;
    let to = b.input(&k_both, "in_0")?;
    b.connect(from, to)?;
    let from = b.output(&pb, "out")?;
    let to = b.input(&k_both, "in_1")?;
    b.connect(from, to)?;
    let k_edge = b.knot("edge", KnotKind::rising_from_zero())?;
    let k_rst = b.knot("rst", KnotKind::signal_in())?;
    let k_flag = b.knot("flag", KnotKind::flag(FlagPriority::ResetWins, false))?;
    let k_out = b.knot("out", KnotKind::signal_out("door.open"))?;
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
/// wyrd_runtime::cookbook::tier_c::run_c02_timed_hold().unwrap();
/// ```
pub fn run_c02_timed_hold() -> Result<()> {
    let mut b = Weave::builder("c02")?;
    let k_plate = b.knot("plate", KnotKind::signal_in())?;
    let k_t = b.knot("t", KnotKind::timer(TimerMode::FedCountdown, 2))?;
    let k_out = b.knot("out", KnotKind::signal_out("unlocked"))?;
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
/// wyrd_runtime::cookbook::tier_c::run_c03_press_n_then_window().unwrap();
/// ```
pub fn run_c03_press_n_then_window() -> Result<()> {
    let mut b = Weave::builder("c03")?;
    let k_inc = b.knot("inc", KnotKind::signal_in())?;
    let k_cnt = b.knot("cnt", KnotKind::counter())?;
    let k_cmp = b.knot("cmp", KnotKind::compare(CompareOp::Gte, Some(2)))?;
    let k_rise = b.knot("rise", KnotKind::rising_from_zero())?;
    let k_hold = b.knot("hold", KnotKind::timer(TimerMode::PulseHold, 2))?;
    let k_out = b.knot("out", KnotKind::signal_out("reward"))?;
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
/// wyrd_runtime::cookbook::tier_c::run_c04_button_cooldown().unwrap();
/// ```
pub fn run_c04_button_cooldown() -> Result<()> {
    let mut b = Weave::builder("c04")?;
    let k_btn = b.knot("btn", KnotKind::signal_in())?;
    let k_edge = b.knot("edge", KnotKind::rising_from_zero())?;
    let k_hold = b.knot("hold", KnotKind::timer(TimerMode::PulseHold, 2))?;
    let k_shot = b.knot("shot", KnotKind::signal_out("shot"))?;
    let k_cool = b.knot("cool", KnotKind::signal_out("cooling"))?;
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
/// wyrd_runtime::cookbook::tier_c::run_c05_axis_digital().unwrap();
/// ```
pub fn run_c05_axis_digital() -> Result<()> {
    let mut b = Weave::builder("c05")?;
    let k_axis = b.knot("axis", KnotKind::signal_in())?;
    let k_th = b.knot(
        "th",
        KnotKind::Threshold {
            high: from_count(5),
            low: from_count(0),
            use_hysteresis: false,
        },
    )?;
    let k_pressed = b.knot("pressed", KnotKind::signal_out("pressed"))?;
    let k_just = b.knot("just", KnotKind::signal_out("just_pressed"))?;
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
/// wyrd_runtime::cookbook::tier_c::run_c06_map_remap().unwrap();
/// ```
pub fn run_c06_map_remap() -> Result<()> {
    let mut b = Weave::builder("c06")?;
    let k_in = b.knot("in", KnotKind::signal_in())?;
    let k_map = b.knot(
        "map",
        KnotKind::Map {
            in_min: ZERO,
            in_max: ONE,
            out_min: from_count(0),
            out_max: from_count(10),
        },
    )?;
    let k_out = b.knot("out", KnotKind::signal_out("scaled"))?;
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
/// wyrd_runtime::cookbook::tier_c::run_c07_digitize_steps().unwrap();
/// ```
pub fn run_c07_digitize_steps() -> Result<()> {
    let mut b = Weave::builder("c07")?;
    let k_in = b.knot("in", KnotKind::signal_in())?;
    let k_dig = b.knot(
        "dig",
        KnotKind::Digitize {
            steps: 2,
            in_min: ZERO,
            in_max: ONE,
            out_min: from_count(0),
            out_max: from_count(1),
        },
    )?;
    let k_out = b.knot("out", KnotKind::signal_out("bin"))?;
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
/// wyrd_runtime::cookbook::tier_c::run_c08_on_start_once().unwrap();
/// ```
pub fn run_c08_on_start_once() -> Result<()> {
    let mut b = Weave::builder("c08")?;
    let k_start = b.knot("start", KnotKind::OnStart)?;
    let k_flag = b.knot("flag", KnotKind::flag(FlagPriority::SetWins, false))?;
    let k_out = b.knot("out", KnotKind::signal_out("booted"))?;
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
/// wyrd_runtime::cookbook::tier_c::run_c09_emit_once().unwrap();
/// ```
pub fn run_c09_emit_once() -> Result<()> {
    let mut b = Weave::builder("c09")?;
    let k_ok = b.knot("ok", KnotKind::signal_in())?;
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
/// wyrd_runtime::cookbook::tier_c::run_c10_or_any_of_keys().unwrap();
/// ```
pub fn run_c10_or_any_of_keys() -> Result<()> {
    let mut b = Weave::builder("c10")?;
    let ka = b.knot("key_a", KnotKind::signal_in())?;
    let kb = b.knot("key_b", KnotKind::signal_in())?;
    let or_id = b.knot("any", KnotKind::or2())?;
    let k_out = b.knot("out", KnotKind::signal_out("open"))?;
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
