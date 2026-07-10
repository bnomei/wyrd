//! Catalog math settle: micro overhead probes + scaled chains (P0).

#[path = "common.rs"]
mod common;

use common::{
    calc_abs_chain, chain_calc_div, chain_calc_mul, chain_digitize, chain_map, chain_sqrt,
    fanout_nots, map_digitize_chain, threshold_simple,
};
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd_core::{from_count, HostTime, ONE, ZERO};

#[divan::bench]
fn settle_map_digitize(bencher: Bencher) {
    let (weave, mut rt) = map_digitize_chain();
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots.len() as u64;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            rt.begin_frame(HostTime { tick: 0 });
            rt.port_writer().set_sense(id, from_count(1));
            rt.loom(black_box(&weave)).unwrap();
            black_box(rt.outbox().signals().len());
        });
}

#[divan::bench]
fn settle_calc_abs(bencher: Bencher) {
    let (weave, mut rt) = calc_abs_chain();
    let a = rt.sense_id("a").unwrap();
    let b = rt.sense_id("b").unwrap();
    let knots = weave.knots.len() as u64;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            rt.begin_frame(HostTime { tick: 0 });
            {
                let mut w = rt.port_writer();
                w.set_sense(a, from_count(-3));
                w.set_sense(b, ONE);
            }
            rt.loom(black_box(&weave)).unwrap();
            black_box(rt.outbox().signals().len());
        });
}

#[divan::bench]
fn settle_threshold(bencher: Bencher) {
    let (weave, mut rt) = threshold_simple();
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots.len() as u64;
    // Alternate high/low so hysteresis-free threshold does real work.
    let mut hi = true;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            rt.begin_frame(HostTime { tick: 0 });
            rt.port_writer()
                .set_sense(id, if hi { ONE } else { ZERO });
            hi = !hi;
            rt.loom(black_box(&weave)).unwrap();
            black_box(rt.outbox().signals().len());
        });
}

/// One Constant fan-out to `n` Not → SignalOut (wide, shallow).
#[divan::bench(args = [8, 32])]
fn settle_fanout_nots(bencher: Bencher, n: usize) {
    let (weave, mut rt) = fanout_nots(n);
    let knots = weave.knots.len() as u64;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            rt.begin_frame(HostTime { tick: 0 });
            rt.loom(black_box(&weave)).unwrap();
            black_box(rt.outbox().signals().len());
        });
}

// --- P0: scaled chains (amortize fixed loom tax) ---

/// SignalIn → Map × n → Out. Args = number of Map knots.
#[divan::bench(args = [16, 64])]
fn settle_map_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_map(n);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots.len() as u64;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            rt.begin_frame(HostTime { tick: 0 });
            rt.port_writer().set_sense(id, ONE);
            rt.loom(black_box(&weave)).unwrap();
            black_box(rt.outbox().signals().len());
        });
}

/// SignalIn → Digitize(steps=8) × n → Out.
#[divan::bench(args = [16, 64])]
fn settle_digitize_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_digitize(n, 8);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots.len() as u64;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            rt.begin_frame(HostTime { tick: 0 });
            // Level ONE so f32 and i32 both sit at the high end of ZERO..ONE
            // (from_count(1) is ~0 on Q16, not equivalent to f32 1.0).
            rt.port_writer().set_sense(id, ONE);
            rt.loom(black_box(&weave)).unwrap();
            black_box(rt.outbox().signals().len());
        });
}

/// SignalIn → Calc(Mul) × n with level ONE on `b` (i32 Q-mul safe).
#[divan::bench(args = [16, 64])]
fn settle_calc_mul_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_calc_mul(n);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots.len() as u64;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            rt.begin_frame(HostTime { tick: 0 });
            rt.port_writer().set_sense(id, ONE);
            rt.loom(black_box(&weave)).unwrap();
            black_box(rt.outbox().signals().len());
        });
}

/// SignalIn → Sqrt × n → Out (positive input).
#[divan::bench(args = [16, 64])]
fn settle_sqrt_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_sqrt(n);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots.len() as u64;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            rt.begin_frame(HostTime { tick: 0 });
            // Positive level: f32 sqrtf; i32 integer isqrt on bits.
            rt.port_writer().set_sense(id, ONE);
            rt.loom(black_box(&weave)).unwrap();
            black_box(rt.outbox().signals().len());
        });
}

/// P1: SignalIn → Calc(Div) × n with ONE divisor (dual-path non-zero).
#[divan::bench(args = [16, 64])]
fn settle_calc_div_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_calc_div(n);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots.len() as u64;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            rt.begin_frame(HostTime { tick: 0 });
            rt.port_writer().set_sense(id, ONE);
            rt.loom(black_box(&weave)).unwrap();
            black_box(rt.outbox().signals().len());
        });
}

fn main() {
    divan::main();
}
