//! Tier B — first five Weaves (GBG middle core).

use super::helpers::{bind_default, signal_out_truthy, tick_senses};
use crate::host::ScriptedHost;
use std::vec;
use wyrd_core::{CompareOp, FlagPriority, KnotKind, Result, TimerMode, ONE, ZERO};
use wyrd_graph::{Pattern, PortRefAuthor, Weave};

/// B01: Monostable Pattern — RisingFromZero → PulseHold (expand-at-load).
pub fn run_b01_monostable_pattern() -> Result<()> {
    let (b, _) = Weave::builder("pat.mono").knot("edge", KnotKind::rising_from_zero())?;
    let (b, _) = b.knot("t", KnotKind::timer(TimerMode::PulseHold, 2))?;
    let inner = b
        .wire_named("edge", "out", "t", "start")
        .build()?;
    let pat = Pattern {
        id: "pat.mono".into(),
        inner,
        exports_in: vec![("start".into(), "edge".into(), "in".into())],
        exports_out: vec![("active".into(), "t".into(), "active".into())],
    };

    let (b, _) = Weave::builder("lvl").knot("btn", KnotKind::signal_in())?;
    let (b, exp) = b.include("hold1", &pat)?;
    let start = exp.port_in("start").expect("start").clone();
    let active = exp.port_out("active").expect("active").clone();
    let (b, _) = b.knot("out", KnotKind::signal_out("lamp"))?;
    let weave = b
        .wire_ports(PortRefAuthor::new("btn", "out"), start)
        .wire_ports(active, PortRefAuthor::new("out", "in"))
        .build()?;

    let mut rt = bind_default(&weave)?;
    let btn = rt.sense_id("btn").expect("btn");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(btn, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "lamp"));
    tick_senses(&mut host, &mut rt, &weave, &[(btn, ONE)])?; // rising
    assert!(signal_out_truthy(&rt, "lamp"));
    tick_senses(&mut host, &mut rt, &weave, &[(btn, ZERO)])?;
    assert!(signal_out_truthy(&rt, "lamp")); // still holding (2 ticks)
    tick_senses(&mut host, &mut rt, &weave, &[(btn, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "lamp"));
    Ok(())
}

/// B02: Two-plate door (And) over ScriptedHost frames.
///
/// ```
/// # use wyrd_runtime::cookbook::tier_b;
/// tier_b::run_b02_two_plate_door().unwrap();
/// ```
pub fn run_b02_two_plate_door() -> Result<()> {
    let (b, pa) = Weave::builder("door").knot("plate_a", KnotKind::signal_in())?;
    let (b, pb) = b.knot("plate_b", KnotKind::signal_in())?;
    let (b, _) = b.and2("both", pa, pb)?;
    let (b, _) = b.knot("door", KnotKind::signal_out("door.open"))?;
    let weave = b.wire_named("both", "out", "door", "in").build()?;

    let mut rt = bind_default(&weave)?;
    let a = rt.sense_id("plate_a").expect("a");
    let b_id = rt.sense_id("plate_b").expect("b");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(a, ONE), (b_id, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "door.open"));
    tick_senses(&mut host, &mut rt, &weave, &[(a, ONE), (b_id, ONE)])?;
    assert!(signal_out_truthy(&rt, "door.open"));
    Ok(())
}

/// B03: Flag toggle on rising toggle port + reset (ResetWins).
pub fn run_b03_flag_toggle() -> Result<()> {
    let (b, _) = Weave::builder("f").knot("tog", KnotKind::signal_in())?;
    let (b, _) = b.knot("rst", KnotKind::signal_in())?;
    let (b, _) = b.knot("flag", KnotKind::flag(FlagPriority::ResetWins, true))?;
    let (b, _) = b.knot("out", KnotKind::signal_out("lamp"))?;
    let weave = b
        .wire_named("tog", "out", "flag", "toggle")
        .wire_named("rst", "out", "flag", "reset")
        .wire_named("flag", "out", "out", "in")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let tog = rt.sense_id("tog").expect("tog");
    let rst = rt.sense_id("rst").expect("rst");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(tog, ONE), (rst, ZERO)])?;
    assert!(signal_out_truthy(&rt, "lamp"));
    tick_senses(&mut host, &mut rt, &weave, &[(tog, ONE), (rst, ZERO)])?;
    assert!(signal_out_truthy(&rt, "lamp")); // held — no second flip
    tick_senses(&mut host, &mut rt, &weave, &[(tog, ZERO), (rst, ONE)])?;
    assert!(!signal_out_truthy(&rt, "lamp"));
    Ok(())
}

/// B04: Counter → Compare(Gte) — Counter owns rising-edge on `inc`.
pub fn run_b04_counter_threshold() -> Result<()> {
    let (b, _) = Weave::builder("c").knot("inc", KnotKind::signal_in())?;
    let (b, _) = b.knot("cnt", KnotKind::counter())?;
    let (b, _) = b.knot("cmp", KnotKind::compare(CompareOp::Gte, Some(2)))?;
    let (b, _) = b.knot("out", KnotKind::signal_out("ready"))?;
    let weave = b
        .wire_named("inc", "out", "cnt", "inc")
        .wire_named("cnt", "count", "cmp", "lhs")
        .wire_named("cmp", "out", "out", "in")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let inc = rt.sense_id("inc").expect("inc");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(inc, ONE)])?;
    assert!(!signal_out_truthy(&rt, "ready")); // count 1
    tick_senses(&mut host, &mut rt, &weave, &[(inc, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "ready"));
    tick_senses(&mut host, &mut rt, &weave, &[(inc, ONE)])?;
    assert!(signal_out_truthy(&rt, "ready")); // count 2
    Ok(())
}

/// B05: Delay Rune (2 ticks) passes level through.
pub fn run_b05_delayed_pulse() -> Result<()> {
    let (b, _) = Weave::builder("d").knot("in", KnotKind::signal_in())?;
    let (b, _) = b.knot("del", KnotKind::Delay { ticks: 2 })?;
    let (b, _) = b.knot("out", KnotKind::signal_out("y"))?;
    let weave = b
        .wire_named("in", "out", "del", "in")
        .wire_named("del", "out", "out", "in")
        .build()?;

    let mut rt = bind_default(&weave)?;
    let id = rt.sense_id("in").expect("in");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(id, ONE)])?;
    assert!(!signal_out_truthy(&rt, "y"));
    tick_senses(&mut host, &mut rt, &weave, &[(id, ONE)])?;
    assert!(!signal_out_truthy(&rt, "y"));
    tick_senses(&mut host, &mut rt, &weave, &[(id, ONE)])?;
    assert!(signal_out_truthy(&rt, "y"));
    Ok(())
}
