//! Catalog: Xor.

use wyrd_core::{is_truthy, HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

fn out_v(rt: &Runtime, path: &str) -> wyrd_core::Signal {
    let pid = rt.path_id(path).unwrap();
    rt.outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| s.value)
        .unwrap_or(ZERO)
}

#[test]
fn xor_truth_table() {
    let (b, _) = Weave::builder("x")
        .knot("a", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("b", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("x", KnotKind::xor()).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("a", "out", "x", "a")
        .wire_named("b", "out", "x", "b")
        .wire_named("x", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let a = rt.sense_id("a").unwrap();
    let b_id = rt.sense_id("b").unwrap();

    for (av, bv, expect) in [
        (ZERO, ZERO, false),
        (ONE, ZERO, true),
        (ZERO, ONE, true),
        (ONE, ONE, false),
    ] {
        rt.begin_frame(HostTime { tick: 0 });
        {
            let mut w = rt.port_writer();
            w.set_sense(a, av);
            w.set_sense(b_id, bv);
        }
        rt.loom(&weave).unwrap();
        assert_eq!(is_truthy(out_v(&rt, "y")), expect);
    }
}
