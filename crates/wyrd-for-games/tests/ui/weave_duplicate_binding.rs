use wyrd::{weave, KnotKind, SignalDomain};

fn main() {
    let _ = weave! {
        id: "duplicate";
        knots {
            same = KnotKind::signal_in(SignalDomain::Bool);
            same = KnotKind::signal_out("out", SignalDomain::Bool);
        }
        threads {
            same.out -> same.in;
        }
    };
}
