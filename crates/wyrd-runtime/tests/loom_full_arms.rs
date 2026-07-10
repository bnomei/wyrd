//! Drive every remaining loom eval arm via real builder → bind → loom → outbox.

use wyrd_core::{
    from_count, is_truthy, CalcOp, CompareOp, FlagPriority, HostTime, KnotKind, ONE, ZERO,
};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

fn out_v(rt: &Runtime, path: &str) -> wyrd_core::Signal {
    let pid = rt.path_id(path).unwrap();
    rt.outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| s.value)
        .unwrap_or(ZERO)
}

fn truthy(rt: &Runtime, path: &str) -> bool {
    is_truthy(out_v(rt, path))
}

#[test]
fn or_and_onstart() {
    let (b, _) = Weave::builder("o")
        .knot("a", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("b", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("or", KnotKind::or2()).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let (b, _) = b.knot("start", KnotKind::OnStart).unwrap();
    let (b, _) = b.knot("sout", KnotKind::signal_out("s")).unwrap();
    let weave = b
        .wire_named("a", "out", "or", "in_0")
        .wire_named("b", "out", "or", "in_1")
        .wire_named("or", "out", "out", "in")
        .wire_named("start", "out", "sout", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let a = rt.sense_id("a").unwrap();
    let b_id = rt.sense_id("b").unwrap();

    // OnStart first frame
    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(a, ZERO);
        w.set_sense(b_id, ZERO);
    }
    rt.loom(&weave).unwrap();
    assert!(truthy(&rt, "s"));
    assert!(!truthy(&rt, "y"));

    // Or true when one high; OnStart false after first
    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(a, ONE);
        w.set_sense(b_id, ZERO);
    }
    rt.loom(&weave).unwrap();
    assert!(!truthy(&rt, "s"));
    assert!(truthy(&rt, "y"));
}

#[test]
fn map_abs_neg_calc_all() {
    let (b, _) = Weave::builder("m")
        .knot("neg_in", KnotKind::constant(from_count(-4)))
        .unwrap();
    let (b, _) = b.knot("abs", KnotKind::Abs).unwrap();
    let (b, _) = b.knot("neg", KnotKind::Neg).unwrap();
    let (b, _) = b
        .knot(
            "map",
            KnotKind::Map {
                in_min: from_count(0),
                in_max: from_count(4),
                out_min: from_count(0),
                out_max: from_count(40),
            },
        )
        .unwrap();
    let (b, _) = b.knot("c2", KnotKind::constant(from_count(2))).unwrap();
    let (b, _) = b.knot("c3", KnotKind::constant(from_count(3))).unwrap();
    let (b, _) = b.knot("add", KnotKind::Calc { op: CalcOp::Add }).unwrap();
    let (b, _) = b.knot("sub", KnotKind::Calc { op: CalcOp::Sub }).unwrap();
    let (b, _) = b.knot("mul", KnotKind::Calc { op: CalcOp::Mul }).unwrap();
    let (b, _) = b.knot("out_abs", KnotKind::signal_out("abs")).unwrap();
    let (b, _) = b.knot("out_neg", KnotKind::signal_out("neg")).unwrap();
    let (b, _) = b.knot("out_map", KnotKind::signal_out("map")).unwrap();
    let (b, _) = b.knot("out_add", KnotKind::signal_out("add")).unwrap();
    let (b, _) = b.knot("out_sub", KnotKind::signal_out("sub")).unwrap();
    let (b, _) = b.knot("out_mul", KnotKind::signal_out("mul")).unwrap();
    let weave = b
        .wire_named("neg_in", "out", "abs", "in")
        .wire_named("neg_in", "out", "neg", "in")
        .wire_named("abs", "out", "map", "in")
        .wire_named("c2", "out", "add", "a")
        .wire_named("c3", "out", "add", "b")
        .wire_named("c3", "out", "sub", "a")
        .wire_named("c2", "out", "sub", "b")
        .wire_named("c2", "out", "mul", "a")
        .wire_named("c3", "out", "mul", "b")
        .wire_named("abs", "out", "out_abs", "in")
        .wire_named("neg", "out", "out_neg", "in")
        .wire_named("map", "out", "out_map", "in")
        .wire_named("add", "out", "out_add", "in")
        .wire_named("sub", "out", "out_sub", "in")
        .wire_named("mul", "out", "out_mul", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "abs"), from_count(4));
    assert_eq!(out_v(&rt, "neg"), from_count(4)); // -(-4) under f32; under i32 saturating_neg of -4 = 4
    assert_eq!(out_v(&rt, "map"), from_count(40)); // 4 maps to 40
    assert_eq!(out_v(&rt, "add"), from_count(5));
    assert_eq!(out_v(&rt, "sub"), from_count(1));
    #[cfg(feature = "signal-f32")]
    assert_eq!(out_v(&rt, "mul"), from_count(6));
    #[cfg(feature = "signal-i32")]
    {
        // Q-mul of whole 2*3 = 0
        assert_eq!(out_v(&rt, "mul"), from_count(0));
    }
}

#[test]
fn flag_setwins_and_counter_dec() {
    let (b, _) = Weave::builder("f")
        .knot("set", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("rst", KnotKind::signal_in()).unwrap();
    let (b, _) = b
        .knot("flag", KnotKind::flag(FlagPriority::SetWins, false))
        .unwrap();
    let (b, _) = b.knot("fout", KnotKind::signal_out("flag")).unwrap();
    let (b, _) = b.knot("inc", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("dec", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("cnt", KnotKind::counter()).unwrap();
    let (b, _) = b.knot("cout", KnotKind::signal_out("count")).unwrap();
    let weave = b
        .wire_named("set", "out", "flag", "set")
        .wire_named("rst", "out", "flag", "reset")
        .wire_named("flag", "out", "fout", "in")
        .wire_named("inc", "out", "cnt", "inc")
        .wire_named("dec", "out", "cnt", "dec")
        .wire_named("cnt", "count", "cout", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let set = rt.sense_id("set").unwrap();
    let rst = rt.sense_id("rst").unwrap();
    let inc = rt.sense_id("inc").unwrap();
    let dec = rt.sense_id("dec").unwrap();

    // SetWins: set wins over reset same tick
    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ONE);
        w.set_sense(rst, ONE);
        w.set_sense(inc, ZERO);
        w.set_sense(dec, ZERO);
    }
    rt.loom(&weave).unwrap();
    assert!(truthy(&rt, "flag"));

    // counter: inc then dec
    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ZERO);
        w.set_sense(rst, ZERO);
        w.set_sense(inc, ONE);
        w.set_sense(dec, ZERO);
    }
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "count"), from_count(1));

    rt.begin_frame(HostTime { tick: 2 });
    {
        let mut w = rt.port_writer();
        w.set_sense(inc, ZERO);
        w.set_sense(dec, ZERO);
    }
    rt.loom(&weave).unwrap();
    rt.begin_frame(HostTime { tick: 3 });
    {
        let mut w = rt.port_writer();
        w.set_sense(inc, ZERO);
        w.set_sense(dec, ONE);
    }
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "count"), from_count(0));
}

#[test]
fn compare_all_ops_wired_rhs() {
    for (op, lhs, rhs, expect) in [
        (CompareOp::Eq, 2, 2, true),
        (CompareOp::Ne, 2, 3, true),
        (CompareOp::Lt, 1, 2, true),
        (CompareOp::Lte, 2, 2, true),
        (CompareOp::Gt, 3, 1, true),
        (CompareOp::Gte, 2, 2, true),
        (CompareOp::Eq, 1, 2, false),
    ] {
        let (b, _) = Weave::builder("c")
            .knot("l", KnotKind::constant(from_count(lhs)))
            .unwrap();
        let (b, _) = b.knot("r", KnotKind::constant(from_count(rhs))).unwrap();
        let (b, _) = b.knot("cmp", KnotKind::compare(op, None)).unwrap();
        let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
        let weave = b
            .wire_named("l", "out", "cmp", "lhs")
            .wire_named("r", "out", "cmp", "rhs")
            .wire_named("cmp", "out", "out", "in")
            .build()
            .unwrap();
        let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
        rt.begin_frame(HostTime { tick: 0 });
        rt.loom(&weave).unwrap();
        assert_eq!(truthy(&rt, "y"), expect, "{op:?}");
    }
}

#[test]
fn flag_setwins_reset_and_toggle_and_resetwins_set() {
    // SetWins: pure reset (no set) clears; pure toggle flips.
    let (b, _) = Weave::builder("sw")
        .knot("set", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("rst", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("tog", KnotKind::signal_in()).unwrap();
    let (b, _) = b
        .knot("flag", KnotKind::flag(FlagPriority::SetWins, true))
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("set", "out", "flag", "set")
        .wire_named("rst", "out", "flag", "reset")
        .wire_named("tog", "out", "flag", "toggle")
        .wire_named("flag", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let set = rt.sense_id("set").unwrap();
    let rst = rt.sense_id("rst").unwrap();
    let tog = rt.sense_id("tog").unwrap();

    // set only → on
    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ONE);
        w.set_sense(rst, ZERO);
        w.set_sense(tog, ZERO);
    }
    rt.loom(&weave).unwrap();
    assert!(truthy(&rt, "y"));

    // SetWins pure reset → off
    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ZERO);
        w.set_sense(rst, ONE);
        w.set_sense(tog, ZERO);
    }
    rt.loom(&weave).unwrap();
    assert!(!truthy(&rt, "y"));

    // SetWins pure toggle rising → on
    rt.begin_frame(HostTime { tick: 2 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ZERO);
        w.set_sense(rst, ZERO);
        w.set_sense(tog, ZERO);
    }
    rt.loom(&weave).unwrap();
    rt.begin_frame(HostTime { tick: 3 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ZERO);
        w.set_sense(rst, ZERO);
        w.set_sense(tog, ONE);
    }
    rt.loom(&weave).unwrap();
    assert!(truthy(&rt, "y"));

    // ResetWins: set without reset turns on (covers set-only arm).
    let (b, _) = Weave::builder("rw")
        .knot("set", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("rst", KnotKind::signal_in()).unwrap();
    let (b, _) = b
        .knot("flag", KnotKind::flag(FlagPriority::ResetWins, false))
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("set", "out", "flag", "set")
        .wire_named("rst", "out", "flag", "reset")
        .wire_named("flag", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let set = rt.sense_id("set").unwrap();
    let rst = rt.sense_id("rst").unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ONE);
        w.set_sense(rst, ZERO);
    }
    rt.loom(&weave).unwrap();
    assert!(truthy(&rt, "y"));
}

#[test]
fn map_zero_span_and_delay_one() {
    let (b, _) = Weave::builder("z")
        .knot(
            "c",
            KnotKind::constant(from_count(1)),
        )
        .unwrap();
    let (b, _) = b
        .knot(
            "map",
            KnotKind::Map {
                in_min: from_count(0),
                in_max: from_count(0), // zero span
                out_min: from_count(7),
                out_max: from_count(9),
            },
        )
        .unwrap();
    let (b, _) = b.knot("d", KnotKind::Delay { ticks: 1 }).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let (b, _) = b.knot("dout", KnotKind::signal_out("d")).unwrap();
    let weave = b
        .wire_named("c", "out", "map", "in")
        .wire_named("map", "out", "out", "in")
        .wire_named("c", "out", "d", "in")
        .wire_named("d", "out", "dout", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "y"), from_count(7));
    assert_eq!(out_v(&rt, "d"), ZERO); // delay 1
    rt.begin_frame(HostTime { tick: 1 });
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "d"), from_count(1));
}

#[test]
fn digitize_four_steps_mid_bin() {
    let (b, _) = Weave::builder("dig")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b
        .knot(
            "d",
            KnotKind::Digitize {
                steps: 4,
                in_min: from_count(0),
                in_max: from_count(4),
                out_min: from_count(0),
                out_max: from_count(30),
            },
        )
        .unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("in", "out", "d", "in")
        .wire_named("d", "out", "out", "in")
        .build()
        .unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let id = rt.sense_id("in").unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, from_count(2));
    rt.loom(&weave).unwrap();
    assert_eq!(out_v(&rt, "y"), from_count(20));
}
