use wyrd::{pattern, KnotKind};

fn main() {
    let _ = pattern! {
        id: "duplicate";
        knots {
            same = KnotKind::rising_from_zero();
            same = KnotKind::rising_from_zero();
        }
        exports {
            input start = same.in;
            output active = same.out;
        }
        threads {}
    };
}
