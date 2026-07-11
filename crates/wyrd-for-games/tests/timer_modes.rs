//! Timer PulseHold + FedCountdown (step 1.1).

use wyrd::SignalDomain;
use wyrd::{HostTime, KnotKind, TimerMode, ONE, ZERO};
use wyrd::Weave;
use wyrd::{cookbook::helpers::signal_out_truthy, BindOpts, Runtime};

fn loom_tick(rt: &mut Runtime, tick: u64, sense: &str, val: wyrd::Signal) {
    rt.begin_frame(HostTime { tick });
    let id = rt.sense_id(sense).unwrap();
    {
        let mut w = rt.port_writer();
        w.set_sense(id, val).unwrap();
    }
    rt.loom();
}

#[test]
fn pulse_hold_active_for_n_ticks_then_off() {
    let mut b = Weave::builder("ph").unwrap();
    let k_btn = b
        .knot("btn", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_t = b
        .knot("t", KnotKind::timer(TimerMode::PulseHold, 3))
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("active", SignalDomain::Bool))
        .unwrap();
    let from = b.output(&k_btn, "out").unwrap();
    let to = b.input(&k_t, "start").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_t, "active").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();

    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();

    loom_tick(&mut rt, 0, "btn", ONE);
    assert!(signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, 1, "btn", ONE);
    assert!(signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, 2, "btn", ONE);
    assert!(signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, 3, "btn", ONE);
    assert!(!signal_out_truthy(&rt, "active"));

    loom_tick(&mut rt, 4, "btn", ZERO);
    assert!(!signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, 5, "btn", ONE);
    assert!(signal_out_truthy(&rt, "active"));
}

#[test]
fn fed_countdown_active_after_n_feed_ticks() {
    let mut b = Weave::builder("fc").unwrap();
    let k_plate = b
        .knot("plate", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_t = b
        .knot("t", KnotKind::timer(TimerMode::FedCountdown, 3))
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("active", SignalDomain::Bool))
        .unwrap();
    let from = b.output(&k_plate, "out").unwrap();
    let to = b.input(&k_t, "feed").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_t, "active").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();

    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();

    loom_tick(&mut rt, 0, "plate", ONE);
    assert!(!signal_out_truthy(&rt, "active")); // rem 3→2
    loom_tick(&mut rt, 1, "plate", ONE);
    assert!(!signal_out_truthy(&rt, "active")); // 2→1
    loom_tick(&mut rt, 2, "plate", ONE);
    assert!(signal_out_truthy(&rt, "active")); // 1→0, done
    loom_tick(&mut rt, 3, "plate", ONE);
    assert!(signal_out_truthy(&rt, "active")); // stay done while fed

    loom_tick(&mut rt, 4, "plate", ZERO);
    assert!(!signal_out_truthy(&rt, "active"));
}

#[test]
fn pulse_hold_ticks_one_and_survives_release() {
    let mut b = Weave::builder("ph1").unwrap();
    let k_btn = b
        .knot("btn", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_t = b
        .knot("t", KnotKind::timer(TimerMode::PulseHold, 2))
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("active", SignalDomain::Bool))
        .unwrap();
    let from = b.output(&k_btn, "out").unwrap();
    let to = b.input(&k_t, "start").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_t, "active").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();

    loom_tick(&mut rt, 0, "btn", ONE);
    assert!(signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, 1, "btn", ZERO);
    assert!(signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, 2, "btn", ZERO);
    assert!(!signal_out_truthy(&rt, "active"));
}

#[test]
fn fed_countdown_drop_mid_count_resets() {
    let mut b = Weave::builder("fc_drop").unwrap();
    let k_plate = b
        .knot("plate", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_t = b
        .knot("t", KnotKind::timer(TimerMode::FedCountdown, 4))
        .unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("active", SignalDomain::Bool))
        .unwrap();
    let from = b.output(&k_plate, "out").unwrap();
    let to = b.input(&k_t, "feed").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_t, "active").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();

    loom_tick(&mut rt, 0, "plate", ONE);
    loom_tick(&mut rt, 1, "plate", ONE);
    assert!(!signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, 2, "plate", ZERO);
    assert!(!signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, 3, "plate", ONE);
    assert!(!signal_out_truthy(&rt, "active"));
    loom_tick(&mut rt, 4, "plate", ONE);
    loom_tick(&mut rt, 5, "plate", ONE);
    loom_tick(&mut rt, 6, "plate", ONE);
    assert!(signal_out_truthy(&rt, "active"));
}
