//! Delay N-tick ring (step 1.2).

use wyrd::SignalDomain;
use wyrd::Weave;
use wyrd::{cookbook::helpers::signal_out_value, BindOpts, Runtime};
use wyrd::{HostTime, KnotKind, ONE, ZERO};

fn tick(rt: &mut Runtime, t: u64, v: wyrd::Signal) {
    rt.begin_frame(HostTime { tick: t });
    let id = rt.sense_id("in").unwrap();
    rt.port_writer().set_sense(id, v).unwrap();
    rt.loom();
}

#[test]
fn delay_zero_is_passthrough() {
    let mut b = Weave::builder("d0").unwrap();
    let k_in = b
        .knot("in", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_d = b.knot("d", KnotKind::Delay { ticks: 0 }).unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Bool))
        .unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_d, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_d, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    tick(&mut rt, 0, ONE);
    assert!(wyrd::is_truthy(signal_out_value(&rt, "y")));
}

#[test]
fn delay_three_ticks() {
    let mut b = Weave::builder("d3").unwrap();
    let k_in = b
        .knot("in", KnotKind::signal_in(SignalDomain::Bool))
        .unwrap();
    let k_d = b.knot("d", KnotKind::Delay { ticks: 3 }).unwrap();
    let k_out = b
        .knot("out", KnotKind::signal_out("y", SignalDomain::Bool))
        .unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_d, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_d, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();

    tick(&mut rt, 0, ONE);
    assert!(!wyrd::is_truthy(signal_out_value(&rt, "y")));
    tick(&mut rt, 1, ZERO);
    assert!(!wyrd::is_truthy(signal_out_value(&rt, "y")));
    tick(&mut rt, 2, ZERO);
    assert!(!wyrd::is_truthy(signal_out_value(&rt, "y")));
    tick(&mut rt, 3, ZERO);
    assert!(wyrd::is_truthy(signal_out_value(&rt, "y")));
    tick(&mut rt, 4, ZERO);
    assert!(!wyrd::is_truthy(signal_out_value(&rt, "y")));
}
