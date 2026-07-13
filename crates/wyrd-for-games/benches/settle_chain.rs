//! Depth / branch settle baselines (Not chains, And door, host tick).

#[path = "common.rs"]
mod common;

use common::{and_door, chain_not};
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd::{tick_once, NullHost};
use wyrd::{HostTime, ONE};

/// `n` = Not knots (total knots ≈ n + 2).
#[divan::bench(args = [16, 64, 128])]
fn settle_not_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_not(n);
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

#[divan::bench]
fn settle_and_door(bencher: Bencher) {
    let (_weave, mut rt) = and_door();
    let a = rt.sense_id("plate_a").unwrap();
    let b_id = rt.sense_id("plate_b").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        {
            let mut w = rt.port_writer();
            w.set_sense(a, ONE).unwrap();
            w.set_sense(b_id, ONE).unwrap();
        }
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// Host `tick_once` (NullHost) on a Not chain.
#[divan::bench(args = [16, 64])]
fn tick_once_not_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_not(n);
    let mut host = NullHost::default();
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        tick_once(&mut host, &mut rt).unwrap();
        black_box(rt.outbox().signals()[0].value);
    });
}

fn main() {
    divan::main();
}
