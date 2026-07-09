//! Timer PulseHold + FedCountdown (step 1.1).

use wyrd_core::{HostTime, KnotKind, TimerMode, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

fn signal_out_truthy(rt: &Runtime, path: &str) -> bool {
    let Some(pid) = rt.path_id(path) else {
        return false;
    };
    rt.outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| wyrd_core::is_truthy(s.value))
        .unwrap_or(false)
}

fn loom_tick(rt: &mut Runtime, weave: &Weave, tick: u64, sense: &str, val: wyrd_core::Signal) {
    rt.begin_frame(HostTime { tick });
    let id = rt.sense_id(sense).unwrap();
    {
        let mut w = rt.port_writer();
        w.set_sense(id, val);
    }
    rt.loom(weave).unwrap();
}

#[test]
fn pulse_hold_active_for_n_ticks_then_off() {
    // start rising → active for exactly 3 looms; held start does not re-arm mid-window
    let (b, _) = Weave::builder("ph")
        .knot("btn", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b
        .knot(
            "t",
            KnotKind::timer(TimerMode::PulseHold, 3),
        )
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("active")).unwrap();
    let weave = b
        .wire_named("btn", "out", "t", "start")
        .wire_named("t", "active", "out", "in")
        .build()
        .unwrap();

    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();

    // rising start
    loom_tick(&mut rt, &weave, 0, "btn", ONE);
    assert!(signal_out_truthy(&rt, "active"));
    // held high — still counting, no re-arm
    loom_tick(&mut rt, &weave, 1, "btn", ONE);
    assert!(signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, &weave, 2, "btn", ONE);
    assert!(signal_out_truthy(&rt, "active"));
    // 4th tick after start: off
    loom_tick(&mut rt, &weave, 3, "btn", ONE);
    assert!(!signal_out_truthy(&rt, "active"));

    // release then press again re-arms
    loom_tick(&mut rt, &weave, 4, "btn", ZERO);
    assert!(!signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, &weave, 5, "btn", ONE);
    assert!(signal_out_truthy(&rt, "active"));
}

#[test]
fn fed_countdown_active_after_n_feed_ticks() {
    // feed for 3 ticks → active on the tick countdown hits 0; drop feed clears
    let (b, _) = Weave::builder("fc")
        .knot("plate", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b
        .knot(
            "t",
            KnotKind::timer(TimerMode::FedCountdown, 3),
        )
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("active")).unwrap();
    let weave = b
        .wire_named("plate", "out", "t", "feed")
        .wire_named("t", "active", "out", "in")
        .build()
        .unwrap();

    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();

    loom_tick(&mut rt, &weave, 0, "plate", ONE);
    assert!(!signal_out_truthy(&rt, "active")); // rem 3→2
    loom_tick(&mut rt, &weave, 1, "plate", ONE);
    assert!(!signal_out_truthy(&rt, "active")); // 2→1
    loom_tick(&mut rt, &weave, 2, "plate", ONE);
    assert!(signal_out_truthy(&rt, "active")); // 1→0, done
    loom_tick(&mut rt, &weave, 3, "plate", ONE);
    assert!(signal_out_truthy(&rt, "active")); // stay done while fed

    loom_tick(&mut rt, &weave, 4, "plate", ZERO);
    assert!(!signal_out_truthy(&rt, "active"));
}

#[test]
fn pulse_hold_ticks_one_and_survives_release() {
    let (b, _) = Weave::builder("ph1")
        .knot("btn", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b
        .knot("t", KnotKind::timer(TimerMode::PulseHold, 2))
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("active")).unwrap();
    let weave = b
        .wire_named("btn", "out", "t", "start")
        .wire_named("t", "active", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();

    loom_tick(&mut rt, &weave, 0, "btn", ONE);
    assert!(signal_out_truthy(&rt, "active"));
    // release mid-window — monostable continues
    loom_tick(&mut rt, &weave, 1, "btn", ZERO);
    assert!(signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, &weave, 2, "btn", ZERO);
    assert!(!signal_out_truthy(&rt, "active"));
}

#[test]
fn fed_countdown_drop_mid_count_resets() {
    let (b, _) = Weave::builder("fc_drop")
        .knot("plate", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b
        .knot("t", KnotKind::timer(TimerMode::FedCountdown, 4))
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("active")).unwrap();
    let weave = b
        .wire_named("plate", "out", "t", "feed")
        .wire_named("t", "active", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();

    loom_tick(&mut rt, &weave, 0, "plate", ONE);
    loom_tick(&mut rt, &weave, 1, "plate", ONE);
    assert!(!signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, &weave, 2, "plate", ZERO);
    assert!(!signal_out_truthy(&rt, "active"));
    // re-feed must re-arm full countdown
    loom_tick(&mut rt, &weave, 3, "plate", ONE);
    assert!(!signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, &weave, 4, "plate", ONE);
    loom_tick(&mut rt, &weave, 5, "plate", ONE);
    loom_tick(&mut rt, &weave, 6, "plate", ONE);
    assert!(signal_out_truthy(&rt, "active"));
}
