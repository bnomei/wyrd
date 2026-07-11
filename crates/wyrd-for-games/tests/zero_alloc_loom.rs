//! Steady-state loom buffer stability (bind-time preallocation).
//!
//! After warmup, repeated loom does not grow outbox capacity or delay_buf length.

use wyrd::SignalDomain;
use wyrd::{HostTime, KnotKind, ONE};
use wyrd::Weave;
use wyrd::{BindOpts, Runtime};

#[test]
fn loom_steady_state_outbox_capacity_stable() {
    let mut b = Weave::builder("z").unwrap();
    let k_c = b
        .knot("c", KnotKind::constant(ONE, SignalDomain::Bool))
        .unwrap();
    let k_n = b.knot("n", KnotKind::not()).unwrap();
    let k_o = b
        .knot("o", KnotKind::signal_out("y", SignalDomain::Bool))
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
    let cap = rt.outbox_signals_capacity();
    assert!(cap >= 1, "bind reserves at least one SignalOut slot");

    for t in 1..64u64 {
        rt.begin_frame(HostTime { tick: t });
        rt.loom();
        assert_eq!(rt.outbox().signals().len(), 1);
        assert_eq!(
            rt.outbox_signals_capacity(),
            cap,
            "outbox capacity must not grow after bind"
        );
    }
}

#[test]
fn delay_ring_sized_at_bind() {
    let mut b = Weave::builder("d").unwrap();
    let k_c = b
        .knot("c", KnotKind::constant(ONE, SignalDomain::Bool))
        .unwrap();
    let k_d = b.knot("d", KnotKind::Delay { ticks: 8 }).unwrap();
    let k_o = b
        .knot("o", KnotKind::signal_out("y", SignalDomain::Bool))
        .unwrap();
    let from = b.output(&k_c, "out").unwrap();
    let to = b.input(&k_d, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_d, "out").unwrap();
    let to = b.input(&k_o, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    assert_eq!(rt.delay_buf_len(), 8);
    let dlen = rt.delay_buf_len();
    for t in 0..16u64 {
        rt.begin_frame(HostTime { tick: t });
        rt.loom();
        assert_eq!(rt.delay_buf_len(), dlen);
    }
    assert_eq!(rt.outbox().signals().len(), 1);
}
