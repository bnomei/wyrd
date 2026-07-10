//! Emit-per-tick hard cap (BindOpts::max_emits_per_tick).

use wyrd_core::{HostTime, KnotKind, ONE};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

#[test]
fn many_emits_same_tick_capped() {
    // 4 independent rising emits same loom; cap at 2.
    let mut b = Weave::builder("cap").unwrap();
    let k_b0 = b.knot("b0", KnotKind::signal_in()).unwrap();
    let k_b1 = b.knot("b1", KnotKind::signal_in()).unwrap();
    let k_b2 = b.knot("b2", KnotKind::signal_in()).unwrap();
    let k_b3 = b.knot("b3", KnotKind::signal_in()).unwrap();
    let k_e0 = b.knot("e0", KnotKind::emit_command("a")).unwrap();
    let k_e1 = b.knot("e1", KnotKind::emit_command("b")).unwrap();
    let k_e2 = b.knot("e2", KnotKind::emit_command("c")).unwrap();
    let k_e3 = b.knot("e3", KnotKind::emit_command("d")).unwrap();
    let from = b.output(&k_b0, "out").unwrap();
    let to = b.input(&k_e0, "trigger").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_b1, "out").unwrap();
    let to = b.input(&k_e1, "trigger").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_b2, "out").unwrap();
    let to = b.input(&k_e2, "trigger").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_b3, "out").unwrap();
    let to = b.input(&k_e3, "trigger").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();

    let mut rt = Runtime::bind(
        weave.clone(),
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
            w.set_sense(*id, ONE).unwrap();
        }
    }
    rt.loom();
    assert_eq!(rt.outbox().emits().len(), 2);
}

#[test]
fn default_cap_allows_typical_emit() {
    let mut b = Weave::builder("e").unwrap();
    let k_btn = b.knot("btn", KnotKind::signal_in()).unwrap();
    let k_em = b.knot("em", KnotKind::emit_command("fire")).unwrap();
    let from = b.output(&k_btn, "out").unwrap();
    let to = b.input(&k_em, "trigger").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let btn = rt.sense_id("btn").unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(btn, ONE).unwrap();
    rt.loom();
    assert_eq!(rt.outbox().emits().len(), 1);
}
