use wyrd_graph::{weave, KnotKind, SignalDomain};

fn main() {
    let _ = weave! {
        id: "invalid-endpoint";
        knots {
            source = KnotKind::signal_in(SignalDomain::Bool);
            sink = KnotKind::signal_out("out", SignalDomain::Bool);
        }
        threads {
            source["out"] -> sink.in;
        }
    };
}
