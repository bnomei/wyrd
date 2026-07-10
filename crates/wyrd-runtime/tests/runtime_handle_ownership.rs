use wyrd_core::{HostTime, KnotKind, PortSlot, ONE};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, HandleError, Runtime};

fn bound_runtime(id: &str) -> Runtime {
    let mut builder = Weave::builder(id).unwrap();
    let sense = builder.knot("sense", KnotKind::signal_in()).unwrap();
    let out = builder.knot("out", KnotKind::signal_out("level")).unwrap();
    let emit = builder
        .knot("emit", KnotKind::emit_command("fire"))
        .unwrap();
    let from = builder.output(&sense, "out").unwrap();
    let to = builder.input(&out, "in").unwrap();
    builder.connect(from, to).unwrap();
    let from = builder.output(&sense, "out").unwrap();
    let to = builder.input(&emit, "trigger").unwrap();
    builder.connect(from, to).unwrap();
    Runtime::bind(builder.build().unwrap(), BindOpts::default()).unwrap()
}

#[test]
fn runtime_resolved_handles_reject_cross_runtime_use() {
    let a = bound_runtime("a");
    let sense = a.sense_id("sense").unwrap();
    let path = a.path_id("level").unwrap();
    let cmd = a.cmd_id("fire").unwrap();
    let knot = a.knot_id("sense").unwrap();

    let mut b = bound_runtime("b");
    assert_eq!(
        b.port_writer().set_sense(sense, ONE),
        Err(HandleError::ForeignRuntime { handle: "sense" })
    );
    assert_eq!(
        b.path_name(path),
        Err(HandleError::ForeignRuntime {
            handle: "host path"
        })
    );
    assert_eq!(
        b.cmd_name(cmd),
        Err(HandleError::ForeignRuntime { handle: "command" })
    );
    assert_eq!(
        b.get_port_checked(knot, PortSlot::new(0)),
        Err(HandleError::ForeignRuntime { handle: "knot" })
    );
    assert_eq!(
        b.set_port_checked(knot, PortSlot::new(0), ONE),
        Err(HandleError::ForeignRuntime { handle: "knot" })
    );

    b.begin_frame(HostTime { tick: 0 });
    assert!(b.outbox().signals().is_empty());
}
