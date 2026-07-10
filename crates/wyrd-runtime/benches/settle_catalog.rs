//! Catalog math settle: Map/Digitize, Calc/Abs, Threshold, fan-out.

#[path = "common.rs"]
mod common;

use common::{calc_abs_chain, fanout_nots, map_digitize_chain, threshold_simple};
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

fn main() {
    divan::main();
}
