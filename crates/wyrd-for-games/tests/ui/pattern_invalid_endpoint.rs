use wyrd::{pattern, KnotKind};

fn main() {
    let _ = pattern! {
        id: "invalid-endpoint";
        knots {
            edge = KnotKind::rising_from_zero();
        }
        exports {
            input start = edge["in"];
            output active = edge.out;
        }
        threads {}
    };
}
