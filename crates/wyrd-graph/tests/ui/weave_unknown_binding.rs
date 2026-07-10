use wyrd_graph::{weave, KnotKind};

fn main() {
    let _ = weave! {
        id: "unknown";
        knots {
            sink = KnotKind::signal_out("out");
        }
        threads {
            missing.out -> sink.in;
        }
    };
}
