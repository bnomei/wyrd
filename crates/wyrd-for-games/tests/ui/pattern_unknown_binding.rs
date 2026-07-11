use wyrd::{pattern, KnotKind};

fn main() {
    let _ = pattern! {
        id: "unknown";
        knots {
            edge = KnotKind::rising_from_zero();
        }
        exports {
            input start = missing.in;
            output active = edge.out;
        }
        threads {}
    };
}
