use wyrd_core::SignalDomain;
use wyrd_core::{KnotKind, ONE};
use wyrd_graph::Weave;
use wyrd_runtime::{BindError, BindOpts, Budget, Runtime};

#[test]
fn aggregate_delay_buffer_overflow_is_rejected_before_offset_truncation() {
    let mut builder = Weave::builder("large-delays").unwrap();
    for i in 0..3 {
        let source = builder
            .knot(
                format!("source-{i}"),
                KnotKind::constant(ONE, SignalDomain::Bool),
            )
            .unwrap();
        let delay = builder
            .knot(format!("delay-{i}"), KnotKind::Delay { ticks: 40_000 })
            .unwrap();
        let from = builder.output(&source, "out").unwrap();
        let to = builder.input(&delay, "in").unwrap();
        builder.connect(from, to).unwrap();
    }
    let weave = builder.build().unwrap();
    let opts = BindOpts {
        budget: Budget {
            max_delay_path_sum: 40_000,
            ..Budget::default()
        },
        ..BindOpts::default()
    };

    assert!(matches!(
        Runtime::bind(weave, opts),
        Err(BindError::CapacityExceeded {
            resource: "delay buffer",
            count: 80_000,
            ..
        })
    ));
}
