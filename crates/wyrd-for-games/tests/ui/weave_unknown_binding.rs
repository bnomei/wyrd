use wyrd::{weave, KnotKind, SignalDomain};

fn main() {
    let _ = weave! {
        id: "unknown";
        knots {
            sink = KnotKind::signal_out("out", SignalDomain::Bool);
        }
        threads {
            missing.out -> sink.in;
        }
    };
}
