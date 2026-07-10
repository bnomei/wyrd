//! Counter rising-edge, Flag toggle/reset, Emit rising-edge (step 1.3).

use wyrd_core::{FlagPriority, HostTime, KnotKind, ONE, ZERO};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

/// Whole-unit count from SignalOut (works for f32 and i32 Q paths via from_count).
fn count_out(rt: &Runtime) -> i32 {
    use wyrd_core::from_count;
    let pid = rt.path_id("count").unwrap();
    let v = rt
        .outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| s.value)
        .unwrap_or(ZERO);
    // Compare against from_count ladder so dual-path stays honest.
    for n in 0..64 {
        if v == from_count(n) {
            return n;
        }
    }
    #[cfg(feature = "signal-f32")]
    {
        v as i32
    }
    #[cfg(feature = "signal-i32")]
    {
        v
    }
}

fn signal_truthy(rt: &Runtime, path: &str) -> bool {
    let pid = rt.path_id(path).unwrap();
    rt.outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| wyrd_core::is_truthy(s.value))
        .unwrap_or(false)
}

#[test]
fn counter_rising_edge_not_level() {
    let mut b = Weave::builder("c").unwrap();
    let k_inc = b.knot("inc", KnotKind::signal_in()).unwrap();
    let k_cnt = b.knot("cnt", KnotKind::counter()).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("count")).unwrap();
    let from = b.output(&k_inc, "out").unwrap();
    let to = b.input(&k_cnt, "inc").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_cnt, "count").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let id = rt.sense_id("inc").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert_eq!(count_out(&rt), 1);

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert_eq!(count_out(&rt), 1);

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, ZERO).unwrap();
    rt.loom();
    rt.begin_frame(HostTime { tick: 3 });
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert_eq!(count_out(&rt), 2);
}

#[test]
fn counter_reset_then_rising_inc_same_tick() {
    let mut b = Weave::builder("cr").unwrap();
    let k_inc = b.knot("inc", KnotKind::signal_in()).unwrap();
    let k_rst = b.knot("rst", KnotKind::signal_in()).unwrap();
    let k_cnt = b.knot("cnt", KnotKind::counter()).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("count")).unwrap();
    let from = b.output(&k_inc, "out").unwrap();
    let to = b.input(&k_cnt, "inc").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_rst, "out").unwrap();
    let to = b.input(&k_cnt, "reset").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_cnt, "count").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let inc = rt.sense_id("inc").unwrap();
    let rst = rt.sense_id("rst").unwrap();

    for t in 0..5u64 {
        rt.begin_frame(HostTime { tick: t });
        {
            let mut w = rt.port_writer();
            w.set_sense(inc, if t % 2 == 0 { ONE } else { ZERO })
                .unwrap();
            w.set_sense(rst, ZERO).unwrap();
        }
        rt.loom();
    }
    rt.begin_frame(HostTime { tick: 10 });
    {
        let mut w = rt.port_writer();
        w.set_sense(inc, ZERO).unwrap();
        w.set_sense(rst, ZERO).unwrap();
    }
    rt.loom();
    rt.begin_frame(HostTime { tick: 11 });
    {
        let mut w = rt.port_writer();
        w.set_sense(inc, ONE).unwrap();
        w.set_sense(rst, ONE).unwrap();
    }
    rt.loom();
    assert_eq!(count_out(&rt), 1);
}

#[test]
fn flag_toggle_rising_and_reset() {
    let mut b = Weave::builder("f").unwrap();
    let k_tog = b.knot("tog", KnotKind::signal_in()).unwrap();
    let k_rst = b.knot("rst", KnotKind::signal_in()).unwrap();
    let k_flag = b
        .knot("flag", KnotKind::flag(FlagPriority::ResetWins, true))
        .unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("lamp")).unwrap();
    let from = b.output(&k_tog, "out").unwrap();
    let to = b.input(&k_flag, "toggle").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_rst, "out").unwrap();
    let to = b.input(&k_flag, "reset").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_flag, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let tog = rt.sense_id("tog").unwrap();
    let rst = rt.sense_id("rst").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut w = rt.port_writer();
        w.set_sense(tog, ONE).unwrap();
        w.set_sense(rst, ZERO).unwrap();
    }
    rt.loom();
    assert!(signal_truthy(&rt, "lamp"));

    rt.begin_frame(HostTime { tick: 1 });
    {
        let mut w = rt.port_writer();
        w.set_sense(tog, ONE).unwrap();
        w.set_sense(rst, ZERO).unwrap();
    }
    rt.loom();
    assert!(signal_truthy(&rt, "lamp"));

    rt.begin_frame(HostTime { tick: 2 });
    {
        let mut w = rt.port_writer();
        w.set_sense(tog, ZERO).unwrap();
        w.set_sense(rst, ONE).unwrap();
    }
    rt.loom();
    assert!(!signal_truthy(&rt, "lamp"));
}

#[test]
fn emit_once_on_held_trigger() {
    let mut b = Weave::builder("e").unwrap();
    let k_btn = b.knot("btn", KnotKind::signal_in()).unwrap();
    let k_em = b.knot("em", KnotKind::emit_command("fire")).unwrap();
    let from = b.output(&k_btn, "out").unwrap();
    let to = b.input(&k_em, "trigger").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    let id = rt.sense_id("btn").unwrap();

    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert_eq!(rt.outbox().emits().len(), 1);

    rt.begin_frame(HostTime { tick: 1 });
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert_eq!(rt.outbox().emits().len(), 0);

    rt.begin_frame(HostTime { tick: 2 });
    rt.port_writer().set_sense(id, ZERO).unwrap();
    rt.loom();
    rt.begin_frame(HostTime { tick: 3 });
    rt.port_writer().set_sense(id, ONE).unwrap();
    rt.loom();
    assert_eq!(rt.outbox().emits().len(), 1);
}
