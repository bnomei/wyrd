//! Isolation sub-benches: separate clear/gather/eval assumptions at scaled N.
//!
//! Use longer Divan weight for decisions:
//!   cargo bench -p wyrd-runtime --bench settle_iso -- --sample-count 300 --min-time 1

#[path = "common.rs"]
mod common;

use common::{
    chain_calc_div, chain_clamp_neg, chain_compare, chain_delays, chain_digitize, chain_map,
    chain_not, chain_sqrt, fanout_nots,
};
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd_core::{HostTime, ONE};

/// Structural baseline: deep Not chain (gather + Not eval + clear).
#[divan::bench(args = [64, 128])]
fn iso_struct_not_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_not(n);
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// Gather / outbox pressure: wide fan-out of Nots (many SignalOut).
#[divan::bench(args = [32, 64])]
fn iso_gather_fanout(bencher: Bencher, n: usize) {
    let (weave, mut rt) = fanout_nots(n);
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// Eval-heavy: Digitize layer stack (amortized).
#[divan::bench(args = [64])]
fn iso_eval_digitize_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_digitize(n, 8);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// Eval-heavy: Sqrt layer stack (amortized).
#[divan::bench(args = [64])]
fn iso_eval_sqrt_chain(bencher: Bencher, n: usize) {
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

/// Map stack as lighter eval control vs Digitize/Sqrt.
#[divan::bench(args = [64])]
fn iso_eval_map_chain(bencher: Bencher, n: usize) {
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

/// Rank1: Calc Div chain (const ONE divisor — stresses Q-div / identity path).
#[divan::bench(args = [64])]
fn iso_eval_div_chain(bencher: Bencher, n: usize) {
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

/// Rank4: Clamp+Neg layers.
#[divan::bench(args = [64])]
fn iso_eval_clamp_neg_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_clamp_neg(n);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// Rank5: Compare chain (const rhs).
#[divan::bench(args = [64])]
fn iso_eval_compare_chain(bencher: Bencher, n: usize) {
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

/// Rank7: Delay chain (ring traffic).
#[divan::bench(args = [32])]
fn iso_eval_delay_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_delays(n, 4); // ticks=4 (power-of-two ring)
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(id, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

fn main() {
    divan::main();
}
