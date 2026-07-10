//! Emit-per-tick hard cap (BindOpts::max_emits_per_tick).

use wyrd_core::{HostTime, KnotKind, ONE};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

#[test]
fn many_emits_same_tick_capped() {
    // 4 independent rising emits same loom; cap at 2.
    let (b, _) = Weave::builder("cap")
        .knot("b0", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("b1", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("b2", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("b3", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("e0", KnotKind::emit_command("a")).unwrap();
    let (b, _) = b.knot("e1", KnotKind::emit_command("b")).unwrap();
    let (b, _) = b.knot("e2", KnotKind::emit_command("c")).unwrap();
    let (b, _) = b.knot("e3", KnotKind::emit_command("d")).unwrap();
    let weave = b
        .wire_named("b0", "out", "e0", "trigger")
        .wire_named("b1", "out", "e1", "trigger")
        .wire_named("b2", "out", "e2", "trigger")
        .wire_named("b3", "out", "e3", "trigger")
        .build()
        .unwrap();

    let mut rt = Runtime::bind(
        &weave,
        BindOpts {
            max_emits_per_tick: 2,
            ..BindOpts::default()
        },
    )
    .unwrap();
    let ids: Vec<_> = (0..4)
        .map(|i| rt.sense_id(&format!("b{i}")).unwrap())
        .collect();

    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        for id in &ids {
            w.set_sense(*id, ONE);
        }
    }
    rt.loom(&weave).unwrap();
    assert_eq!(rt.outbox().emits().len(), 2);
}

#[test]
fn default_cap_allows_typical_emit() {
    let (b, _) = Weave::builder("e")
        .knot("btn", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("em", KnotKind::emit_command("fire")).unwrap();
    let weave = b.wire_named("btn", "out", "em", "trigger").build().unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let btn = rt.sense_id("btn").unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(btn, ONE);
    rt.loom(&weave).unwrap();
    assert_eq!(rt.outbox().emits().len(), 1);
}
