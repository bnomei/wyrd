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
    let mut b = Weave::builder("o").unwrap();
    let k_a = b.knot("a", KnotKind::signal_in()).unwrap();
    let k_b = b.knot("b", KnotKind::signal_in()).unwrap();
    let k_or = b.knot("or", KnotKind::or2()).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let k_start = b.knot("start", KnotKind::OnStart).unwrap();
    let k_sout = b.knot("sout", KnotKind::signal_out("s")).unwrap();
    let from = b.output(&k_a, "out").unwrap();
    let to = b.input(&k_or, "in_0").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_b, "out").unwrap();
    let to = b.input(&k_or, "in_1").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_or, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_start, "out").unwrap();
    let to = b.input(&k_sout, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let a = rt.sense_id("a").unwrap();
    let b_id = rt.sense_id("b").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(a, ZERO).unwrap();
        w.set_sense(b_id, ZERO).unwrap();
    }
    rt.loom();
    assert!(truthy(&rt, "s"));
    assert!(!truthy(&rt, "y"));

    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(a, ONE).unwrap();
        w.set_sense(b_id, ZERO).unwrap();
    }
    rt.loom();
    assert!(!truthy(&rt, "s"));
    assert!(truthy(&rt, "y"));
}

#[test]
fn map_abs_neg_calc_all() {
    let mut b = Weave::builder("m").unwrap();
    let k_neg_in = b
        .knot("neg_in", KnotKind::constant(from_count(-4)))
        .unwrap();
    let k_abs = b.knot("abs", KnotKind::Abs).unwrap();
    let k_neg = b.knot("neg", KnotKind::Neg).unwrap();
    let k_map = b
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
    let k_c2 = b.knot("c2", KnotKind::constant(from_count(2))).unwrap();
    let k_c3 = b.knot("c3", KnotKind::constant(from_count(3))).unwrap();
    let k_add = b.knot("add", KnotKind::Calc { op: CalcOp::Add }).unwrap();
    let k_sub = b.knot("sub", KnotKind::Calc { op: CalcOp::Sub }).unwrap();
    let k_mul = b.knot("mul", KnotKind::Calc { op: CalcOp::Mul }).unwrap();
    let k_out_abs = b.knot("out_abs", KnotKind::signal_out("abs")).unwrap();
    let k_out_neg = b.knot("out_neg", KnotKind::signal_out("neg")).unwrap();
    let k_out_map = b.knot("out_map", KnotKind::signal_out("map")).unwrap();
    let k_out_add = b.knot("out_add", KnotKind::signal_out("add")).unwrap();
    let k_out_sub = b.knot("out_sub", KnotKind::signal_out("sub")).unwrap();
    let k_out_mul = b.knot("out_mul", KnotKind::signal_out("mul")).unwrap();
    let from = b.output(&k_neg_in, "out").unwrap();
    let to = b.input(&k_abs, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_neg_in, "out").unwrap();
    let to = b.input(&k_neg, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_abs, "out").unwrap();
    let to = b.input(&k_map, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_c2, "out").unwrap();
    let to = b.input(&k_add, "a").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_c3, "out").unwrap();
    let to = b.input(&k_add, "b").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_c3, "out").unwrap();
    let to = b.input(&k_sub, "a").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_c2, "out").unwrap();
    let to = b.input(&k_sub, "b").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_c2, "out").unwrap();
    let to = b.input(&k_mul, "a").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_c3, "out").unwrap();
    let to = b.input(&k_mul, "b").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_abs, "out").unwrap();
    let to = b.input(&k_out_abs, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_neg, "out").unwrap();
    let to = b.input(&k_out_neg, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_map, "out").unwrap();
    let to = b.input(&k_out_map, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_add, "out").unwrap();
    let to = b.input(&k_out_add, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_sub, "out").unwrap();
    let to = b.input(&k_out_sub, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_mul, "out").unwrap();
    let to = b.input(&k_out_mul, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    assert_eq!(out_v(&rt, "abs"), from_count(4));
    assert_eq!(out_v(&rt, "neg"), from_count(4)); // -(-4) under f32; under i32 saturating_neg of -4 = 4
    assert_eq!(out_v(&rt, "map"), from_count(40)); // 4 maps to 40
    assert_eq!(out_v(&rt, "add"), from_count(5));
    assert_eq!(out_v(&rt, "sub"), from_count(1));
    #[cfg(feature = "signal-f32")]
    assert_eq!(out_v(&rt, "mul"), from_count(6));
    #[cfg(feature = "signal-i32")]
    {
        assert_eq!(out_v(&rt, "mul"), from_count(0));
    }
}

#[test]
fn flag_setwins_and_counter_dec() {
    let mut b = Weave::builder("f").unwrap();
    let k_set = b.knot("set", KnotKind::signal_in()).unwrap();
    let k_rst = b.knot("rst", KnotKind::signal_in()).unwrap();
    let k_flag = b
        .knot("flag", KnotKind::flag(FlagPriority::SetWins, false))
        .unwrap();
    let k_fout = b.knot("fout", KnotKind::signal_out("flag")).unwrap();
    let k_inc = b.knot("inc", KnotKind::signal_in()).unwrap();
    let k_dec = b.knot("dec", KnotKind::signal_in()).unwrap();
    let k_cnt = b.knot("cnt", KnotKind::counter()).unwrap();
    let k_cout = b.knot("cout", KnotKind::signal_out("count")).unwrap();
    let from = b.output(&k_set, "out").unwrap();
    let to = b.input(&k_flag, "set").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_rst, "out").unwrap();
    let to = b.input(&k_flag, "reset").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_flag, "out").unwrap();
    let to = b.input(&k_fout, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_inc, "out").unwrap();
    let to = b.input(&k_cnt, "inc").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_dec, "out").unwrap();
    let to = b.input(&k_cnt, "dec").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_cnt, "count").unwrap();
    let to = b.input(&k_cout, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let set = rt.sense_id("set").unwrap();
    let rst = rt.sense_id("rst").unwrap();
    let inc = rt.sense_id("inc").unwrap();
    let dec = rt.sense_id("dec").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ONE).unwrap();
        w.set_sense(rst, ONE).unwrap();
        w.set_sense(inc, ZERO).unwrap();
        w.set_sense(dec, ZERO).unwrap();
    }
    rt.loom();
    assert!(truthy(&rt, "flag"));

    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ZERO).unwrap();
        w.set_sense(rst, ZERO).unwrap();
        w.set_sense(inc, ONE).unwrap();
        w.set_sense(dec, ZERO).unwrap();
    }
    rt.loom();
    assert_eq!(out_v(&rt, "count"), from_count(1));

    rt.begin_frame(HostTime { tick: 2 });
    {
        let mut w = rt.port_writer();
        w.set_sense(inc, ZERO).unwrap();
        w.set_sense(dec, ZERO).unwrap();
    }
    rt.loom();
    rt.begin_frame(HostTime { tick: 3 });
    {
        let mut w = rt.port_writer();
        w.set_sense(inc, ZERO).unwrap();
        w.set_sense(dec, ONE).unwrap();
    }
    rt.loom();
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
        let mut b = Weave::builder("c").unwrap();
        let k_l = b.knot("l", KnotKind::constant(from_count(lhs))).unwrap();
        let k_r = b.knot("r", KnotKind::constant(from_count(rhs))).unwrap();
        let k_cmp = b.knot("cmp", KnotKind::compare(op, None)).unwrap();
        let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
        let from = b.output(&k_l, "out").unwrap();
        let to = b.input(&k_cmp, "lhs").unwrap();
        b.connect(from, to).unwrap();
        let from = b.output(&k_r, "out").unwrap();
        let to = b.input(&k_cmp, "rhs").unwrap();
        b.connect(from, to).unwrap();
        let from = b.output(&k_cmp, "out").unwrap();
        let to = b.input(&k_out, "in").unwrap();
        b.connect(from, to).unwrap();
        let weave = b.build().unwrap();
        let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
        rt.begin_frame(HostTime { tick: 0 });
        rt.loom();
        assert_eq!(truthy(&rt, "y"), expect, "{op:?}");
    }
}

#[test]
fn flag_setwins_reset_and_toggle_and_resetwins_set() {
    let mut b = Weave::builder("sw").unwrap();
    let k_set = b.knot("set", KnotKind::signal_in()).unwrap();
    let k_rst = b.knot("rst", KnotKind::signal_in()).unwrap();
    let k_tog = b.knot("tog", KnotKind::signal_in()).unwrap();
    let k_flag = b
        .knot("flag", KnotKind::flag(FlagPriority::SetWins, true))
        .unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_set, "out").unwrap();
    let to = b.input(&k_flag, "set").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_rst, "out").unwrap();
    let to = b.input(&k_flag, "reset").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_tog, "out").unwrap();
    let to = b.input(&k_flag, "toggle").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_flag, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let set = rt.sense_id("set").unwrap();
    let rst = rt.sense_id("rst").unwrap();
    let tog = rt.sense_id("tog").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ONE).unwrap();
        w.set_sense(rst, ZERO).unwrap();
        w.set_sense(tog, ZERO).unwrap();
    }
    rt.loom();
    assert!(truthy(&rt, "y"));

    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ZERO).unwrap();
        w.set_sense(rst, ONE).unwrap();
        w.set_sense(tog, ZERO).unwrap();
    }
    rt.loom();
    assert!(!truthy(&rt, "y"));

    rt.begin_frame(HostTime { tick: 2 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ZERO).unwrap();
        w.set_sense(rst, ZERO).unwrap();
        w.set_sense(tog, ZERO).unwrap();
    }
    rt.loom();
    rt.begin_frame(HostTime { tick: 3 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ZERO).unwrap();
        w.set_sense(rst, ZERO).unwrap();
        w.set_sense(tog, ONE).unwrap();
    }
    rt.loom();
    assert!(truthy(&rt, "y"));

    let mut b = Weave::builder("rw").unwrap();
    let k_set = b.knot("set", KnotKind::signal_in()).unwrap();
    let k_rst = b.knot("rst", KnotKind::signal_in()).unwrap();
    let k_flag = b
        .knot("flag", KnotKind::flag(FlagPriority::ResetWins, false))
        .unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_set, "out").unwrap();
    let to = b.input(&k_flag, "set").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_rst, "out").unwrap();
    let to = b.input(&k_flag, "reset").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_flag, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let set = rt.sense_id("set").unwrap();
    let rst = rt.sense_id("rst").unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(set, ONE).unwrap();
        w.set_sense(rst, ZERO).unwrap();
    }
    rt.loom();
    assert!(truthy(&rt, "y"));
}

#[test]
fn map_zero_span_and_delay_one() {
    let mut b = Weave::builder("z").unwrap();
    let k_c = b.knot("c", KnotKind::constant(from_count(1))).unwrap();
    let k_map = b
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
    let k_d = b.knot("d", KnotKind::Delay { ticks: 1 }).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let k_dout = b.knot("dout", KnotKind::signal_out("d")).unwrap();
    let from = b.output(&k_c, "out").unwrap();
    let to = b.input(&k_map, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_map, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_c, "out").unwrap();
    let to = b.input(&k_d, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_d, "out").unwrap();
    let to = b.input(&k_dout, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    assert_eq!(out_v(&rt, "y"), from_count(7));
    assert_eq!(out_v(&rt, "d"), ZERO); // delay 1
    rt.begin_frame(HostTime { tick: 1 });
    rt.loom();
    assert_eq!(out_v(&rt, "d"), from_count(1));
}

#[test]
fn digitize_four_steps_mid_bin() {
    let mut b = Weave::builder("dig").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
    let k_d = b
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
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_d, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_d, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let id = rt.sense_id("in").unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, from_count(2)).unwrap();
    rt.loom();
    assert_eq!(out_v(&rt, "y"), from_count(20));
}
