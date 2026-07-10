//! First five Weaves — pedagogic recipes (CI-backed cookbook).
//!
//! 1. Monostable Pattern (RisingFromZero → PulseHold)
//! 2. Two-plate door (And)
//! 3. Held button / Flag
//! 4. Edge → Counter → Compare
//! 5. Delayed pulse (Delay Rune)

use wyrd_core::{
    from_count, is_truthy, CompareOp, FlagPriority, HostTime, KnotKind, TimerMode, ONE, ZERO,
};
use wyrd_graph::{Pattern, Weave};
use wyrd_runtime::{tick_once, BindOpts, HostCommand, Runtime, ScriptedHost};

fn level(rt: &Runtime, path: &str) -> bool {
    let pid = rt.path_id(path).unwrap();
    rt.outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| is_truthy(s.value))
        .unwrap_or(false)
}

/// 1. Monostable: edge → hold active for N ticks (Pattern include).
#[test]
fn recipe_monostable_pattern() {
    let (b, _) = Weave::builder("pat.mono")
        .knot("edge", KnotKind::rising_from_zero())
        .unwrap();
    let (b, _) = b
        .knot("t", KnotKind::timer(TimerMode::PulseHold, 2))
        .unwrap();
    let inner = b
        .wire_named("edge", "out", "t", "start")
        .build()
        .unwrap();
    let pat = Pattern {
        id: "pat.mono".into(),
        inner,
        exports_in: vec![("start".into(), "edge".into(), "in".into())],
        exports_out: vec![("active".into(), "t".into(), "active".into())],
    };

    let (b, _) = Weave::builder("lvl")
        .knot("btn", KnotKind::signal_in())
        .unwrap();
    let (b, exp) = b.include("hold1", &pat).unwrap();
    let start = exp.port_in("start").unwrap().clone();
    let active = exp.port_out("active").unwrap().clone();
    let (b, _) = b.knot("out", KnotKind::signal_out("lamp")).unwrap();
    use wyrd_graph::PortRefAuthor;
    let weave = b
        .wire_ports(PortRefAuthor::new("btn", "out"), start)
        .wire_ports(active, PortRefAuthor::new("out", "in"))
        .build()
        .unwrap();

    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let btn = rt.sense_id("btn").unwrap();
    let mut host = ScriptedHost::new();
    host.push_frame([(btn, ZERO)]);
    host.push_frame([(btn, ONE)]); // rising
    host.push_frame([(btn, ZERO)]);
    host.push_frame([(btn, ZERO)]);

    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(!level(&rt, "lamp"));
    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(level(&rt, "lamp")); // hold starts
    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(level(&rt, "lamp")); // still holding (2 ticks)
    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(!level(&rt, "lamp")); // expired
}

/// 2. Two-plate door (And).
#[test]
fn recipe_two_plate_door() {
    let (b, pa) = Weave::builder("door")
        .knot("plate_a", KnotKind::signal_in())
        .unwrap();
    let (b, pb) = b.knot("plate_b", KnotKind::signal_in()).unwrap();
    let (b, _) = b.and2("both", pa, pb).unwrap();
    let (b, _) = b.knot("door", KnotKind::signal_out("door.open")).unwrap();
    let weave = b.wire_named("both", "out", "door", "in").build().unwrap();

    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let a = rt.sense_id("plate_a").unwrap();
    let b_id = rt.sense_id("plate_b").unwrap();
    let mut host = ScriptedHost::new();
    host.push_frame([(a, ONE), (b_id, ZERO)]);
    host.push_frame([(a, ONE), (b_id, ONE)]);

    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(!level(&rt, "door.open"));
    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(level(&rt, "door.open"));
}

/// 3. Held button → Flag toggle on rising, reset.
#[test]
fn recipe_flag_toggle() {
    let (b, _) = Weave::builder("f")
        .knot("tog", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("rst", KnotKind::signal_in()).unwrap();
    let (b, _) = b
        .knot("flag", KnotKind::flag(FlagPriority::ResetWins, true))
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("lamp")).unwrap();
    let weave = b
        .wire_named("tog", "out", "flag", "toggle")
        .wire_named("rst", "out", "flag", "reset")
        .wire_named("flag", "out", "out", "in")
        .build()
        .unwrap();

    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let tog = rt.sense_id("tog").unwrap();
    let rst = rt.sense_id("rst").unwrap();
    let mut host = ScriptedHost::new();
    host.push_frame([(tog, ONE), (rst, ZERO)]); // toggle on
    host.push_frame([(tog, ONE), (rst, ZERO)]); // held — no second flip
    host.push_frame([(tog, ZERO), (rst, ONE)]); // reset

    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(level(&rt, "lamp"));
    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(level(&rt, "lamp"));
    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(!level(&rt, "lamp"));
}

/// 4. Rising edge → Counter → Compare threshold.
#[test]
fn recipe_counter_threshold() {
    let (b, _) = Weave::builder("c")
        .knot("inc", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("cnt", KnotKind::counter()).unwrap();
    let (b, _) = b
        .knot("cmp", KnotKind::compare(CompareOp::Gte, Some(2)))
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("ready")).unwrap();
    let weave = b
        .wire_named("inc", "out", "cnt", "inc")
        .wire_named("cnt", "count", "cmp", "lhs")
        .wire_named("cmp", "out", "out", "in")
        .build()
        .unwrap();

    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let inc = rt.sense_id("inc").unwrap();
    let mut host = ScriptedHost::new();
    // rising, hold, fall, rising → two edges
    host.push_frame([(inc, ONE)]);
    host.push_frame([(inc, ZERO)]);
    host.push_frame([(inc, ONE)]);

    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(!level(&rt, "ready")); // count 1
    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(!level(&rt, "ready"));
    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(level(&rt, "ready")); // count 2
}

/// 5. Delayed pulse (Delay Rune, 2 ticks).
#[test]
fn recipe_delayed_pulse() {
    let (b, _) = Weave::builder("d")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("del", KnotKind::Delay { ticks: 2 }).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("in", "out", "del", "in")
        .wire_named("del", "out", "out", "in")
        .build()
        .unwrap();

    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let id = rt.sense_id("in").unwrap();
    let mut host = ScriptedHost::new();
    host.push_frame([(id, ONE)]);
    host.push_frame([(id, ONE)]);
    host.push_frame([(id, ONE)]);

    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(!level(&rt, "y"));
    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(!level(&rt, "y"));
    tick_once(&mut host, &mut rt, &weave).unwrap();
    assert!(level(&rt, "y"));
    let _ = from_count(0);
    let _ = HostTime { tick: 0 };
    let _ = HostCommand::SetLevel {
        path: rt.path_id("y").unwrap(),
        value: ZERO,
    };
}
