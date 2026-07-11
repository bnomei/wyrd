//! Delay N-tick ring (step 1.2).

use wyrd_core::{HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{cookbook::helpers::signal_out_value, BindOpts, Runtime};

fn tick(rt: &mut Runtime, t: u64, v: wyrd_core::Signal) {
    rt.begin_frame(HostTime { tick: t });
    let id = rt.sense_id("in").unwrap();
    rt.port_writer().set_sense(id, v).unwrap();
    rt.loom();
}

#[test]
fn delay_zero_is_passthrough() {
    let mut b = Weave::builder("d0").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
    let k_d = b.knot("d", KnotKind::Delay { ticks: 0 }).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_d, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_d, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    tick(&mut rt, 0, ONE);
    assert!(wyrd_core::is_truthy(signal_out_value(&rt, "y")));
}

#[test]
fn delay_three_ticks() {
    let mut b = Weave::builder("d3").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
    let k_d = b.knot("d", KnotKind::Delay { ticks: 3 }).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_d, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_d, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();

    tick(&mut rt, 0, ONE);
    assert!(!wyrd_core::is_truthy(signal_out_value(&rt, "y")));
    tick(&mut rt, 1, ZERO);
    assert!(!wyrd_core::is_truthy(signal_out_value(&rt, "y")));
    tick(&mut rt, 2, ZERO);
    assert!(!wyrd_core::is_truthy(signal_out_value(&rt, "y")));
    tick(&mut rt, 3, ZERO);
    assert!(wyrd_core::is_truthy(signal_out_value(&rt, "y")));
    tick(&mut rt, 4, ZERO);
    assert!(!wyrd_core::is_truthy(signal_out_value(&rt, "y")));
}
