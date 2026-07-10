//! Stateful settle: Delay rings, gated Random.

#[path = "common.rs"]
mod common;

use common::{delay_chain, random_gated};
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd_core::{HostTime, ONE, ZERO};

#[divan::bench(args = [1, 8, 32])]
fn settle_delay(bencher: Bencher, ticks: u16) {
    let (weave, mut rt) = delay_chain(ticks);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots.len() as u64;
    let mut on = true;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            rt.begin_frame(HostTime { tick: 0 });
            rt.port_writer()
                .set_sense(id, if on { ONE } else { ZERO });
            on = !on;
            rt.loom(black_box(&weave)).unwrap();
            black_box(rt.outbox().signals().len());
        });
}

/// Rising gate each tick so Random samples every settle.
#[divan::bench]
fn settle_random_gated(bencher: Bencher) {
    let (weave, mut rt) = random_gated();
    let g = rt.sense_id("g").unwrap();
    let knots = weave.knots.len() as u64;
    let mut phase = false;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            // 0 → 1 rising edge each iteration.
            rt.begin_frame(HostTime { tick: 0 });
            rt.port_writer().set_sense(g, ZERO);
            rt.loom(black_box(&weave)).unwrap();

            phase = !phase;
            rt.begin_frame(HostTime { tick: 1 });
            rt.port_writer().set_sense(g, ONE);
            rt.loom(black_box(&weave)).unwrap();
            black_box(rt.outbox().signals().len());
        });
}

fn main() {
    divan::main();
}
