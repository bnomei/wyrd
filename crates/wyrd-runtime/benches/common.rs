//! Shared Weave builders for Divan benches (included via `#[path]`).
//!
//! Not a stand-alone bench target (`autobenches = false` in Cargo.toml).
#![allow(dead_code)] // each bench binary uses a subset of helpers

use wyrd_core::{
    from_count, CalcOp, CompareOp, FlagPriority, KnotKind, Seed, TimerMode, ONE, ZERO,
};
use wyrd_graph::{expand_pattern, Budget, Pattern, PortRefAuthor, Weave};
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
        weave,
        BindOpts {
            budget: deep_budget(),
            ..BindOpts::default()
        },
    )
    .unwrap()
}

/// Constant → Not × n → SignalOut (total knots ≈ n + 2).
pub fn chain_not_weave(n: usize) -> Weave {
    let (mut b, _) = Weave::builder("chain")
        .knot("c0", KnotKind::constant(ONE))
        .unwrap();
    let mut prev = "c0".to_string();
    for i in 0..n {
        let id = format!("n{i}");
        let (b2, _) = b.knot(&id, KnotKind::not()).unwrap();
        b = b2.wire_named(&prev, "out", &id, "in");
        prev = id;
    }
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    b.wire_named(&prev, "out", "out", "in").build().unwrap()
}

pub fn chain_not(n: usize) -> (Weave, Runtime) {
    let weave = chain_not_weave(n);
    let rt = bind_deep(&weave);
    (weave, rt)
}

/// Two plates → And → SignalOut (classic door).
pub fn and_door() -> (Weave, Runtime) {
    let (b, pa) = Weave::builder("door")
        .knot("plate_a", KnotKind::signal_in())
        .unwrap();
    let (b, pb) = b.knot("plate_b", KnotKind::signal_in()).unwrap();
    let (b, _) = b.and2("both", pa, pb).unwrap();
    let (b, _) = b.knot("door", KnotKind::signal_out("door.open")).unwrap();
    let weave = b.wire_named("both", "out", "door", "in").build().unwrap();
    let rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    (weave, rt)
}

/// SignalIn → Map → Digitize → SignalOut (catalog math).
pub fn map_digitize_chain() -> (Weave, Runtime) {
    let (b, _) = Weave::builder("md")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b
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
    let (b, _) = b
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
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("in", "out", "map", "in")
        .wire_named("map", "out", "dig", "in")
        .wire_named("dig", "out", "out", "in")
        .build()
        .unwrap();
    let rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    (weave, rt)
}

/// SignalIn pair → Calc(Add) → Abs → SignalOut.
pub fn calc_abs_chain() -> (Weave, Runtime) {
    let (b, _) = Weave::builder("ca")
        .knot("a", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("b", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("add", KnotKind::Calc { op: CalcOp::Add }).unwrap();
    let (b, _) = b.knot("abs", KnotKind::Abs).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("a", "out", "add", "a")
        .wire_named("b", "out", "add", "b")
        .wire_named("add", "out", "abs", "in")
        .wire_named("abs", "out", "out", "in")
        .build()
        .unwrap();
    let rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    (weave, rt)
}

/// SignalIn → Delay(n) → SignalOut.
pub fn delay_chain(ticks: u16) -> (Weave, Runtime) {
    let (b, _) = Weave::builder("dl")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("d", KnotKind::Delay { ticks }).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("in", "out", "d", "in")
        .wire_named("d", "out", "out", "in")
        .build()
        .unwrap();
    let rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    (weave, rt)
}

/// Gated Random → SignalOut (seeded).
pub fn random_gated() -> (Weave, Runtime) {
    let (b, _) = Weave::builder("rnd")
        .knot("g", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("r", KnotKind::random(true)).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("g", "out", "r", "gate")
        .wire_named("r", "out", "out", "in")
        .build()
        .unwrap();
    let rt = Runtime::bind(
        &weave,
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
    let (b, _) = Weave::builder("th")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("t", KnotKind::threshold_default()).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b
        .wire_named("in", "out", "t", "in")
        .wire_named("t", "out", "out", "in")
        .build()
        .unwrap();
    let rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    (weave, rt)
}

/// Wide fan-out: one Constant → n Not → each SignalOut (stress gather / outbox).
pub fn fanout_nots(n: usize) -> (Weave, Runtime) {
    let (mut b, _) = Weave::builder("fo")
        .knot("c", KnotKind::constant(ONE))
        .unwrap();
    for i in 0..n {
        let nid = format!("n{i}");
        let oid = format!("o{i}");
        let (b2, _) = b.knot(&nid, KnotKind::not()).unwrap();
        let (b3, _) = b2
            .knot(&oid, KnotKind::signal_out(format!("y{i}")))
            .unwrap();
        b = b3
            .wire_named("c", "out", &nid, "in")
            .wire_named(&nid, "out", &oid, "in");
    }
    let weave = b.build().unwrap();
    let mut budget = deep_budget();
    budget.max_fan_out = 512;
    budget.soft_fan_out = 512;
    let rt = Runtime::bind(
        &weave,
        BindOpts {
            budget,
            ..BindOpts::default()
        },
    )
    .unwrap();
    (weave, rt)
}

// ---------------------------------------------------------------------------
// P0: scaled chains (amortize fixed loom tax)
// ---------------------------------------------------------------------------

fn bind_scaled(weave: &Weave, extra: impl FnOnce(&mut Budget)) -> Runtime {
    let mut budget = deep_budget();
    extra(&mut budget);
    Runtime::bind(
        weave,
        BindOpts {
            budget,
            ..BindOpts::default()
        },
    )
    .unwrap()
}

/// SignalIn → Map × n → SignalOut (identity-ish linear map).
pub fn chain_map(n: usize) -> (Weave, Runtime) {
    let (mut b, _) = Weave::builder("cmap")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let mut prev = "in".to_string();
    for i in 0..n {
        let id = format!("m{i}");
        let (b2, _) = b
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
        b = b2.wire_named(&prev, "out", &id, "in");
        prev = id;
    }
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b.wire_named(&prev, "out", "out", "in").build().unwrap();
    let rt = bind_scaled(&weave, |_| {});
    (weave, rt)
}

/// SignalIn → Digitize × n → SignalOut.
pub fn chain_digitize(n: usize, steps: u16) -> (Weave, Runtime) {
    let (mut b, _) = Weave::builder("cdig")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let mut prev = "in".to_string();
    for i in 0..n {
        let id = format!("d{i}");
        let (b2, _) = b
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
        b = b2.wire_named(&prev, "out", &id, "in");
        prev = id;
    }
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b.wire_named(&prev, "out", "out", "in").build().unwrap();
    let rt = bind_scaled(&weave, |_| {});
    (weave, rt)
}

/// SignalIn → Calc(Mul) × n with shared Constant(ONE) on every `b` port.
///
/// Uses **level** `ONE` so Q-mul stays non-zero under `signal-i32` (`ONE*ONE=ONE`).
/// Whole-count mul would collapse to 0 on i32 (documented dual-path trap).
pub fn chain_calc_mul(n: usize) -> (Weave, Runtime) {
    let (mut b, _) = Weave::builder("cmul")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b2, _) = b.knot("one", KnotKind::constant(ONE)).unwrap();
    b = b2;
    let mut prev = "in".to_string();
    for i in 0..n {
        let id = format!("mul{i}");
        let (b2, _) = b.knot(&id, KnotKind::Calc { op: CalcOp::Mul }).unwrap();
        b = b2
            .wire_named(&prev, "out", &id, "a")
            .wire_named("one", "out", &id, "b");
        prev = id;
    }
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b.wire_named(&prev, "out", "out", "in").build().unwrap();
    let rt = bind_scaled(&weave, |bud| {
        bud.max_fan_out = (n as u16).saturating_add(4).max(16);
        bud.soft_fan_out = bud.max_fan_out;
    });
    (weave, rt)
}

/// SignalIn → Sqrt × n → SignalOut (feed positive levels).
pub fn chain_sqrt(n: usize) -> (Weave, Runtime) {
    let (mut b, _) = Weave::builder("csqrt")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let mut prev = "in".to_string();
    for i in 0..n {
        let id = format!("s{i}");
        let (b2, _) = b.knot(&id, KnotKind::sqrt()).unwrap();
        b = b2.wire_named(&prev, "out", &id, "in");
        prev = id;
    }
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b.wire_named(&prev, "out", "out", "in").build().unwrap();
    let rt = bind_scaled(&weave, |_| {});
    (weave, rt)
}

/// SignalIn → Delay(ticks) × n → SignalOut (ring traffic scales with n).
pub fn chain_delays(n: usize, ticks: u16) -> (Weave, Runtime) {
    let (mut b, _) = Weave::builder("cdel")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let mut prev = "in".to_string();
    for i in 0..n {
        let id = format!("dl{i}");
        let (b2, _) = b.knot(&id, KnotKind::Delay { ticks }).unwrap();
        b = b2.wire_named(&prev, "out", &id, "in");
        prev = id;
    }
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b.wire_named(&prev, "out", "out", "in").build().unwrap();
    let path_sum = (n as u32).saturating_mul(ticks as u32).min(u16::MAX as u32) as u16;
    let rt = bind_scaled(&weave, |bud| {
        bud.max_delay_path_sum = path_sum.max(32);
    });
    (weave, rt)
}

// ---------------------------------------------------------------------------
// P1: product stateful + emit storm
// ---------------------------------------------------------------------------

/// Puzzle-kit weave: Counter, Flag, PulseHold, FedCountdown + Rising edge.
///
/// Senses: `start` (edge into rising/flag/pulse), `feed` (fed timer + flag reset).
/// Outs: `count`, `flag`, `pulse`, `fed`.
pub fn stateful_kit() -> (Weave, Runtime) {
    let (b, _) = Weave::builder("kit")
        .knot("start", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("feed", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("rise", KnotKind::rising_from_zero()).unwrap();
    let (b, _) = b.knot("cnt", KnotKind::counter()).unwrap();
    let (b, _) = b
        .knot("flg", KnotKind::flag(FlagPriority::SetWins, false))
        .unwrap();
    let (b, _) = b
        .knot("pulse", KnotKind::timer(TimerMode::PulseHold, 2))
        .unwrap();
    // ticks=2 so a 4-phase script with two consecutive feed-high samples can arm active.
    let (b, _) = b
        .knot("fed", KnotKind::timer(TimerMode::FedCountdown, 2))
        .unwrap();
    let (b, _) = b.knot("out_c", KnotKind::signal_out("count")).unwrap();
    let (b, _) = b.knot("out_f", KnotKind::signal_out("flag")).unwrap();
    let (b, _) = b.knot("out_p", KnotKind::signal_out("pulse")).unwrap();
    let (b, _) = b.knot("out_d", KnotKind::signal_out("fed")).unwrap();
    let weave = b
        .wire_named("start", "out", "rise", "in")
        .wire_named("rise", "out", "cnt", "inc")
        .wire_named("start", "out", "flg", "set")
        .wire_named("feed", "out", "flg", "reset")
        .wire_named("rise", "out", "pulse", "start")
        .wire_named("feed", "out", "fed", "feed")
        .wire_named("cnt", "count", "out_c", "in")
        .wire_named("flg", "out", "out_f", "in")
        .wire_named("pulse", "active", "out_p", "in")
        .wire_named("fed", "active", "out_d", "in")
        .build()
        .unwrap();
    let rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    (weave, rt)
}

/// Shared gate → n EmitCommand (raise max_emits_per_tick).
pub fn emit_storm(n: usize) -> (Weave, Runtime) {
    let (mut b, _) = Weave::builder("em")
        .knot("g", KnotKind::signal_in())
        .unwrap();
    for i in 0..n {
        let id = format!("e{i}");
        let (b2, _) = b
            .knot(&id, KnotKind::emit_command(format!("cmd{i}")))
            .unwrap();
        b = b2.wire_named("g", "out", &id, "trigger");
    }
    let weave = b.build().unwrap();
    let mut budget = deep_budget();
    budget.max_fan_out = (n as u16).saturating_add(4).max(16);
    budget.soft_fan_out = budget.max_fan_out;
    let rt = Runtime::bind(
        &weave,
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
    let (mut b, _) = Weave::builder("cdiv")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b2, _) = b.knot("one", KnotKind::constant(ONE)).unwrap();
    b = b2;
    let mut prev = "in".to_string();
    for i in 0..n {
        let id = format!("div{i}");
        let (b2, _) = b.knot(&id, KnotKind::Calc { op: CalcOp::Div }).unwrap();
        b = b2
            .wire_named(&prev, "out", &id, "a")
            .wire_named("one", "out", &id, "b");
        prev = id;
    }
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b.wire_named(&prev, "out", "out", "in").build().unwrap();
    let rt = bind_scaled(&weave, |bud| {
        bud.max_fan_out = (n as u16).saturating_add(4).max(16);
        bud.soft_fan_out = bud.max_fan_out;
    });
    (weave, rt)
}

// ---------------------------------------------------------------------------
// P2: edges + remaining catalog surface
// ---------------------------------------------------------------------------

/// SignalIn → Rising / Falling / Change → three SignalOuts.
pub fn edges_pack() -> (Weave, Runtime) {
    let (b, _) = Weave::builder("edg")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("r", KnotKind::rising_from_zero()).unwrap();
    let (b, _) = b.knot("f", KnotKind::falling_to_zero()).unwrap();
    let (b, _) = b.knot("c", KnotKind::change()).unwrap();
    let (b, _) = b.knot("or", KnotKind::signal_out("rise")).unwrap();
    let (b, _) = b.knot("of", KnotKind::signal_out("fall")).unwrap();
    let (b, _) = b.knot("oc", KnotKind::signal_out("chg")).unwrap();
    let weave = b
        .wire_named("in", "out", "r", "in")
        .wire_named("in", "out", "f", "in")
        .wire_named("in", "out", "c", "in")
        .wire_named("r", "out", "or", "in")
        .wire_named("f", "out", "of", "in")
        .wire_named("c", "out", "oc", "in")
        .build()
        .unwrap();
    // Fan-out 3 from `in` fits default max_fan_out (8).
    let rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    (weave, rt)
}

/// Or(2), Xor, Select with constants + SignalIn sel/a.
pub fn logic_pack() -> (Weave, Runtime) {
    let (b, _) = Weave::builder("log")
        .knot("a", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("b", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("sel", KnotKind::signal_in()).unwrap();
    let (b, _) = b.knot("or", KnotKind::or2()).unwrap();
    let (b, _) = b.knot("xor", KnotKind::xor()).unwrap();
    let (b, _) = b.knot("selk", KnotKind::select()).unwrap();
    let (b, _) = b.knot("ca", KnotKind::constant(ZERO)).unwrap();
    let (b, _) = b.knot("cb", KnotKind::constant(ONE)).unwrap();
    let (b, _) = b.knot("oo", KnotKind::signal_out("or")).unwrap();
    let (b, _) = b.knot("ox", KnotKind::signal_out("xor")).unwrap();
    let (b, _) = b.knot("os", KnotKind::signal_out("sel")).unwrap();
    let weave = b
        .wire_named("a", "out", "or", "in_0")
        .wire_named("b", "out", "or", "in_1")
        .wire_named("a", "out", "xor", "a")
        .wire_named("b", "out", "xor", "b")
        .wire_named("sel", "out", "selk", "sel")
        .wire_named("ca", "out", "selk", "a")
        .wire_named("cb", "out", "selk", "b")
        .wire_named("or", "out", "oo", "in")
        .wire_named("xor", "out", "ox", "in")
        .wire_named("selk", "out", "os", "in")
        .build()
        .unwrap();
    let rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    (weave, rt)
}

/// SignalIn → (Neg → Clamp) × n layers → Out.
pub fn chain_clamp_neg(n: usize) -> (Weave, Runtime) {
    let (mut b, _) = Weave::builder("ccl")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let mut prev = "in".to_string();
    for i in 0..n {
        let neg = format!("n{i}");
        let cl = format!("c{i}");
        let (b2, _) = b.knot(&neg, KnotKind::Neg).unwrap();
        let (b3, _) = b2
            .knot(&cl, KnotKind::clamp(from_count(-2), from_count(2)))
            .unwrap();
        b = b3
            .wire_named(&prev, "out", &neg, "in")
            .wire_named(&neg, "out", &cl, "in");
        prev = cl;
    }
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b.wire_named(&prev, "out", "out", "in").build().unwrap();
    let rt = bind_scaled(&weave, |_| {});
    (weave, rt)
}

/// SignalIn → Compare(Gte, rhs_const=0) × n → Out.
pub fn chain_compare(n: usize) -> (Weave, Runtime) {
    let (mut b, _) = Weave::builder("ccmp")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let mut prev = "in".to_string();
    for i in 0..n {
        let id = format!("cmp{i}");
        let (b2, _) = b
            .knot(&id, KnotKind::compare(CompareOp::Gte, Some(0)))
            .unwrap();
        b = b2.wire_named(&prev, "out", &id, "lhs");
        prev = id;
    }
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b.wire_named(&prev, "out", "out", "in").build().unwrap();
    let rt = bind_scaled(&weave, |_| {});
    (weave, rt)
}

/// OnStart → SignalOut (first-frame pulse; low value as ongoing bench).
pub fn onstart_out() -> (Weave, Runtime) {
    let (b, _) = Weave::builder("ons").knot("s", KnotKind::OnStart).unwrap();
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b.wire_named("s", "out", "out", "in").build().unwrap();
    let rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    (weave, rt)
}

/// Small authored graph used for bind-cost benches (not deep).
pub fn small_authored_weave() -> Weave {
    let (b, _) = Weave::builder("bind_me")
        .knot("in", KnotKind::signal_in())
        .unwrap();
    let (b, _) = b.knot("n", KnotKind::not()).unwrap();
    let (b, _) = b
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
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    b.wire_named("in", "out", "n", "in")
        .wire_named("n", "out", "map", "in")
        .wire_named("map", "out", "out", "in")
        .build()
        .unwrap()
}

// ---------------------------------------------------------------------------
// P3: pattern expand / include (load path)
// ---------------------------------------------------------------------------

/// Monostable pattern (RisingFromZero → PulseHold) — same shape as cookbook.
pub fn monostable_pattern() -> Pattern {
    let (b, _) = Weave::builder("pat.mono")
        .knot("edge", KnotKind::rising_from_zero())
        .unwrap();
    let (b, _) = b
        .knot("t", KnotKind::timer(TimerMode::PulseHold, 2))
        .unwrap();
    let inner = b.wire_named("edge", "out", "t", "start").build().unwrap();
    Pattern {
        id: "pat.mono".into(),
        inner,
        exports_in: vec![("start".into(), "edge".into(), "in".into())],
        exports_out: vec![("active".into(), "t".into(), "active".into())],
    }
}

/// Expand monostable only (no Runtime). Returns expanded knot count.
pub fn expand_monostable_once() -> usize {
    let p = monostable_pattern();
    let (knots, _threads, _exp) = expand_pattern("hold1", &p).unwrap();
    knots.len()
}

/// Parent: SignalIn + include monostable + SignalOut → ready-to-bind Weave.
pub fn weave_with_monostable_include() -> Weave {
    let pat = monostable_pattern();
    let (b, _) = Weave::builder("lvl")
        .knot("btn", KnotKind::signal_in())
        .unwrap();
    let (b, exp) = b.include("hold1", &pat).unwrap();
    let start = exp.port_in("start").unwrap().clone();
    let active = exp.port_out("active").unwrap().clone();
    let (b, _) = b.knot("out", KnotKind::signal_out("lamp")).unwrap();
    b.wire_ports(PortRefAuthor::new("btn", "out"), start)
        .wire_ports(active, PortRefAuthor::new("out", "in"))
        .build()
        .unwrap()
}
