//! Shared Weave builders for Divan benches (included via `#[path]`).
//!
//! Not a stand-alone bench target (`autobenches = false` in Cargo.toml).
#![allow(dead_code)] // each bench binary uses a subset of helpers

use wyrd_core::{
    from_count, CalcOp, CompareOp, FlagPriority, KnotKind, Seed, TimerMode, ONE, ZERO,
};
use wyrd_graph::{
    Budget, KnotDef, Pattern, PatternDef, PatternExportDef, PortRefDef, ThreadDef, Weave, WeaveDef,
};
use wyrd_runtime::{BindOpts, Runtime};

/// Raised budgets for deep Not-chains (default hard depth is 16).
pub fn deep_budget() -> Budget {
    Budget {
        max_chain_depth: 512,
        max_knots: 512,
        max_threads: 1024,
        ..Budget::default()
    }
}

pub fn bind_deep(weave: &Weave) -> Runtime {
    Runtime::bind(
        weave.clone(),
        BindOpts {
            budget: deep_budget(),
            ..BindOpts::default()
        },
    )
    .unwrap()
}

/// Constant → Not × n → SignalOut (total knots ≈ n + 2).
pub fn chain_not_weave(n: usize) -> Weave {
    let mut b = Weave::builder("chain").unwrap();
    let mut prev = b.knot("c0", KnotKind::constant(ONE)).unwrap();
    for i in 0..n {
        let id = format!("n{i}");
        let next = b.knot(&id, KnotKind::not()).unwrap();
        let from = b.output(&prev, "out").unwrap();
        let to = b.input(&next, "in").unwrap();
        b.connect(from, to).unwrap();
        prev = next;
    }
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&prev, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    b.build().unwrap()
}

pub fn chain_not(n: usize) -> (Weave, Runtime) {
    let weave = chain_not_weave(n);
    let rt = bind_deep(&weave);
    (weave, rt)
}

/// Two plates → And → SignalOut (classic door).
pub fn and_door() -> (Weave, Runtime) {
    let mut b = Weave::builder("door").unwrap();
    let pa = b.knot("plate_a", KnotKind::signal_in()).unwrap();
    let pb = b.knot("plate_b", KnotKind::signal_in()).unwrap();
    let k_both = b.knot("both", KnotKind::and2()).unwrap();
    let from = b.output(&pa, "out").unwrap();
    let to = b.input(&k_both, "in_0").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&pb, "out").unwrap();
    let to = b.input(&k_both, "in_1").unwrap();
    b.connect(from, to).unwrap();
    let k_door = b.knot("door", KnotKind::signal_out("door.open")).unwrap();
    let from = b.output(&k_both, "out").unwrap();
    let to = b.input(&k_door, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    (weave, rt)
}

/// SignalIn → Map → Digitize → SignalOut (catalog math).
pub fn map_digitize_chain() -> (Weave, Runtime) {
    let mut b = Weave::builder("md").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
    let k_map = b
        .knot(
            "map",
            KnotKind::Map {
                in_min: ZERO,
                in_max: ONE,
                out_min: ZERO,
                out_max: ONE,
            },
        )
        .unwrap();
    let k_dig = b
        .knot(
            "dig",
            KnotKind::Digitize {
                steps: 8,
                in_min: ZERO,
                in_max: ONE,
                out_min: ZERO,
                out_max: ONE,
            },
        )
        .unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_map, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_map, "out").unwrap();
    let to = b.input(&k_dig, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_dig, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    (weave, rt)
}

/// SignalIn pair → Calc(Add) → Abs → SignalOut.
pub fn calc_abs_chain() -> (Weave, Runtime) {
    let mut b = Weave::builder("ca").unwrap();
    let k_a = b.knot("a", KnotKind::signal_in()).unwrap();
    let k_b = b.knot("b", KnotKind::signal_in()).unwrap();
    let k_add = b.knot("add", KnotKind::Calc { op: CalcOp::Add }).unwrap();
    let k_abs = b.knot("abs", KnotKind::Abs).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_a, "out").unwrap();
    let to = b.input(&k_add, "a").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_b, "out").unwrap();
    let to = b.input(&k_add, "b").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_add, "out").unwrap();
    let to = b.input(&k_abs, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_abs, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    (weave, rt)
}

/// SignalIn → Delay(n) → SignalOut.
pub fn delay_chain(ticks: u16) -> (Weave, Runtime) {
    let mut b = Weave::builder("dl").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
    let k_d = b.knot("d", KnotKind::Delay { ticks }).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_d, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_d, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    (weave, rt)
}

/// Gated Random → SignalOut (seeded).
pub fn random_gated() -> (Weave, Runtime) {
    let mut b = Weave::builder("rnd").unwrap();
    let k_g = b.knot("g", KnotKind::signal_in()).unwrap();
    let k_r = b.knot("r", KnotKind::random(true)).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_g, "out").unwrap();
    let to = b.input(&k_r, "gate").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_r, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = Runtime::bind(
        weave.clone(),
        BindOpts {
            seed: Some(Seed(0xBEEF_CAFE_u64)),
            ..BindOpts::default()
        },
    )
    .unwrap();
    (weave, rt)
}

/// SignalIn → Threshold → SignalOut (no hysteresis).
pub fn threshold_simple() -> (Weave, Runtime) {
    let mut b = Weave::builder("th").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
    let k_t = b.knot("t", KnotKind::threshold_default()).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_t, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_t, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    (weave, rt)
}

/// Wide fan-out: one Constant → n Not → each SignalOut (stress gather / outbox).
pub fn fanout_nots(n: usize) -> (Weave, Runtime) {
    let mut b = Weave::builder("fo").unwrap();
    let source = b.knot("c", KnotKind::constant(ONE)).unwrap();
    for i in 0..n {
        let nid = format!("n{i}");
        let oid = format!("o{i}");
        let not = b.knot(&nid, KnotKind::not()).unwrap();
        let out = b.knot(&oid, KnotKind::signal_out(format!("y{i}"))).unwrap();
        let from = b.output(&source, "out").unwrap();
        let to = b.input(&not, "in").unwrap();
        b.connect(from, to).unwrap();
        let from = b.output(&not, "out").unwrap();
        let to = b.input(&out, "in").unwrap();
        b.connect(from, to).unwrap();
    }
    let weave = b.build().unwrap();
    let mut budget = deep_budget();
    budget.max_fan_out = 512;
    budget.soft_fan_out = 512;
    let rt = Runtime::bind(
        weave.clone(),
        BindOpts {
            budget,
            ..BindOpts::default()
        },
    )
    .unwrap();
    (weave, rt)
}

fn bind_scaled(weave: &Weave, extra: impl FnOnce(&mut Budget)) -> Runtime {
    let mut budget = deep_budget();
    extra(&mut budget);
    Runtime::bind(
        weave.clone(),
        BindOpts {
            budget,
            ..BindOpts::default()
        },
    )
    .unwrap()
}

/// SignalIn → Map × n → SignalOut (identity-ish linear map).
pub fn chain_map(n: usize) -> (Weave, Runtime) {
    let mut b = Weave::builder("cmap").unwrap();
    let mut prev = b.knot("in", KnotKind::signal_in()).unwrap();
    for i in 0..n {
        let id = format!("m{i}");
        let next = b
            .knot(
                &id,
                KnotKind::Map {
                    in_min: ZERO,
                    in_max: ONE,
                    out_min: ZERO,
                    out_max: ONE,
                },
            )
            .unwrap();
        let from = b.output(&prev, "out").unwrap();
        let to = b.input(&next, "in").unwrap();
        b.connect(from, to).unwrap();
        prev = next;
    }
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&prev, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = bind_scaled(&weave, |_| {});
    (weave, rt)
}

/// SignalIn → Digitize × n → SignalOut.
pub fn chain_digitize(n: usize, steps: u16) -> (Weave, Runtime) {
    let mut b = Weave::builder("cdig").unwrap();
    let mut prev = b.knot("in", KnotKind::signal_in()).unwrap();
    for i in 0..n {
        let id = format!("d{i}");
        let next = b
            .knot(
                &id,
                KnotKind::Digitize {
                    steps,
                    in_min: ZERO,
                    in_max: ONE,
                    out_min: ZERO,
                    out_max: ONE,
                },
            )
            .unwrap();
        let from = b.output(&prev, "out").unwrap();
        let to = b.input(&next, "in").unwrap();
        b.connect(from, to).unwrap();
        prev = next;
    }
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&prev, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = bind_scaled(&weave, |_| {});
    (weave, rt)
}

/// SignalIn → Calc(Mul) × n with shared Constant(ONE) on every `b` port.
///
/// Uses **level** `ONE` so Q-mul stays non-zero under `signal-i32` (`ONE*ONE=ONE`).
/// Whole-count mul would collapse to 0 on i32 (documented dual-path trap).
pub fn chain_calc_mul(n: usize) -> (Weave, Runtime) {
    let mut b = Weave::builder("cmul").unwrap();
    let mut prev = b.knot("in", KnotKind::signal_in()).unwrap();
    let one = b.knot("one", KnotKind::constant(ONE)).unwrap();
    for i in 0..n {
        let id = format!("mul{i}");
        let next = b.knot(&id, KnotKind::Calc { op: CalcOp::Mul }).unwrap();
        let from = b.output(&prev, "out").unwrap();
        let to = b.input(&next, "a").unwrap();
        b.connect(from, to).unwrap();
        let from = b.output(&one, "out").unwrap();
        let to = b.input(&next, "b").unwrap();
        b.connect(from, to).unwrap();
        prev = next;
    }
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&prev, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = bind_scaled(&weave, |bud| {
        bud.max_fan_out = (n as u16).saturating_add(4).max(16);
        bud.soft_fan_out = bud.max_fan_out;
    });
    (weave, rt)
}

/// SignalIn → Sqrt × n → SignalOut (feed positive levels).
pub fn chain_sqrt(n: usize) -> (Weave, Runtime) {
    let mut b = Weave::builder("csqrt").unwrap();
    let mut prev = b.knot("in", KnotKind::signal_in()).unwrap();
    for i in 0..n {
        let id = format!("s{i}");
        let next = b.knot(&id, KnotKind::sqrt()).unwrap();
        let from = b.output(&prev, "out").unwrap();
        let to = b.input(&next, "in").unwrap();
        b.connect(from, to).unwrap();
        prev = next;
    }
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&prev, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = bind_scaled(&weave, |_| {});
    (weave, rt)
}

/// SignalIn → Delay(ticks) × n → SignalOut (ring traffic scales with n).
pub fn chain_delays(n: usize, ticks: u16) -> (Weave, Runtime) {
    let mut b = Weave::builder("cdel").unwrap();
    let mut prev = b.knot("in", KnotKind::signal_in()).unwrap();
    for i in 0..n {
        let id = format!("dl{i}");
        let next = b.knot(&id, KnotKind::Delay { ticks }).unwrap();
        let from = b.output(&prev, "out").unwrap();
        let to = b.input(&next, "in").unwrap();
        b.connect(from, to).unwrap();
        prev = next;
    }
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&prev, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let path_sum = (n as u32).saturating_mul(ticks as u32).min(u16::MAX as u32) as u16;
    let rt = bind_scaled(&weave, |bud| {
        bud.max_delay_path_sum = path_sum.max(32);
    });
    (weave, rt)
}

/// Puzzle-kit weave: Counter, Flag, PulseHold, FedCountdown + Rising edge.
///
/// Senses: `start` (edge into rising/flag/pulse), `feed` (fed timer + flag reset).
/// Outs: `count`, `flag`, `pulse`, `fed`.
pub fn stateful_kit() -> (Weave, Runtime) {
    let mut b = Weave::builder("kit").unwrap();
    let k_start = b.knot("start", KnotKind::signal_in()).unwrap();
    let k_feed = b.knot("feed", KnotKind::signal_in()).unwrap();
    let k_rise = b.knot("rise", KnotKind::rising_from_zero()).unwrap();
    let k_cnt = b.knot("cnt", KnotKind::counter()).unwrap();
    let k_flg = b
        .knot("flg", KnotKind::flag(FlagPriority::SetWins, false))
        .unwrap();
    let k_pulse = b
        .knot("pulse", KnotKind::timer(TimerMode::PulseHold, 2))
        .unwrap();
    // ticks=2 so a 4-phase script with two consecutive feed-high samples can arm active.
    let k_fed = b
        .knot("fed", KnotKind::timer(TimerMode::FedCountdown, 2))
        .unwrap();
    let k_out_c = b.knot("out_c", KnotKind::signal_out("count")).unwrap();
    let k_out_f = b.knot("out_f", KnotKind::signal_out("flag")).unwrap();
    let k_out_p = b.knot("out_p", KnotKind::signal_out("pulse")).unwrap();
    let k_out_d = b.knot("out_d", KnotKind::signal_out("fed")).unwrap();
    let from = b.output(&k_start, "out").unwrap();
    let to = b.input(&k_rise, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_rise, "out").unwrap();
    let to = b.input(&k_cnt, "inc").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_start, "out").unwrap();
    let to = b.input(&k_flg, "set").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_feed, "out").unwrap();
    let to = b.input(&k_flg, "reset").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_rise, "out").unwrap();
    let to = b.input(&k_pulse, "start").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_feed, "out").unwrap();
    let to = b.input(&k_fed, "feed").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_cnt, "count").unwrap();
    let to = b.input(&k_out_c, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_flg, "out").unwrap();
    let to = b.input(&k_out_f, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_pulse, "active").unwrap();
    let to = b.input(&k_out_p, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_fed, "active").unwrap();
    let to = b.input(&k_out_d, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    (weave, rt)
}

/// Shared gate → n EmitCommand (raise max_emits_per_tick).
pub fn emit_storm(n: usize) -> (Weave, Runtime) {
    let mut b = Weave::builder("em").unwrap();
    let gate = b.knot("g", KnotKind::signal_in()).unwrap();
    for i in 0..n {
        let id = format!("e{i}");
        let emit = b
            .knot(&id, KnotKind::emit_command(format!("cmd{i}")))
            .unwrap();
        let from = b.output(&gate, "out").unwrap();
        let to = b.input(&emit, "trigger").unwrap();
        b.connect(from, to).unwrap();
    }
    let weave = b.build().unwrap();
    let mut budget = deep_budget();
    budget.max_fan_out = (n as u16).saturating_add(4).max(16);
    budget.soft_fan_out = budget.max_fan_out;
    let rt = Runtime::bind(
        weave.clone(),
        BindOpts {
            budget,
            max_emits_per_tick: (n as u16).saturating_add(1).max(8),
            ..BindOpts::default()
        },
    )
    .unwrap();
    (weave, rt)
}

/// SignalIn → Calc(Div) × n with shared ONE divisor (non-zero dual-path).
pub fn chain_calc_div(n: usize) -> (Weave, Runtime) {
    let mut b = Weave::builder("cdiv").unwrap();
    let mut prev = b.knot("in", KnotKind::signal_in()).unwrap();
    let one = b.knot("one", KnotKind::constant(ONE)).unwrap();
    for i in 0..n {
        let id = format!("div{i}");
        let next = b.knot(&id, KnotKind::Calc { op: CalcOp::Div }).unwrap();
        let from = b.output(&prev, "out").unwrap();
        let to = b.input(&next, "a").unwrap();
        b.connect(from, to).unwrap();
        let from = b.output(&one, "out").unwrap();
        let to = b.input(&next, "b").unwrap();
        b.connect(from, to).unwrap();
        prev = next;
    }
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&prev, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = bind_scaled(&weave, |bud| {
        bud.max_fan_out = (n as u16).saturating_add(4).max(16);
        bud.soft_fan_out = bud.max_fan_out;
    });
    (weave, rt)
}

/// SignalIn → Rising / Falling / Change → three SignalOuts.
pub fn edges_pack() -> (Weave, Runtime) {
    let mut b = Weave::builder("edg").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
    let k_r = b.knot("r", KnotKind::rising_from_zero()).unwrap();
    let k_f = b.knot("f", KnotKind::falling_to_zero()).unwrap();
    let k_c = b.knot("c", KnotKind::change()).unwrap();
    let k_or = b.knot("or", KnotKind::signal_out("rise")).unwrap();
    let k_of = b.knot("of", KnotKind::signal_out("fall")).unwrap();
    let k_oc = b.knot("oc", KnotKind::signal_out("chg")).unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_r, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_f, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_c, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_r, "out").unwrap();
    let to = b.input(&k_or, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_f, "out").unwrap();
    let to = b.input(&k_of, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_c, "out").unwrap();
    let to = b.input(&k_oc, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    // Fan-out 3 from `in` fits default max_fan_out (8).
    let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    (weave, rt)
}

/// Or(2), Xor, Select with constants + SignalIn sel/a.
pub fn logic_pack() -> (Weave, Runtime) {
    let mut b = Weave::builder("log").unwrap();
    let k_a = b.knot("a", KnotKind::signal_in()).unwrap();
    let k_b = b.knot("b", KnotKind::signal_in()).unwrap();
    let k_sel = b.knot("sel", KnotKind::signal_in()).unwrap();
    let k_or = b.knot("or", KnotKind::or2()).unwrap();
    let k_xor = b.knot("xor", KnotKind::xor()).unwrap();
    let k_selk = b.knot("selk", KnotKind::select()).unwrap();
    let k_ca = b.knot("ca", KnotKind::constant(ZERO)).unwrap();
    let k_cb = b.knot("cb", KnotKind::constant(ONE)).unwrap();
    let k_oo = b.knot("oo", KnotKind::signal_out("or")).unwrap();
    let k_ox = b.knot("ox", KnotKind::signal_out("xor")).unwrap();
    let k_os = b.knot("os", KnotKind::signal_out("sel")).unwrap();
    let from = b.output(&k_a, "out").unwrap();
    let to = b.input(&k_or, "in_0").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_b, "out").unwrap();
    let to = b.input(&k_or, "in_1").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_a, "out").unwrap();
    let to = b.input(&k_xor, "a").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_b, "out").unwrap();
    let to = b.input(&k_xor, "b").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_sel, "out").unwrap();
    let to = b.input(&k_selk, "sel").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_ca, "out").unwrap();
    let to = b.input(&k_selk, "a").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_cb, "out").unwrap();
    let to = b.input(&k_selk, "b").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_or, "out").unwrap();
    let to = b.input(&k_oo, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_xor, "out").unwrap();
    let to = b.input(&k_ox, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_selk, "out").unwrap();
    let to = b.input(&k_os, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    (weave, rt)
}

/// SignalIn → (Neg → Clamp) × n layers → Out.
pub fn chain_clamp_neg(n: usize) -> (Weave, Runtime) {
    let mut b = Weave::builder("ccl").unwrap();
    let mut prev = b.knot("in", KnotKind::signal_in()).unwrap();
    for i in 0..n {
        let neg = format!("n{i}");
        let cl = format!("c{i}");
        let neg_knot = b.knot(&neg, KnotKind::Neg).unwrap();
        let clamp = b
            .knot(&cl, KnotKind::clamp(from_count(-2), from_count(2)))
            .unwrap();
        let from = b.output(&prev, "out").unwrap();
        let to = b.input(&neg_knot, "in").unwrap();
        b.connect(from, to).unwrap();
        let from = b.output(&neg_knot, "out").unwrap();
        let to = b.input(&clamp, "in").unwrap();
        b.connect(from, to).unwrap();
        prev = clamp;
    }
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&prev, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = bind_scaled(&weave, |_| {});
    (weave, rt)
}

/// SignalIn → Compare(Gte, rhs_const=0) × n → Out.
pub fn chain_compare(n: usize) -> (Weave, Runtime) {
    let mut b = Weave::builder("ccmp").unwrap();
    let mut prev = b.knot("in", KnotKind::signal_in()).unwrap();
    for i in 0..n {
        let id = format!("cmp{i}");
        let next = b
            .knot(&id, KnotKind::compare(CompareOp::Gte, Some(0)))
            .unwrap();
        let from = b.output(&prev, "out").unwrap();
        let to = b.input(&next, "lhs").unwrap();
        b.connect(from, to).unwrap();
        prev = next;
    }
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&prev, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = bind_scaled(&weave, |_| {});
    (weave, rt)
}

/// OnStart → SignalOut (first-frame pulse; low value as ongoing bench).
pub fn onstart_out() -> (Weave, Runtime) {
    let mut b = Weave::builder("ons").unwrap();
    let k_s = b.knot("s", KnotKind::OnStart).unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_s, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    let weave = b.build().unwrap();
    let rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
    (weave, rt)
}

/// Small authored graph used for bind-cost benches (not deep).
pub fn small_authored_weave() -> Weave {
    let mut b = Weave::builder("bind_me").unwrap();
    let k_in = b.knot("in", KnotKind::signal_in()).unwrap();
    let k_n = b.knot("n", KnotKind::not()).unwrap();
    let k_map = b
        .knot(
            "map",
            KnotKind::Map {
                in_min: from_count(0),
                in_max: from_count(1),
                out_min: from_count(0),
                out_max: from_count(10),
            },
        )
        .unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let from = b.output(&k_in, "out").unwrap();
    let to = b.input(&k_n, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_n, "out").unwrap();
    let to = b.input(&k_map, "in").unwrap();
    b.connect(from, to).unwrap();
    let from = b.output(&k_map, "out").unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(from, to).unwrap();
    b.build().unwrap()
}

/// Monostable pattern (RisingFromZero → PulseHold) — same shape as cookbook.
pub fn monostable_pattern() -> Pattern {
    Pattern::try_from(PatternDef {
        id: "pat.mono".into(),
        inner: WeaveDef {
            id: "pat.mono.inner".into(),
            numeric: wyrd_core::NumericPath::compiled(),
            knots: vec![
                KnotDef {
                    id: "edge".into(),
                    kind: KnotKind::rising_from_zero(),
                },
                KnotDef {
                    id: "t".into(),
                    kind: KnotKind::timer(TimerMode::PulseHold, 2),
                },
            ],
            threads: vec![ThreadDef {
                from: PortRefDef::new("edge", "out"),
                to: PortRefDef::new("t", "start"),
            }],
        },
        inputs: vec![PatternExportDef::new("start", "edge", "in")],
        outputs: vec![PatternExportDef::new("active", "t", "active")],
    })
    .unwrap()
}

/// Expand monostable only (no Runtime). Returns expanded knot count.
pub fn expand_monostable_once() -> usize {
    let p = monostable_pattern();
    let mut b = Weave::builder("expand-bench").unwrap();
    let trigger = b.knot("trigger", KnotKind::signal_in()).unwrap();
    let sink = b.knot("sink", KnotKind::signal_out("sink")).unwrap();
    let instance = b.include("hold1", &p).unwrap();
    let from = b.output(&trigger, "out").unwrap();
    let to = instance.input("start").unwrap();
    b.connect(from, to).unwrap();
    let from = instance.output("active").unwrap();
    let to = b.input(&sink, "in").unwrap();
    b.connect(from, to).unwrap();
    b.build().unwrap().knots().len()
}

/// Parent: SignalIn + include monostable + SignalOut → ready-to-bind Weave.
pub fn weave_with_monostable_include() -> Weave {
    let pat = monostable_pattern();
    let mut b = Weave::builder("lvl").unwrap();
    let k_btn = b.knot("btn", KnotKind::signal_in()).unwrap();
    let exp = b.include("hold1", &pat).unwrap();
    let start = exp.input("start").unwrap();
    let active = exp.output("active").unwrap();
    let k_out = b.knot("out", KnotKind::signal_out("lamp")).unwrap();
    let from = b.output(&k_btn, "out").unwrap();
    b.connect(from, start).unwrap();
    let to = b.input(&k_out, "in").unwrap();
    b.connect(active, to).unwrap();
    b.build().unwrap()
}
