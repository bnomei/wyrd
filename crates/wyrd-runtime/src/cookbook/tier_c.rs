//! Tier C — GBG / Zelda literacy (softlock-aware compositions).
//!
//! Full Weave listings under each function’s **Examples** in rustdoc.

use super::helpers::{bind_default, emit_count, signal_out_truthy, signal_out_value, tick_senses};
use crate::host::ScriptedHost;
use wyrd_core::{
    from_count, CompareOp, FlagPriority, KnotKind, PortSlot, Result, TimerMode, ONE, ZERO,
};
use wyrd_graph::Weave;

/// C01: MultiSwitchLatch — both plates once together → Flag until reset.
///
/// # Examples
///
/// ```
/// use wyrd_core::{FlagPriority, KnotKind, ONE, ZERO};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::ScriptedHost;
/// use wyrd_runtime::cookbook::helpers::{bind_default, signal_out_truthy, tick_senses};
///
/// // a,b ──► And ──► Rising ──► Flag.set
/// // rst ──► Flag.reset
/// // Flag ──► door.open  (stays open after plates leave)
/// let (b, pa) = Weave::builder("c01").knot("a", KnotKind::signal_in()).unwrap();
/// let (b, pb) = b.knot("b", KnotKind::signal_in()).unwrap();
/// let (b, _) = b.and2("both", pa, pb).unwrap();
/// let (b, _) = b.knot("edge", KnotKind::rising_from_zero()).unwrap();
/// let (b, _) = b.knot("rst", KnotKind::signal_in()).unwrap();
/// let (b, _) = b
///     .knot("flag", KnotKind::flag(FlagPriority::ResetWins, false))
///     .unwrap();
/// let (b, _) = b.knot("out", KnotKind::signal_out("door.open")).unwrap();
/// let weave = b
///     .wire_named("both", "out", "edge", "in")
///     .wire_named("edge", "out", "flag", "set")
///     .wire_named("rst", "out", "flag", "reset")
///     .wire_named("flag", "out", "out", "in")
///     .build()
///     .unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let a = rt.sense_id("a").unwrap();
/// let b_id = rt.sense_id("b").unwrap();
/// let rst = rt.sense_id("rst").unwrap();
/// let mut host = ScriptedHost::new();
/// tick_senses(&mut host, &mut rt, &weave, &[(a, ONE), (b_id, ONE), (rst, ZERO)]).unwrap();
/// assert!(signal_out_truthy(&rt, "door.open"));
/// tick_senses(&mut host, &mut rt, &weave, &[(a, ZERO), (b_id, ZERO), (rst, ZERO)]).unwrap();
/// assert!(signal_out_truthy(&rt, "door.open")); // latched
/// ```
pub fn run_c01_multi_switch_latch() -> Result<()> {
    let (b, pa) = Weave::builder("c01").knot("a", KnotKind::signal_in())?;
    let (b, pb) = b.knot("b", KnotKind::signal_in())?;
    let (b, _) = b.and2("both", pa, pb)?;
    let (b, _) = b.knot("edge", KnotKind::rising_from_zero())?;
    let (b, _) = b.knot("rst", KnotKind::signal_in())?;
    let (b, _) = b.knot("flag", KnotKind::flag(FlagPriority::ResetWins, false))?;
    let (b, _) = b.knot("out", KnotKind::signal_out("door.open"))?;
    let weave = b
        .wire_named("both", "out", "edge", "in")
        .wire_named("edge", "out", "flag", "set")
        .wire_named("rst", "out", "flag", "reset")
        .wire_named("flag", "out", "out", "in")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let a = rt.sense_id("a").expect("a");
    let b_id = rt.sense_id("b").expect("b");
    let rst = rt.sense_id("rst").expect("rst");
    let mut host = ScriptedHost::new();

    tick_senses(
        &mut host,
        &mut rt,
        &weave,
        &[(a, ONE), (b_id, ZERO), (rst, ZERO)],
    )?;
    assert!(!signal_out_truthy(&rt, "door.open"));
    tick_senses(
        &mut host,
        &mut rt,
        &weave,
        &[(a, ONE), (b_id, ONE), (rst, ZERO)],
    )?;
    assert!(signal_out_truthy(&rt, "door.open"));
    tick_senses(
        &mut host,
        &mut rt,
        &weave,
        &[(a, ZERO), (b_id, ZERO), (rst, ZERO)],
    )?;
    assert!(signal_out_truthy(&rt, "door.open"));
    tick_senses(
        &mut host,
        &mut rt,
        &weave,
        &[(a, ZERO), (b_id, ZERO), (rst, ONE)],
    )?;
    assert!(!signal_out_truthy(&rt, "door.open"));
    Ok(())
}

/// C02: TimedHold — FedCountdown while plate held.
///
/// # Examples
///
/// ```
/// use wyrd_core::{KnotKind, TimerMode, ONE, ZERO};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::ScriptedHost;
/// use wyrd_runtime::cookbook::helpers::{bind_default, signal_out_truthy, tick_senses};
///
/// // plate ──feed──► Timer(FedCountdown, 2).active ──► unlocked
/// // Leave plate → countdown resets (soft cancel).
/// let (b, _) = Weave::builder("c02")
///     .knot("plate", KnotKind::signal_in())
///     .unwrap();
/// let (b, _) = b
///     .knot("t", KnotKind::timer(TimerMode::FedCountdown, 2))
///     .unwrap();
/// let (b, _) = b.knot("out", KnotKind::signal_out("unlocked")).unwrap();
/// let weave = b
///     .wire_named("plate", "out", "t", "feed")
///     .wire_named("t", "active", "out", "in")
///     .build()
///     .unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let plate = rt.sense_id("plate").unwrap();
/// let mut host = ScriptedHost::new();
/// tick_senses(&mut host, &mut rt, &weave, &[(plate, ONE)]).unwrap();
/// assert!(!signal_out_truthy(&rt, "unlocked"));
/// tick_senses(&mut host, &mut rt, &weave, &[(plate, ONE)]).unwrap();
/// assert!(signal_out_truthy(&rt, "unlocked"));
/// ```
pub fn run_c02_timed_hold() -> Result<()> {
    let (b, _) = Weave::builder("c02").knot("plate", KnotKind::signal_in())?;
    let (b, _) = b.knot("t", KnotKind::timer(TimerMode::FedCountdown, 2))?;
    let (b, _) = b.knot("out", KnotKind::signal_out("unlocked"))?;
    let weave = b
        .wire_named("plate", "out", "t", "feed")
        .wire_named("t", "active", "out", "in")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let plate = rt.sense_id("plate").expect("plate");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(plate, ONE)])?;
    assert!(!signal_out_truthy(&rt, "unlocked"));
    tick_senses(&mut host, &mut rt, &weave, &[(plate, ONE)])?;
    assert!(signal_out_truthy(&rt, "unlocked"));
    tick_senses(&mut host, &mut rt, &weave, &[(plate, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "unlocked"));
    Ok(())
}

/// C03: Press N times → edge Compare → monostable reward window.
///
/// # Examples
///
/// ```
/// use wyrd_core::{CompareOp, KnotKind, TimerMode, ONE, ZERO};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::ScriptedHost;
/// use wyrd_runtime::cookbook::helpers::{bind_default, signal_out_truthy, tick_senses};
///
/// // inc ──► Counter ──count──► Compare(Gte 2)
/// //                              └──► Rising ──► PulseHold ──► reward
/// // Never wire held Compare into Timer.start (would re-trigger every tick).
/// let (b, _) = Weave::builder("c03")
///     .knot("inc", KnotKind::signal_in())
///     .unwrap();
/// let (b, _) = b.knot("cnt", KnotKind::counter()).unwrap();
/// let (b, _) = b
///     .knot("cmp", KnotKind::compare(CompareOp::Gte, Some(2)))
///     .unwrap();
/// let (b, _) = b.knot("rise", KnotKind::rising_from_zero()).unwrap();
/// let (b, _) = b.knot("hold", KnotKind::timer(TimerMode::PulseHold, 2)).unwrap();
/// let (b, _) = b.knot("out", KnotKind::signal_out("reward")).unwrap();
/// let weave = b
///     .wire_named("inc", "out", "cnt", "inc")
///     .wire_named("cnt", "count", "cmp", "lhs")
///     .wire_named("cmp", "out", "rise", "in")
///     .wire_named("rise", "out", "hold", "start")
///     .wire_named("hold", "active", "out", "in")
///     .build()
///     .unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let inc = rt.sense_id("inc").unwrap();
/// let mut host = ScriptedHost::new();
/// tick_senses(&mut host, &mut rt, &weave, &[(inc, ONE)]).unwrap();
/// tick_senses(&mut host, &mut rt, &weave, &[(inc, ZERO)]).unwrap();
/// tick_senses(&mut host, &mut rt, &weave, &[(inc, ONE)]).unwrap();
/// assert!(signal_out_truthy(&rt, "reward"));
/// ```
pub fn run_c03_press_n_then_window() -> Result<()> {
    let (b, _) = Weave::builder("c03").knot("inc", KnotKind::signal_in())?;
    let (b, _) = b.knot("cnt", KnotKind::counter())?;
    let (b, _) = b.knot("cmp", KnotKind::compare(CompareOp::Gte, Some(2)))?;
    let (b, _) = b.knot("rise", KnotKind::rising_from_zero())?;
    let (b, _) = b.knot("hold", KnotKind::timer(TimerMode::PulseHold, 2))?;
    let (b, _) = b.knot("out", KnotKind::signal_out("reward"))?;
    let weave = b
        .wire_named("inc", "out", "cnt", "inc")
        .wire_named("cnt", "count", "cmp", "lhs")
        .wire_named("cmp", "out", "rise", "in")
        .wire_named("rise", "out", "hold", "start")
        .wire_named("hold", "active", "out", "in")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let inc = rt.sense_id("inc").expect("inc");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(inc, ONE)])?;
    assert!(!signal_out_truthy(&rt, "reward"));
    tick_senses(&mut host, &mut rt, &weave, &[(inc, ZERO)])?;
    tick_senses(&mut host, &mut rt, &weave, &[(inc, ONE)])?;
    assert!(signal_out_truthy(&rt, "reward"));
    tick_senses(&mut host, &mut rt, &weave, &[(inc, ZERO)])?;
    assert!(signal_out_truthy(&rt, "reward"));
    tick_senses(&mut host, &mut rt, &weave, &[(inc, ZERO)])?;
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
/// use wyrd_core::{KnotKind, TimerMode, ONE, ZERO};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::ScriptedHost;
/// use wyrd_runtime::cookbook::helpers::{bind_default, signal_out_truthy, tick_senses};
///
/// // btn ──► Rising ──┬──► shot (one-tick pulse)
/// //                  └──► PulseHold.start
/// //         PulseHold.active ──► cooling (level cue)
/// let (b, _) = Weave::builder("c04")
///     .knot("btn", KnotKind::signal_in())
///     .unwrap();
/// let (b, _) = b.knot("edge", KnotKind::rising_from_zero()).unwrap();
/// let (b, _) = b.knot("hold", KnotKind::timer(TimerMode::PulseHold, 2)).unwrap();
/// let (b, _) = b.knot("shot", KnotKind::signal_out("shot")).unwrap();
/// let (b, _) = b.knot("cool", KnotKind::signal_out("cooling")).unwrap();
/// let weave = b
///     .wire_named("btn", "out", "edge", "in")
///     .wire_named("edge", "out", "hold", "start")
///     .wire_named("edge", "out", "shot", "in")
///     .wire_named("hold", "active", "cool", "in")
///     .build()
///     .unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let btn = rt.sense_id("btn").unwrap();
/// let mut host = ScriptedHost::new();
/// tick_senses(&mut host, &mut rt, &weave, &[(btn, ONE)]).unwrap();
/// assert!(signal_out_truthy(&rt, "shot"));
/// assert!(signal_out_truthy(&rt, "cooling"));
/// tick_senses(&mut host, &mut rt, &weave, &[(btn, ONE)]).unwrap();
/// assert!(!signal_out_truthy(&rt, "shot")); // held — no second edge
/// ```
pub fn run_c04_button_cooldown() -> Result<()> {
    let (b, _) = Weave::builder("c04").knot("btn", KnotKind::signal_in())?;
    let (b, _) = b.knot("edge", KnotKind::rising_from_zero())?;
    let (b, _) = b.knot("hold", KnotKind::timer(TimerMode::PulseHold, 2))?;
    let (b, _) = b.knot("shot", KnotKind::signal_out("shot"))?;
    let (b, _) = b.knot("cool", KnotKind::signal_out("cooling"))?;
    let weave = b
        .wire_named("btn", "out", "edge", "in")
        .wire_named("edge", "out", "hold", "start")
        .wire_named("edge", "out", "shot", "in")
        .wire_named("hold", "active", "cool", "in")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let btn = rt.sense_id("btn").expect("btn");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(btn, ONE)])?;
    assert!(signal_out_truthy(&rt, "shot"));
    assert!(signal_out_truthy(&rt, "cooling"));
    tick_senses(&mut host, &mut rt, &weave, &[(btn, ONE)])?;
    assert!(!signal_out_truthy(&rt, "shot"));
    assert!(signal_out_truthy(&rt, "cooling"));
    tick_senses(&mut host, &mut rt, &weave, &[(btn, ZERO)])?;
    tick_senses(&mut host, &mut rt, &weave, &[(btn, ONE)])?;
    assert!(signal_out_truthy(&rt, "shot"));
    Ok(())
}

/// C05: AxisDigital — Threshold pressed + `crossed_up` pulse.
///
/// # Examples
///
/// ```
/// use wyrd_core::{from_count, KnotKind};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::ScriptedHost;
/// use wyrd_runtime::cookbook::helpers::{bind_default, signal_out_truthy, tick_senses};
///
/// // axis ──► Threshold ──out──► pressed
/// //                    └──crossed_up──► just_pressed
/// let (b, _) = Weave::builder("c05")
///     .knot("axis", KnotKind::signal_in())
///     .unwrap();
/// let (b, _) = b
///     .knot(
///         "th",
///         KnotKind::Threshold {
///             high: from_count(5),
///             low: from_count(0),
///             use_hysteresis: false,
///         },
///     )
///     .unwrap();
/// let (b, _) = b.knot("pressed", KnotKind::signal_out("pressed")).unwrap();
/// let (b, _) = b.knot("just", KnotKind::signal_out("just_pressed")).unwrap();
/// let weave = b
///     .wire_named("axis", "out", "th", "in")
///     .wire_named("th", "out", "pressed", "in")
///     .wire_named("th", "crossed_up", "just", "in")
///     .build()
///     .unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let axis = rt.sense_id("axis").unwrap();
/// let mut host = ScriptedHost::new();
/// tick_senses(&mut host, &mut rt, &weave, &[(axis, from_count(5))]).unwrap();
/// assert!(signal_out_truthy(&rt, "pressed"));
/// assert!(signal_out_truthy(&rt, "just_pressed"));
/// ```
pub fn run_c05_axis_digital() -> Result<()> {
    let (b, _) = Weave::builder("c05").knot("axis", KnotKind::signal_in())?;
    let (b, _) = b.knot(
        "th",
        KnotKind::Threshold {
            high: from_count(5),
            low: from_count(0),
            use_hysteresis: false,
        },
    )?;
    let (b, _) = b.knot("pressed", KnotKind::signal_out("pressed"))?;
    let (b, _) = b.knot("just", KnotKind::signal_out("just_pressed"))?;
    let weave = b
        .wire_named("axis", "out", "th", "in")
        .wire_named("th", "out", "pressed", "in")
        .wire_named("th", "crossed_up", "just", "in")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let axis = rt.sense_id("axis").expect("axis");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(axis, from_count(4))])?;
    assert!(!signal_out_truthy(&rt, "pressed"));
    assert!(!signal_out_truthy(&rt, "just_pressed"));
    tick_senses(&mut host, &mut rt, &weave, &[(axis, from_count(5))])?;
    assert!(signal_out_truthy(&rt, "pressed"));
    assert!(signal_out_truthy(&rt, "just_pressed"));
    tick_senses(&mut host, &mut rt, &weave, &[(axis, from_count(5))])?;
    assert!(signal_out_truthy(&rt, "pressed"));
    assert!(!signal_out_truthy(&rt, "just_pressed"));
    Ok(())
}

/// C06: Map remap ZERO..ONE → 0..10 counts.
///
/// # Examples
///
/// ```
/// use wyrd_core::{from_count, KnotKind, ONE, ZERO};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::ScriptedHost;
/// use wyrd_runtime::cookbook::helpers::{bind_default, signal_out_value, tick_senses};
///
/// // in ──► Map(0..1 → 0..10) ──► scaled
/// let (b, _) = Weave::builder("c06")
///     .knot("in", KnotKind::signal_in())
///     .unwrap();
/// let (b, _) = b
///     .knot(
///         "map",
///         KnotKind::Map {
///             in_min: ZERO,
///             in_max: ONE,
///             out_min: from_count(0),
///             out_max: from_count(10),
///         },
///     )
///     .unwrap();
/// let (b, _) = b.knot("out", KnotKind::signal_out("scaled")).unwrap();
/// let weave = b
///     .wire_named("in", "out", "map", "in")
///     .wire_named("map", "out", "out", "in")
///     .build()
///     .unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let id = rt.sense_id("in").unwrap();
/// let mut host = ScriptedHost::new();
/// tick_senses(&mut host, &mut rt, &weave, &[(id, ONE)]).unwrap();
/// assert_eq!(signal_out_value(&rt, "scaled"), from_count(10));
/// ```
pub fn run_c06_map_remap() -> Result<()> {
    let (b, _) = Weave::builder("c06").knot("in", KnotKind::signal_in())?;
    let (b, _) = b.knot(
        "map",
        KnotKind::Map {
            in_min: ZERO,
            in_max: ONE,
            out_min: from_count(0),
            out_max: from_count(10),
        },
    )?;
    let (b, _) = b.knot("out", KnotKind::signal_out("scaled"))?;
    let weave = b
        .wire_named("in", "out", "map", "in")
        .wire_named("map", "out", "out", "in")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let id = rt.sense_id("in").expect("in");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(id, ZERO)])?;
    assert_eq!(signal_out_value(&rt, "scaled"), from_count(0));
    tick_senses(&mut host, &mut rt, &weave, &[(id, ONE)])?;
    assert_eq!(signal_out_value(&rt, "scaled"), from_count(10));
    Ok(())
}

/// C07: Digitize into steps over ZERO..ONE.
///
/// # Examples
///
/// ```
/// use wyrd_core::{from_count, KnotKind, ONE, ZERO};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::ScriptedHost;
/// use wyrd_runtime::cookbook::helpers::{bind_default, signal_out_value, tick_senses};
///
/// // in ──► Digitize(steps=2) ──► bin
/// let (b, _) = Weave::builder("c07")
///     .knot("in", KnotKind::signal_in())
///     .unwrap();
/// let (b, _) = b
///     .knot(
///         "dig",
///         KnotKind::Digitize {
///             steps: 2,
///             in_min: ZERO,
///             in_max: ONE,
///             out_min: from_count(0),
///             out_max: from_count(1),
///         },
///     )
///     .unwrap();
/// let (b, _) = b.knot("out", KnotKind::signal_out("bin")).unwrap();
/// let weave = b
///     .wire_named("in", "out", "dig", "in")
///     .wire_named("dig", "out", "out", "in")
///     .build()
///     .unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let id = rt.sense_id("in").unwrap();
/// let mut host = ScriptedHost::new();
/// tick_senses(&mut host, &mut rt, &weave, &[(id, ZERO)]).unwrap();
/// assert_eq!(signal_out_value(&rt, "bin"), from_count(0));
/// tick_senses(&mut host, &mut rt, &weave, &[(id, ONE)]).unwrap();
/// assert_eq!(signal_out_value(&rt, "bin"), from_count(1));
/// ```
pub fn run_c07_digitize_steps() -> Result<()> {
    let (b, _) = Weave::builder("c07").knot("in", KnotKind::signal_in())?;
    let (b, _) = b.knot(
        "dig",
        KnotKind::Digitize {
            steps: 2,
            in_min: ZERO,
            in_max: ONE,
            out_min: from_count(0),
            out_max: from_count(1),
        },
    )?;
    let (b, _) = b.knot("out", KnotKind::signal_out("bin"))?;
    let weave = b
        .wire_named("in", "out", "dig", "in")
        .wire_named("dig", "out", "out", "in")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let id = rt.sense_id("in").expect("in");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(id, ZERO)])?;
    assert_eq!(signal_out_value(&rt, "bin"), from_count(0));
    tick_senses(&mut host, &mut rt, &weave, &[(id, ONE)])?;
    assert_eq!(signal_out_value(&rt, "bin"), from_count(1));
    Ok(())
}

/// C08: OnStart latches Flag once (second tick OnStart is falsey).
///
/// # Examples
///
/// ```
/// use wyrd_core::{FlagPriority, KnotKind};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::ScriptedHost;
/// use wyrd_runtime::cookbook::helpers::{bind_default, signal_out_truthy, tick_senses};
///
/// // OnStart ──► Flag.set ──► booted (latched)
/// let (b, _) = Weave::builder("c08")
///     .knot("start", KnotKind::OnStart)
///     .unwrap();
/// let (b, _) = b
///     .knot("flag", KnotKind::flag(FlagPriority::SetWins, false))
///     .unwrap();
/// let (b, _) = b.knot("out", KnotKind::signal_out("booted")).unwrap();
/// let weave = b
///     .wire_named("start", "out", "flag", "set")
///     .wire_named("flag", "out", "out", "in")
///     .build()
///     .unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let mut host = ScriptedHost::new();
/// tick_senses(&mut host, &mut rt, &weave, &[]).unwrap();
/// assert!(signal_out_truthy(&rt, "booted"));
/// ```
pub fn run_c08_on_start_once() -> Result<()> {
    let (b, _) = Weave::builder("c08").knot("start", KnotKind::OnStart)?;
    let (b, _) = b.knot("flag", KnotKind::flag(FlagPriority::SetWins, false))?;
    let (b, _) = b.knot("out", KnotKind::signal_out("booted"))?;
    let weave = b
        .wire_named("start", "out", "flag", "set")
        .wire_named("flag", "out", "out", "in")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[])?;
    assert!(signal_out_truthy(&rt, "booted"));
    tick_senses(&mut host, &mut rt, &weave, &[])?;
    assert!(signal_out_truthy(&rt, "booted"));
    Ok(())
}

/// C09: Emit once — level → Rising → Emit.trigger (never level→Emit forever).
///
/// # Examples
///
/// ```
/// use wyrd_core::{KnotKind, ONE};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::ScriptedHost;
/// use wyrd_runtime::cookbook::helpers::{bind_default, emit_count, tick_senses};
///
/// // ok ──► Rising ──trigger──► EmitCommand("sfx.ping")
/// let (b, _) = Weave::builder("c09")
///     .knot("ok", KnotKind::signal_in())
///     .unwrap();
/// let (b, _) = b.knot("edge", KnotKind::rising_from_zero()).unwrap();
/// let (b, _) = b.knot("em", KnotKind::emit_command("sfx.ping")).unwrap();
/// let weave = b
///     .wire_named("ok", "out", "edge", "in")
///     .wire_named("edge", "out", "em", "trigger")
///     .build()
///     .unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let ok = rt.sense_id("ok").unwrap();
/// let mut host = ScriptedHost::new();
/// tick_senses(&mut host, &mut rt, &weave, &[(ok, ONE)]).unwrap();
/// assert_eq!(emit_count(&rt, "sfx.ping"), 1);
/// tick_senses(&mut host, &mut rt, &weave, &[(ok, ONE)]).unwrap();
/// assert_eq!(emit_count(&rt, "sfx.ping"), 0); // held — no spam
/// ```
pub fn run_c09_emit_once() -> Result<()> {
    let (b, _) = Weave::builder("c09").knot("ok", KnotKind::signal_in())?;
    let (b, _) = b.knot("edge", KnotKind::rising_from_zero())?;
    let (b, _) = b.knot("em", KnotKind::emit_command("sfx.ping"))?;
    let weave = b
        .wire_named("ok", "out", "edge", "in")
        .wire_named("edge", "out", "em", "trigger")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let ok = rt.sense_id("ok").expect("ok");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(ok, ONE)])?;
    assert_eq!(emit_count(&rt, "sfx.ping"), 1);
    tick_senses(&mut host, &mut rt, &weave, &[(ok, ONE)])?;
    assert_eq!(emit_count(&rt, "sfx.ping"), 0);
    Ok(())
}

/// C10: Or any-of keys.
///
/// # Examples
///
/// ```
/// use wyrd_core::{KnotKind, PortSlot, ONE, ZERO};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::ScriptedHost;
/// use wyrd_runtime::cookbook::helpers::{bind_default, signal_out_truthy, tick_senses};
///
/// // key_a ──┐
/// //         ├──► Or ──► open
/// // key_b ──┘
/// let (b, ka) = Weave::builder("c10")
///     .knot("key_a", KnotKind::signal_in())
///     .unwrap();
/// let (b, kb) = b.knot("key_b", KnotKind::signal_in()).unwrap();
/// let (b, or_id) = b.knot("any", KnotKind::or2()).unwrap();
/// let (b, _) = b.knot("out", KnotKind::signal_out("open")).unwrap();
/// let weave = b
///     .wire((ka, PortSlot(0)), (or_id, PortSlot(0)))
///     .unwrap()
///     .wire((kb, PortSlot(0)), (or_id, PortSlot(1)))
///     .unwrap()
///     .wire_named("any", "out", "out", "in")
///     .build()
///     .unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let a = rt.sense_id("key_a").unwrap();
/// let b_id = rt.sense_id("key_b").unwrap();
/// let mut host = ScriptedHost::new();
/// tick_senses(&mut host, &mut rt, &weave, &[(a, ONE), (b_id, ZERO)]).unwrap();
/// assert!(signal_out_truthy(&rt, "open"));
/// ```
pub fn run_c10_or_any_of_keys() -> Result<()> {
    let (b, ka) = Weave::builder("c10").knot("key_a", KnotKind::signal_in())?;
    let (b, kb) = b.knot("key_b", KnotKind::signal_in())?;
    let (b, or_id) = b.knot("any", KnotKind::or2())?;
    let (b, _) = b.knot("out", KnotKind::signal_out("open"))?;
    let weave = b
        .wire((ka, PortSlot(0)), (or_id, PortSlot(0)))?
        .wire((kb, PortSlot(0)), (or_id, PortSlot(1)))?
        .wire_named("any", "out", "out", "in")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let a = rt.sense_id("key_a").expect("a");
    let b_id = rt.sense_id("key_b").expect("b");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(a, ZERO), (b_id, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "open"));
    tick_senses(&mut host, &mut rt, &weave, &[(a, ONE), (b_id, ZERO)])?;
    assert!(signal_out_truthy(&rt, "open"));
    tick_senses(&mut host, &mut rt, &weave, &[(a, ZERO), (b_id, ONE)])?;
    assert!(signal_out_truthy(&rt, "open"));
    Ok(())
}
