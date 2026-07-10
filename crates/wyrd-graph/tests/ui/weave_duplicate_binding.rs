use wyrd_graph::{weave, KnotKind};

fn main() {
    let _ = weave! {
        id: "duplicate";
        knots {
            same = KnotKind::signal_in();
            same = KnotKind::signal_out("out");
        }
        threads {
            same.out -> same.in;
        }
    };
}
