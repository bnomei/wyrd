use wyrd_graph::{weave, KnotKind};

fn main() {
    let _ = weave! {
        id: "invalid-endpoint";
        knots {
            source = KnotKind::signal_in();
            sink = KnotKind::signal_out("out");
        }
        threads {
            source["out"] -> sink.in;
        }
    };
}
