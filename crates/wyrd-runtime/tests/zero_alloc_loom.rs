//! Steady-state loom buffer stability (bind-time preallocation).
//!
//! After warmup, repeated loom does not grow outbox capacity or delay_buf length.

use wyrd_core::{HostTime, KnotKind, ONE};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

#[test]
fn loom_steady_state_outbox_capacity_stable() {
    let (b, _) = Weave::builder("z")
        .knot("c", KnotKind::constant(ONE))
        .unwrap();
    let (b, _) = b.knot("n", KnotKind::not()).unwrap();
    let (b, _) = b.knot("o", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("c", "out", "n", "in")
        .wire_named("n", "out", "o", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.loom(&weave).unwrap();
    let cap = rt.outbox_signals_capacity();
    assert!(cap >= 1, "bind reserves at least one SignalOut slot");

    for t in 1..64u64 {
        rt.begin_frame(HostTime { tick: t });
        rt.loom(&weave).unwrap();
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
    let (b, _) = Weave::builder("d")
        .knot("c", KnotKind::constant(ONE))
        .unwrap();
    let (b, _) = b.knot("d", KnotKind::Delay { ticks: 8 }).unwrap();
    let (b, _) = b.knot("o", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("c", "out", "d", "in")
        .wire_named("d", "out", "o", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    assert_eq!(rt.delay_buf_len(), 8);
    let dlen = rt.delay_buf_len();
    for t in 0..16u64 {
        rt.begin_frame(HostTime { tick: t });
        rt.loom(&weave).unwrap();
        assert_eq!(rt.delay_buf_len(), dlen);
    }
    assert_eq!(rt.outbox().signals().len(), 1);
}
