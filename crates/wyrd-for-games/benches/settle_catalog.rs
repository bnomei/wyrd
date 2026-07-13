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
use wyrd::{from_count, from_level, HostTime, ONE, ZERO};

#[divan::bench]
fn settle_map_digitize_high_endpoint(bencher: Bencher) {
    let (_weave, mut rt) = map_digitize_chain();
    let id = rt.sense_id("in").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

#[divan::bench]
fn settle_calc_abs(bencher: Bencher) {
    let (_weave, mut rt) = calc_abs_chain();
    let a = rt.sense_id("a").unwrap();
    let b = rt.sense_id("b").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        {
            let mut w = rt.port_writer();
            w.set_sense(a, from_level(-0.75)).unwrap();
            w.set_sense(b, from_level(0.25)).unwrap();
        }
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

#[divan::bench]
fn settle_threshold(bencher: Bencher) {
    let (_weave, mut rt) = threshold_simple();
    let id = rt.sense_id("in").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(3u64)).bench_local(|| {
        let mut observed = [ZERO; 3];
        for (slot, value) in observed.iter_mut().zip([ZERO, ONE, ZERO]) {
            rt.begin_frame(HostTime { tick });
            tick = tick.wrapping_add(1);
            rt.port_writer().set_sense(id, value).unwrap();
            rt.loom();
            *slot = rt.outbox().signals()[0].value;
        }
        black_box(observed);
    });
}

/// One Constant fan-out to `n` Not → SignalOut (wide, shallow).
#[divan::bench(args = [8, 32])]
fn settle_fanout_nots(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = fanout_nots(n);
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.loom();
        black_box(rt.outbox().signals());
    });
}

/// SignalIn → Map × n → Out. Args = number of Map knots.
#[divan::bench(args = [16, 64])]
fn settle_map_identity_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_map(n);
    let id = rt.sense_id("in").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// SignalIn → Digitize(steps=8) × n → Out.
#[divan::bench(args = [16, 64])]
fn settle_digitize_high_endpoint_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_digitize(n, 8);
    let id = rt.sense_id("in").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        // Level ONE so f32 and i32 both sit at the high end of ZERO..ONE
        // (from_count(1) is ~0 on Q16, not equivalent to f32 1.0).
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// SignalIn → Calc(Mul) × n with level ONE on `b` (i32 Q-mul safe).
#[divan::bench(args = [16, 64])]
fn settle_calc_mul_by_one_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_calc_mul(n);
    let id = rt.sense_id("in").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// SignalIn → Sqrt × n → Out (positive input).
#[divan::bench(args = [16, 64])]
fn settle_sqrt_one_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_sqrt(n);
    let id = rt.sense_id("in").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// P1: SignalIn → Calc(Div) × n with ONE divisor (dual-path non-zero).
#[divan::bench(args = [16, 64])]
fn settle_calc_div_by_one_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_calc_div(n);
    let id = rt.sense_id("in").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// Rising / Falling / Change pack over a complete low-high-low cycle.
#[divan::bench]
fn settle_edges_pack(bencher: Bencher) {
    let (_weave, mut rt) = edges_pack();
    let id = rt.sense_id("in").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(3u64)).bench_local(|| {
        for value in [ZERO, ONE, ZERO] {
            rt.begin_frame(HostTime { tick });
            tick = tick.wrapping_add(1);
            rt.port_writer().set_sense(id, value).unwrap();
            rt.loom();
            black_box(rt.outbox().signals());
        }
    });
}

/// Or / Xor / Select pack.
#[divan::bench]
fn settle_logic_pack(bencher: Bencher) {
    let (_weave, mut rt) = logic_pack();
    let a = rt.sense_id("a").unwrap();
    let b = rt.sense_id("b").unwrap();
    let sel = rt.sense_id("sel").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(4u64)).bench_local(|| {
        for (av, bv, sv) in [
            (ZERO, ZERO, ZERO),
            (ONE, ZERO, ZERO),
            (ONE, ONE, ONE),
            (ZERO, ONE, ONE),
        ] {
            rt.begin_frame(HostTime { tick });
            tick = tick.wrapping_add(1);
            {
                let mut w = rt.port_writer();
                w.set_sense(a, av).unwrap();
                w.set_sense(b, bv).unwrap();
                w.set_sense(sel, sv).unwrap();
            }
            rt.loom();
            black_box(rt.outbox().signals());
        }
    });
}

/// Neg → Clamp layers × n.
#[divan::bench(args = [16, 64])]
fn settle_clamp_neg_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_clamp_neg(n);
    let id = rt.sense_id("in").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(id, from_count(3)).unwrap();
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// Compare(Gte, rhs_const=0) × n.
#[divan::bench(args = [16, 64])]
fn settle_compare_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_compare(n);
    let id = rt.sense_id("in").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(id, from_count(1)).unwrap();
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// First loom of a freshly bound OnStart graph.
#[divan::bench]
fn settle_onstart_first_loom(bencher: Bencher) {
    let (_weave, mut probe) = onstart_out();
    probe.begin_frame(HostTime { tick: 0 });
    probe.loom();
    assert_eq!(probe.outbox().signals()[0].value, ONE);
    bencher
        .counter(ItemsCount::new(1u64))
        .with_inputs(|| onstart_out().1)
        .bench_local_refs(|rt| {
            rt.begin_frame(HostTime { tick: 0 });
            rt.loom();
            black_box(rt.outbox().signals()[0].value);
        });
}

/// Steady-state loom after the OnStart pulse has been consumed.
#[divan::bench]
fn settle_onstart_steady_state(bencher: Bencher) {
    let (_weave, mut rt) = onstart_out();
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    assert_eq!(rt.outbox().signals()[0].value, ONE);
    let mut tick = 1u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

fn main() {
    divan::main();
}
