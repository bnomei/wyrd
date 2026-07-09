//! Integration: hello invert + and_door host loop (v2 dense ids).

use wyrd_core::{HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

#[test]
fn hello_not() {
    let (b, _) = Weave::builder("hello")
        .knot("c", KnotKind::constant(ONE))
        .unwrap();
    let (b, _) = b.knot("n", KnotKind::not()).unwrap();
    let (b, _) = b.knot("o", KnotKind::signal_out("debug.inverted")).unwrap();
    let weave = b
        .wire_named("c", "out", "n", "in")
        .wire_named("n", "out", "o", "in")
        .build()
        .unwrap();

    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom(&weave).unwrap();
    let box_ = rt.outbox();
    assert_eq!(box_.signals().len(), 1);
    assert!(!wyrd_core::is_truthy(box_.signals()[0].value));
    assert_eq!(rt.path_name(box_.signals()[0].path), "debug.inverted");
}

#[test]
fn and_door_dense_sense() {
    let (b, pa) = Weave::builder("door")
        .knot("plate_a", KnotKind::signal_in())
        .unwrap();
    let (b, pb) = b.knot("plate_b", KnotKind::signal_in()).unwrap();
    let (b, _both) = b.and2("both", pa, pb).unwrap();
    let (b, _) = b.knot("door", KnotKind::signal_out("door.open")).unwrap();
    let weave = b.wire_named("both", "out", "door", "in").build().unwrap();

    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let id_a = rt.sense_id("plate_a").unwrap();
    let id_b = rt.sense_id("plate_b").unwrap();
    let path = rt.path_id("door.open").unwrap();

    // only A
    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(id_a, ONE);
        w.set_sense(id_b, ZERO);
    }
    rt.loom(&weave).unwrap();
    let v = rt
        .outbox()
        .signals()
        .iter()
        .find(|s| s.path == path)
        .map(|s| s.value)
        .unwrap_or(ZERO);
    assert!(!wyrd_core::is_truthy(v));

    // both
    rt.begin_frame(HostTime { tick: 2 });
    {
        let mut w = rt.port_writer();
        w.set_sense(id_a, ONE);
        w.set_sense(id_b, ONE);
    }
    rt.loom(&weave).unwrap();
    let v = rt
        .outbox()
        .signals()
        .iter()
        .find(|s| s.path == path)
        .map(|s| s.value)
        .unwrap_or(ZERO);
    assert!(wyrd_core::is_truthy(v));
}
