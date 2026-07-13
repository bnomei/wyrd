//! Isolation sub-benches: separate clear/gather/eval assumptions at scaled N.
//!
//! Use longer Divan weight for decisions:
//!   cargo bench -p wyrd-for-games --bench settle_iso -- --sample-count 300 --min-time 1

#[path = "common.rs"]
mod common;

#[cfg(feature = "signal-i32")]
use common::chain_map_ranges;
use common::{
    chain_calc_div, chain_clamp_neg, chain_compare, chain_delays, chain_digitize, chain_map,
    chain_not, chain_sqrt, fanout_nots,
};
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd::{from_count, HostTime, ONE};
#[cfg(feature = "signal-i32")]
use wyrd::{SignalDomain, ZERO};

/// Structural baseline: deep Not chain (gather + Not eval + clear).
#[divan::bench(args = [64, 128])]
fn iso_struct_not_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_not(n);
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// Gather / outbox pressure: wide fan-out of Nots (many SignalOut).
#[divan::bench(args = [32, 64])]
fn iso_gather_fanout(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = fanout_nots(n);
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.loom();
        black_box(rt.outbox().signals());
    });
}

/// Eval-heavy: Digitize layer stack (amortized).
#[divan::bench(args = [64])]
fn iso_eval_digitize_high_endpoint_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_digitize(n, 8);
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

/// Eval-heavy: Sqrt layer stack (amortized).
#[divan::bench(args = [64])]
fn iso_eval_sqrt_one_chain(bencher: Bencher, n: usize) {
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

/// Map stack as lighter eval control vs Digitize/Sqrt.
#[divan::bench(args = [64])]
fn iso_eval_map_identity_chain(bencher: Bencher, n: usize) {
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

/// i32-only power-of-two reduced denominator: shift instead of division.
#[cfg(feature = "signal-i32")]
#[divan::bench(args = [64])]
fn iso_eval_map_shift_high_endpoint_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_map_ranges(n, SignalDomain::Level, -ONE, ONE, ZERO, ONE);
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

/// i32-only non-power-of-two reduced denominator: exact general division.
#[cfg(feature = "signal-i32")]
#[divan::bench(args = [64])]
fn iso_eval_map_general_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_map_ranges(
        n,
        SignalDomain::Count,
        from_count(0),
        from_count(10),
        from_count(-37),
        from_count(997),
    );
    let id = rt.sense_id("in").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(id, from_count(5)).unwrap();
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// Rank1: Calc Div chain (const ONE divisor — stresses Q-div / identity path).
#[divan::bench(args = [64])]
fn iso_eval_div_by_one_chain(bencher: Bencher, n: usize) {
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

/// Rank4: Clamp+Neg layers.
#[divan::bench(args = [64])]
fn iso_eval_clamp_neg_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_clamp_neg(n);
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

/// Rank5: Compare chain (const rhs).
#[divan::bench(args = [64])]
fn iso_eval_compare_chain(bencher: Bencher, n: usize) {
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

/// Rank7: Delay chain (ring traffic).
#[divan::bench(args = [32])]
fn iso_eval_delay_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_delays(n, 4); // ticks=4 (power-of-two ring)
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

fn main() {
    divan::main();
}
