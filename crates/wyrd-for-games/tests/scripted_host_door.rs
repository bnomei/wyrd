//! ScriptedHost: deterministic two-plate door without Bevy.

use wyrd::SignalDomain;
use wyrd::Weave;
use wyrd::{is_truthy, KnotKind, ONE, ZERO};
use wyrd::{tick_once, BindOpts, HostCommand, Runtime, ScriptedHost};

#[test]
fn scripted_and_door_levels() {
    let mut b = Weave::builder("door").unwrap();
    let pa = b
        .knot("plate_a", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let pb = b
        .knot("plate_b", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_both = b.knot("both", KnotKind::and2()).unwrap();
    let from = b.output(&pa, "out").unwrap();
    let to = b.input(&k_both, "in_0").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&pb, "out").unwrap();
    let to = b.input(&k_both, "in_1").unwrap();
    b.connect(from, to).unwrap();
    let k_door = b
        .knot(
            "door",
            KnotKind::signal_out("door.open", SignalDomain::Bool),
        )
        .unwrap();
    let from = b.output(&k_both, "out").unwrap();
    let to = b.input(&k_door, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();

    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let id_a = rt.sense_id("plate_a").unwrap();
    let id_b = rt.sense_id("plate_b").unwrap();
    let path = rt.path_id("door.open").unwrap();

    let mut host = ScriptedHost::new();
    host.push_frame([(id_a, ONE), (id_b, ZERO)]);
    host.push_frame([(id_a, ONE), (id_b, ONE)]);
    host.push_frame([(id_a, ZERO), (id_b, ZERO)]);

    tick_once(&mut host, &mut rt).unwrap();
    assert_eq!(host.last_commands().len(), 1);
    match host.last_commands()[0] {
        HostCommand::SetLevel { path: p, value } => {
            assert_eq!(p, path);
            assert!(!is_truthy(value));
        }
        HostCommand::Emit { .. } => panic!("expected SetLevel"),
        _ => panic!("unexpected host command"),
    }

    tick_once(&mut host, &mut rt).unwrap();
    match host.last_commands()[0] {
        HostCommand::SetLevel { path: p, value } => {
            assert_eq!(p, path);
            assert!(is_truthy(value));
        }
        HostCommand::Emit { .. } => panic!("expected SetLevel"),
        _ => panic!("unexpected host command"),
    }

    tick_once(&mut host, &mut rt).unwrap();
    match host.last_commands()[0] {
        HostCommand::SetLevel { value, .. } => assert!(!is_truthy(value)),
        HostCommand::Emit { .. } => panic!("expected SetLevel"),
        _ => panic!("unexpected host command"),
    }

    assert_eq!(host.tick, 3);
    assert_eq!(host.commands_per_tick, vec![1, 1, 1]);
}

#[test]
fn null_host_advances_tick() {
    use wyrd::{tick_once, NullHost};

    let mut b = Weave::builder("n").unwrap();

    let _k_c = b
        .knot("c", KnotKind::constant(ONE, SignalDomain::Bool))
        .unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let mut host = NullHost::default();
    tick_once(&mut host, &mut rt).unwrap();
    tick_once(&mut host, &mut rt).unwrap();
    assert_eq!(host.tick, 2);
}
