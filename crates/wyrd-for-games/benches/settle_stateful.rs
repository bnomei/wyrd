//! Stateful settle: Delay, Random; P0 delay chain; P1 kit + emit storm.

#[path = "common.rs"]
mod common;

use common::{chain_delays, delay_chain, emit_storm, random_gated, stateful_kit};
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd::{HostTime, ONE, ZERO};

/// Overhead probe: single Delay, vary ring length (fixed 3 knots — often flat).
#[divan::bench(args = [1, 8, 32])]
fn settle_delay(bencher: Bencher, ticks: u16) {
    let (_weave, mut rt) = delay_chain(ticks);
    let id = rt.sense_id("in").unwrap();
    let mut on = true;
    let mut tick = 0u64;
    // Fill and wrap the ring before timing so every sample measures steady traffic.
    for _ in 0..(usize::from(ticks) * 2 + 2) {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer()
            .set_sense(id, if on { ONE } else { ZERO })
            .unwrap();
        on = !on;
        rt.loom();
    }
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer()
            .set_sense(id, if on { ONE } else { ZERO })
            .unwrap();
        on = !on;
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// P0: n × Delay(ticks=4) chain — ring traffic scales with n.
#[divan::bench(args = [8, 32])]
fn settle_delay_chain(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = chain_delays(n, 4);
    let id = rt.sense_id("in").unwrap();
    let mut on = true;
    let mut tick = 0u64;
    // Flush the complete delay path before timing so calibration starts steady.
    for _ in 0..(n * 4 + 2) {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer()
            .set_sense(id, if on { ONE } else { ZERO })
            .unwrap();
        on = !on;
        rt.loom();
    }
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer()
            .set_sense(id, if on { ONE } else { ZERO })
            .unwrap();
        on = !on;
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// Rising gate each tick so Random samples every settle.
/// **Two looms per sample** (fall then rise), counted as two timed operations.
#[divan::bench]
fn settle_random_gated(bencher: Bencher) {
    let (_weave, mut rt) = random_gated();
    let g = rt.sense_id("g").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(2u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(g, ZERO).unwrap();
        rt.loom();

        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(g, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

/// Counter / Flag / PulseHold / FedCountdown in a complete seven-loom cycle
/// that returns timers and counter to idle.
#[divan::bench]
fn settle_stateful_kit_cycle(bencher: Bencher) {
    let (_weave, mut rt) = stateful_kit();
    let start = rt.sense_id("start").unwrap();
    let feed = rt.sense_id("feed").unwrap();
    let reset = rt.sense_id("reset").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(7u64)).bench_local(|| {
        for (sv, fv, rv) in [
            (ZERO, ZERO, ONE),
            (ONE, ZERO, ZERO),
            (ONE, ONE, ZERO),
            (ZERO, ONE, ZERO),
            (ZERO, ZERO, ZERO),
            (ZERO, ZERO, ZERO),
            (ZERO, ZERO, ONE),
        ] {
            rt.begin_frame(HostTime { tick });
            tick = tick.wrapping_add(1);
            {
                let mut w = rt.port_writer();
                w.set_sense(start, sv).unwrap();
                w.set_sense(feed, fv).unwrap();
                w.set_sense(reset, rv).unwrap();
            }
            rt.loom();
            black_box(rt.outbox().signals());
        }
    });
}

/// Shared gate → n EmitCommands; **forced rising edge every sample** (2 looms: low then high).
/// ItemsCount = two loom operations (fall then rise).
#[divan::bench(args = [8, 32])]
fn settle_emit_storm(bencher: Bencher, n: usize) {
    let (_weave, mut rt) = emit_storm(n);
    let g = rt.sense_id("g").unwrap();
    let mut tick = 0u64;
    bencher.counter(ItemsCount::new(2u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(g, ZERO).unwrap();
        rt.loom();

        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(g, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().emits());
    });
}

fn main() {
    divan::main();
}
