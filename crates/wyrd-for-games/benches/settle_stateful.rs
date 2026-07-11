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
    let (weave, mut rt) = delay_chain(ticks);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    let mut on = true;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer()
            .set_sense(id, if on { ONE } else { ZERO })
            .unwrap();
        on = !on;
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// P0: n × Delay(ticks=4) chain — ring traffic scales with n.
#[divan::bench(args = [8, 32])]
fn settle_delay_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_delays(n, 4);
    let id = rt.sense_id("in").unwrap();
    let knots = weave.knots().len() as u64;
    let mut on = true;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer()
            .set_sense(id, if on { ONE } else { ZERO })
            .unwrap();
        on = !on;
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// Rising gate each tick so Random samples every settle.
/// **Two looms per sample** (fall then rise) — items/s not 1:1 with other rows.
#[divan::bench]
fn settle_random_gated(bencher: Bencher) {
    let (weave, mut rt) = random_gated();
    let g = rt.sense_id("g").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(g, ZERO).unwrap();
        rt.loom();

        rt.begin_frame(HostTime { tick: 1 });
        rt.port_writer().set_sense(g, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().signals().len());
    });
}

/// Counter / Flag / PulseHold / FedCountdown with a 4-phase sense script.
/// One loom per sample; phase advances so edges fire over iterations.
#[divan::bench]
fn settle_stateful_kit(bencher: Bencher) {
    let (weave, mut rt) = stateful_kit();
    let start = rt.sense_id("start").unwrap();
    let feed = rt.sense_id("feed").unwrap();
    let knots = weave.knots().len() as u64;
    // 0: idle, 1: start rise, 2: hold start+feed, 3: release start keep feed
    let mut phase = 0u8;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        let (sv, fv) = match phase % 4 {
            0 => (ZERO, ZERO),
            1 => (ONE, ZERO),
            2 => (ONE, ONE),
            _ => (ZERO, ONE),
        };
        phase = phase.wrapping_add(1);
        rt.begin_frame(HostTime { tick: 0 });
        {
            let mut w = rt.port_writer();
            w.set_sense(start, sv).unwrap();
            w.set_sense(feed, fv).unwrap();
        }
        rt.loom();
        black_box((rt.outbox().signals().len(), rt.outbox().emits().len()));
    });
}

/// Shared gate → n EmitCommands; **forced rising edge every sample** (2 looms: low then high).
/// ItemsCount = **knots** (gate + n emits) for suite comparability.
#[divan::bench(args = [8, 32])]
fn settle_emit_storm(bencher: Bencher, n: usize) {
    let (weave, mut rt) = emit_storm(n);
    let g = rt.sense_id("g").unwrap();
    let knots = weave.knots().len() as u64;
    bencher.counter(ItemsCount::new(knots)).bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.port_writer().set_sense(g, ZERO).unwrap();
        rt.loom();

        rt.begin_frame(HostTime { tick: 1 });
        rt.port_writer().set_sense(g, ONE).unwrap();
        rt.loom();
        black_box(rt.outbox().emits().len());
    });
}

fn main() {
    divan::main();
}
