//! Catalog math settle: micro overhead probes + scaled chains (P0).

#[path = "common.rs"]
mod common;

use common::{
    calc_abs_chain, chain_calc_div, chain_calc_mul, chain_clamp_neg, chain_compare, chain_digitize,
    chain_map, chain_sqrt, edges_pack, fanout_nots, logic_pack, map_digitize_chain, onstart_out,
    threshold_simple,
};
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd::{from_count, HostTime, ONE, ZERO};

#[divan::bench]
fn settle_map_digitize(bencher: Bencher) {
    let (weave, mut rt) = map_digitize_chain();
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(id, from_count(1)).unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

#[divan::bench]
fn settle_calc_abs(bencher: Bencher) {
    let (weave, mut rt) = calc_abs_chain();
    let a = rt.sense_id("a").unwrap();
    let b = rt.sense_id("b").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        {
            let mut w = rt.port_writer();
            w.set_sense(a, from_count(-3)).unwrap();
            w.set_sense(b, ONE).unwrap();
        }
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

#[divan::bench]
fn settle_threshold(bencher: Bencher) {
    let (weave, mut rt) = threshold_simple();
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    // Alternate high/low so hysteresis-free threshold does real work.
    let mut hi = true;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer()
            .set_sense(id, if hi { ONE } else { ZERO })
            .unwrap();
        hi = !hi;
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// One Constant fan-out to `n` Not → SignalOut (wide, shallow).
#[divan::bench(args = [8, 32])]
fn settle_fanout_nots(bencher: Bencher, n: usize) {
    let (weave, mut rt) = fanout_nots(n);
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// SignalIn → Map × n → Out. Args = number of Map knots.
#[divan::bench(args = [16, 64])]
fn settle_map_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_map(n);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// SignalIn → Digitize(steps=8) × n → Out.
#[divan::bench(args = [16, 64])]
fn settle_digitize_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_digitize(n, 8);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        // Level ONE so f32 and i32 both sit at the high end of ZERO..ONE
        // (from_count(1) is ~0 on Q16, not equivalent to f32 1.0).
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// SignalIn → Calc(Mul) × n with level ONE on `b` (i32 Q-mul safe).
#[divan::bench(args = [16, 64])]
fn settle_calc_mul_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_calc_mul(n);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// SignalIn → Sqrt × n → Out (positive input).
#[divan::bench(args = [16, 64])]
fn settle_sqrt_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_sqrt(n);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// P1: SignalIn → Calc(Div) × n with ONE divisor (dual-path non-zero).
#[divan::bench(args = [16, 64])]
fn settle_calc_div_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_calc_div(n);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// Rising / Falling / Change pack; toggle input each sample.
#[divan::bench]
fn settle_edges_pack(bencher: Bencher) {
    let (weave, mut rt) = edges_pack();
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    let mut hi = false;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        hi = !hi;
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer()
            .set_sense(id, if hi { ONE } else { ZERO })
            .unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// Or / Xor / Select pack.
#[divan::bench]
fn settle_logic_pack(bencher: Bencher) {
    let (weave, mut rt) = logic_pack();
    let a = rt.sense_id("a").unwrap();
    let b = rt.sense_id("b").unwrap();
    let sel = rt.sense_id("sel").unwrap();
    let knots = weave.knots().len() as u64;
    let mut phase = 0u8;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        phase = phase.wrapping_add(1);
        let (av, bv, sv) = match phase % 4 {
            0 => (ZERO, ZERO, ZERO),
            1 => (ONE, ZERO, ZERO),
            2 => (ONE, ONE, ONE),
            _ => (ZERO, ONE, ONE),
        };
        rt.begin_frame(HostTime { tick: 0 });
        {
            let mut w = rt.port_writer();
            w.set_sense(a, av).unwrap();
            w.set_sense(b, bv).unwrap();
            w.set_sense(sel, sv).unwrap();
        }
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// Neg → Clamp layers × n.
#[divan::bench(args = [16, 64])]
fn settle_clamp_neg_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_clamp_neg(n);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(id, from_count(3)).unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// Compare(Gte, rhs_const=0) × n.
#[divan::bench(args = [16, 64])]
fn settle_compare_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_compare(n);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// OnStart pulse (only first frame differs — overhead probe).
#[divan::bench]
fn settle_onstart(bencher: Bencher) {
    let (weave, mut rt) = onstart_out();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        // Fresh runtime each sample would dominate; re-use and accept
        // subsequent frames are ZERO (still exercises OnStart arm + out).
        rt.begin_frame(HostTime { tick: 0 });
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

fn main() {
    divan::main();
}
