//! Integration: hello invert + and_door host loop (v2 dense ids).

use wyrd::SignalDomain;
use wyrd::{HostTime, KnotKind, ONE, ZERO};
use wyrd::Weave;
use wyrd::{cookbook::helpers::signal_out_value, BindOpts, Runtime};

#[test]
fn hello_not() {
    let mut b = Weave::builder("hello").unwrap();
    let k_c = b
        .knot("c", KnotKind::constant(ONE, SignalDomain::Bool))
        .unwrap();
    let k_n = b.knot("n", KnotKind::not()).unwrap();
    let k_o = b
        .knot(
            "o",
            KnotKind::signal_out("debug.inverted", SignalDomain::Bool),
        )
        .unwrap();
    let from = b.output(&k_c, "out").unwrap();
    let to = b.input(&k_n, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_n, "out").unwrap();
    let to = b.input(&k_o, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();

    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    let box_ = rt.outbox();
    assert_eq!(box_.signals().len(), 1);
    assert!(!wyrd::is_truthy(box_.signals()[0].value));
    assert_eq!(rt.path_name(box_.signals()[0].path), Ok("debug.inverted"));
}

#[test]
fn and_door_dense_sense() {
    let mut b = Weave::builder("door").unwrap();
    let pa = b
        .knot("plate_a", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let pb = b
        .knot("plate_b", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let _both = b.knot("both", KnotKind::and2()).unwrap();
    let from = b.output(&pa, "out").unwrap();
    let to = b.input(&_both, "in_0").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&pb, "out").unwrap();
    let to = b.input(&_both, "in_1").unwrap();
    b.connect(from, to).unwrap();
    let k_door = b
        .knot(
            "door",
            KnotKind::signal_out("door.open", SignalDomain::Bool),
        )
        .unwrap();
    let from = b.output(&_both, "out").unwrap();
    let to = b.input(&k_door, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();

    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let id_a = rt.sense_id("plate_a").unwrap();
    let id_b = rt.sense_id("plate_b").unwrap();

    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(id_a, ONE).unwrap();
        w.set_sense(id_b, ZERO).unwrap();
    }
    rt.loom();
    let v = signal_out_value(&rt, "door.open");
    assert!(!wyrd::is_truthy(v));

    rt.begin_frame(HostTime { tick: 2 });
    {
        let mut w = rt.port_writer();
        w.set_sense(id_a, ONE).unwrap();
        w.set_sense(id_b, ONE).unwrap();
    }
    rt.loom();
    let v = signal_out_value(&rt, "door.open");
    assert!(wyrd::is_truthy(v));
}
