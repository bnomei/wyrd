//! Tier B — first five Weaves (GBG middle core).
//!
//! Full Weave listings appear under each function’s **Examples** in rustdoc.

#![allow(clippy::result_large_err)] // CookbookError intentionally preserves context.

use super::helpers::{bind_default, signal_out_truthy, tick_senses};
use super::Result;
use crate::runtime_impl::host::ScriptedHost;
use crate::foundation::{
    from_count, CompareOp, FlagPriority, KnotKind, SignalDomain, TimerMode, ONE, ZERO,
};
use crate::authoring::{
    KnotDef, Pattern, PatternDef, PatternExportDef, PortRefDef, ThreadDef, Weave, WeaveDef,
};

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
                    kind: KnotKind::rising_from_zero(),
                },
                KnotDef {
                    id: "t".into(),
                    kind: KnotKind::timer(TimerMode::PulseHold, 2),
                },
            ],
            threads: alloc::vec![ThreadDef {
                from: PortRefDef::new("edge", "out"),
                to: PortRefDef::new("t", "start"),
            }],
        },
        inputs: alloc::vec![PatternExportDef::new("start", "edge", "in")],
        outputs: alloc::vec![PatternExportDef::new("active", "t", "active")],
    })?;

    let mut b = Weave::builder("lvl")?;
    let k_btn = b.knot("btn", KnotKind::signal_in(SignalDomain::Bool))?;
    let exp = b.include("hold1", &pat)?;
    let start = exp.input("start")?;
    let active = exp.output("active")?;
    let k_out = b.knot("out", KnotKind::signal_out("lamp", SignalDomain::Bool))?;
    let btn_out = b.output(&k_btn, "out")?;
    b.connect(btn_out, start)?;
    let out_in = b.input(&k_out, "in")?;
    b.connect(active, out_in)?;
    let weave = b.build()?;

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

/// B02: Two-plate door (And) over ScriptedHost frames.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_b::run_b02_two_plate_door().unwrap();
/// ```
pub fn run_b02_two_plate_door() -> Result<()> {
    let mut b = Weave::builder("door")?;
    let pa = b.knot("plate_a", KnotKind::signal_in(SignalDomain::Bool))?;
    let pb = b.knot("plate_b", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_both = b.knot("both", KnotKind::and2())?;
    let from = b.output(&pa, "out")?;
    let to = b.input(&k_both, "in_0")?;
    b.connect(from, to)?;
    let from = b.output(&pb, "out")?;
    let to = b.input(&k_both, "in_1")?;
    b.connect(from, to)?;
    let k_door = b.knot(
        "door",
        KnotKind::signal_out("door.open", SignalDomain::Bool),
    )?;
    let from = b.output(&k_both, "out")?;
    let to = b.input(&k_door, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

    let mut rt = bind_default(&weave)?;
    let a = rt.sense_id("plate_a").expect("a");
    let b_id = rt.sense_id("plate_b").expect("b");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &[(a, ONE), (b_id, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "door.open"));
    tick_senses(&mut host, &mut rt, &[(a, ONE), (b_id, ONE)])?;
    assert!(signal_out_truthy(&rt, "door.open"));
    Ok(())
}

/// B03: Flag toggle on rising `toggle` port + `reset` (ResetWins).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_b::run_b03_flag_toggle().unwrap();
/// ```
pub fn run_b03_flag_toggle() -> Result<()> {
    let mut b = Weave::builder("f")?;
    let k_tog = b.knot("tog", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_rst = b.knot("rst", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_flag = b.knot("flag", KnotKind::flag(FlagPriority::ResetWins, true))?;
    let k_out = b.knot("out", KnotKind::signal_out("lamp", SignalDomain::Bool))?;
    let from = b.output(&k_tog, "out")?;
    let to = b.input(&k_flag, "toggle")?;
    b.connect(from, to)?;
    let from = b.output(&k_rst, "out")?;
    let to = b.input(&k_flag, "reset")?;
    b.connect(from, to)?;
    let from = b.output(&k_flag, "out")?;
    let to = b.input(&k_out, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

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

/// B04: Counter → Compare(Gte) — Counter owns rising-edge on `inc` (no extra Rising).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_b::run_b04_counter_threshold().unwrap();
/// ```
pub fn run_b04_counter_threshold() -> Result<()> {
    let mut b = Weave::builder("c")?;
    let k_inc = b.knot("inc", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_cnt = b.knot("cnt", KnotKind::counter())?;
    let k_cmp = b.knot(
        "cmp",
        KnotKind::compare(CompareOp::Gte, Some(from_count(2)), SignalDomain::Count),
    )?;
    let k_out = b.knot("out", KnotKind::signal_out("ready", SignalDomain::Bool))?;
    let from = b.output(&k_inc, "out")?;
    let to = b.input(&k_cnt, "inc")?;
    b.connect(from, to)?;
    let from = b.output(&k_cnt, "count")?;
    let to = b.input(&k_cmp, "lhs")?;
    b.connect(from, to)?;
    let from = b.output(&k_cmp, "out")?;
    let to = b.input(&k_out, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

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

/// B05: Delay Rune (2 ticks) passes level through.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_b::run_b05_delayed_pulse().unwrap();
/// ```
pub fn run_b05_delayed_pulse() -> Result<()> {
    let mut b = Weave::builder("d")?;
    let k_in = b.knot("in", KnotKind::signal_in(SignalDomain::Level))?;
    let k_del = b.knot("del", KnotKind::Delay { ticks: 2 })?;
    let k_out = b.knot("out", KnotKind::signal_out("y", SignalDomain::Level))?;
    let from = b.output(&k_in, "out")?;
    let to = b.input(&k_del, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_del, "out")?;
    let to = b.input(&k_out, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

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
