//! Stable representative workloads for portable runtime performance decisions.
//!
//! Keep setup and correctness checks outside timed regions. Counters describe
//! top-level operations (loom, bind, snapshot, fingerprint, write, or host tick).

#[path = "common.rs"]
mod common;

use common::{
    activity_lanes, bind_emit_storm, chain_map, chain_not, emit_storm_weave, map_general_cycle,
    mul_div_cycle, parallel_delays, parallel_digitize, parallel_sqrt, sense_bank, sense_density,
};
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd::cookbook::tier_d::d01_shrine_chamber_weave;
use wyrd::{
    from_count, from_level, tick_once, BindOpts, CmdId, HandleError, Host, HostPathId, HostTime,
    Outbox, PortWriter, Runtime, SenseId, Signal, ONE, ZERO,
};

fn prime_numeric(rt: &mut Runtime, input: Signal, expected: Signal) -> SenseId {
    let sense = rt.sense_id("in").unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(sense, input).unwrap();
    rt.loom();
    assert_eq!(rt.outbox().signals().len(), 1);
    assert_eq!(rt.outbox().signals()[0].value, expected);
    sense
}

fn bench_numeric(bencher: Bencher, mut rt: Runtime, input: Signal, expected: Signal) {
    let sense = prime_numeric(&mut rt, input, expected);
    let mut tick = 1u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(sense, input).unwrap();
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

#[divan::bench]
fn numeric_map_identity_64(bencher: Bencher) {
    let (_weave, rt) = chain_map(64);
    bench_numeric(bencher, rt, ONE, ONE);
}

#[divan::bench]
fn numeric_map_general_cycle_64(bencher: Bencher) {
    let (_weave, rt) = map_general_cycle(32);
    bench_numeric(bencher, rt, from_count(5), from_count(5));
}

#[divan::bench]
fn numeric_mul_div_nonidentity_cycle_64(bencher: Bencher) {
    let (_weave, rt) = mul_div_cycle(32);
    bench_numeric(bencher, rt, from_count(7), from_count(7));
}

#[divan::bench]
fn numeric_digitize_midpoint_parallel_64(bencher: Bencher) {
    let (_weave, rt) = parallel_digitize(64);
    bench_numeric(bencher, rt, from_count(35), from_count(40));
}

#[divan::bench]
fn numeric_sqrt_nonperfect_parallel_64(bencher: Bencher) {
    let (_weave, rt) = parallel_sqrt(64);
    bench_numeric(bencher, rt, from_count(10), from_count(3));
}

#[divan::bench(args = [3, 4])]
fn delay_parallel_rings_32(bencher: Bencher, ticks: u16) {
    let (_weave, mut rt) = parallel_delays(32, ticks);
    assert_eq!(rt.delay_buf_len(), usize::from(ticks) * 32);
    let sense = rt.sense_id("in").unwrap();
    let warm_looms = u64::from(ticks) * 2;
    for tick in 0..warm_looms {
        let value = if tick % 2 == 0 { ONE } else { ZERO };
        rt.begin_frame(HostTime { tick });
        rt.port_writer().set_sense(sense, value).unwrap();
        rt.loom();
    }
    let expected = if (u64::from(ticks) - 1) % 2 == 0 {
        ONE
    } else {
        ZERO
    };
    assert_eq!(rt.outbox().signals()[0].value, expected);

    let mut tick = warm_looms;
    let mut value = if tick % 2 == 0 { ONE } else { ZERO };
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        rt.port_writer().set_sense(sense, value).unwrap();
        value = if value == ZERO { ONE } else { ZERO };
        rt.loom();
        black_box(rt.outbox().signals()[0].value);
    });
}

fn emit_opts(n: usize, cap: u16) -> BindOpts {
    let mut budget = common::deep_budget();
    budget.max_fan_out = (n as u16).saturating_add(4).max(16);
    budget.soft_fan_out = budget.max_fan_out;
    BindOpts {
        budget,
        max_emits_per_tick: cap,
        ..BindOpts::default()
    }
}

fn bench_emit_bind(bencher: Bencher, n: usize, cap: u16) {
    let weave = emit_storm_weave(n);
    let (_probe_weave, mut probe) = bind_emit_storm(n, cap);
    let gate = probe.sense_id("g").unwrap();
    probe.begin_frame(HostTime { tick: 0 });
    probe.port_writer().set_sense(gate, ONE).unwrap();
    probe.loom();
    assert_eq!(probe.outbox().emits().len(), n.min(usize::from(cap)));
    assert_eq!(
        probe.outbox().dropped_emits(),
        n.saturating_sub(usize::from(cap))
    );
    let opts = emit_opts(n, cap);
    bencher
        .counter(ItemsCount::new(1u64))
        .with_inputs(|| (weave.clone(), opts.clone()))
        .bench_local_values(|(weave, opts)| {
            black_box(Runtime::bind(black_box(weave), opts).unwrap());
        });
}

#[divan::bench]
fn bind_emit_32_cap_0(bencher: Bencher) {
    bench_emit_bind(bencher, 32, 0);
}

#[divan::bench]
fn bind_emit_32_cap_8(bencher: Bencher) {
    bench_emit_bind(bencher, 32, 8);
}

#[divan::bench]
fn bind_emit_32_cap_all(bencher: Bencher) {
    bench_emit_bind(bencher, 32, 32);
}

#[divan::bench]
fn bind_emit_256_cap_8(bencher: Bencher) {
    bench_emit_bind(bencher, 256, 8);
}

#[divan::bench]
fn state_fingerprint_not_64(bencher: Bencher) {
    let (_weave, rt) = chain_not(64);
    let expected = rt.runtime_fingerprint();
    assert_ne!(expected, 0);
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        black_box(rt.runtime_fingerprint());
    });
}

#[divan::bench]
fn state_snapshot_fresh_not_64(bencher: Bencher) {
    let (_weave, mut rt) = chain_not(64);
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    let fingerprint = rt.runtime_fingerprint();
    assert_eq!(rt.snapshot().fingerprint(), fingerprint);
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        black_box(rt.snapshot());
    });
}

#[divan::bench]
fn state_snapshot_reused_not_64(bencher: Bencher) {
    let (_weave, mut rt) = chain_not(64);
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    let mut state = rt.snapshot();
    rt.snapshot_into(&mut state);
    assert_eq!(state.fingerprint(), rt.runtime_fingerprint());
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.snapshot_into(&mut state);
        black_box(&state);
    });
}

#[divan::bench]
fn state_snapshot_fresh_delay_128x4(bencher: Bencher) {
    let (_weave, mut rt) = parallel_delays(128, 4);
    assert_eq!(rt.delay_buf_len(), 512);
    let sense = rt.sense_id("in").unwrap();
    rt.begin_frame(HostTime { tick: 0 });
    rt.port_writer().set_sense(sense, ONE).unwrap();
    rt.loom();
    let fingerprint = rt.runtime_fingerprint();
    assert_eq!(rt.snapshot().fingerprint(), fingerprint);
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        black_box(rt.snapshot());
    });
}

fn sense_values(n: usize, high: bool) -> Vec<Signal> {
    (0..n)
        .map(|i| match (i % 3, high) {
            (0, false) => ZERO,
            (0, true) => ONE,
            (1, false) => from_level(0.25),
            (1, true) => from_level(0.75),
            (_, false) => from_count(7),
            (_, true) => from_count(9),
        })
        .collect()
}

#[divan::bench(args = [1, 16, 64, 256])]
fn sense_checked_writes(bencher: Bencher, writes: usize) {
    let (_weave, mut rt) = sense_bank(256);
    let ids: Vec<_> = (0..writes)
        .map(|i| rt.sense_id(&format!("s{i}")).unwrap())
        .collect();
    let low = sense_values(writes, false);
    let high = sense_values(writes, true);
    let mut high_phase = false;
    bencher
        .counter(ItemsCount::new(writes as u64))
        .bench_local(|| {
            let values = if high_phase { &high } else { &low };
            high_phase = !high_phase;
            let mut writer = rt.port_writer();
            for (&id, &value) in ids.iter().zip(values) {
                writer.set_sense(id, value).unwrap();
            }
            black_box(&writer);
        });
}

#[divan::bench(args = [0, 16, 48])]
fn sense_density_full_loom_64(bencher: Bencher, senses: usize) {
    let (_weave, mut rt) = sense_density(64, senses);
    assert_eq!(rt.kind_tag_count(), 64);
    let ids: Vec<_> = (0..senses)
        .map(|i| rt.sense_id(&format!("s{i}")).unwrap())
        .collect();
    let mut tick = 0u64;
    let mut high = false;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        high = !high;
        {
            let mut writer = rt.port_writer();
            for &id in &ids {
                writer.set_sense(id, if high { ONE } else { ZERO }).unwrap();
            }
        }
        rt.loom();
        black_box((rt.kind_tag_count(), rt.outbox().signals().len()));
    });
}

#[divan::bench(args = [0, 1, 4, 16])]
fn activity_layered_lanes(bencher: Bencher, changed: usize) {
    let (_weave, mut rt) = activity_lanes(16, 3);
    let ids: Vec<_> = (0..16)
        .map(|i| rt.sense_id(&format!("s{i}")).unwrap())
        .collect();
    let mut values = [ZERO; 16];
    rt.begin_frame(HostTime { tick: 0 });
    {
        let mut writer = rt.port_writer();
        for &id in &ids {
            writer.set_sense(id, ZERO).unwrap();
        }
    }
    rt.loom();
    assert_eq!(rt.outbox().signals().len(), 16);
    assert!(rt
        .outbox()
        .signals()
        .iter()
        .all(|sample| sample.value == ONE));

    let mut tick = 1u64;
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        for value in values.iter_mut().take(changed) {
            *value = if *value == ZERO { ONE } else { ZERO };
        }
        rt.begin_frame(HostTime { tick });
        tick = tick.wrapping_add(1);
        {
            let mut writer = rt.port_writer();
            for (&id, &value) in ids.iter().zip(&values) {
                writer.set_sense(id, value).unwrap();
            }
        }
        rt.loom();
        black_box(rt.outbox().signals());
    });
}

struct ShrineHost {
    tick: u64,
    crate_on_pad: SenseId,
    player_on_pad: SenseId,
    relic_placed: SenseId,
    bridge_lever: SenseId,
    player_at_exit: SenseId,
    gate_path: HostPathId,
    bridge_path: HostPathId,
    transition: CmdId,
    last_gate: Signal,
    last_bridge: Signal,
    emits: usize,
}

impl Host for ShrineHost {
    fn time(&self) -> HostTime {
        HostTime { tick: self.tick }
    }

    fn sample_into(&mut self, ports: &mut PortWriter<'_>) -> Result<(), HandleError> {
        let phase = self.tick % 4;
        let ready = phase == 1 || phase == 2;
        let exit = phase == 2;
        for (id, value) in [
            (self.crate_on_pad, if ready { ONE } else { ZERO }),
            (self.player_on_pad, if ready { ONE } else { ZERO }),
            (self.relic_placed, if ready { ONE } else { ZERO }),
            (self.bridge_lever, if ready { ONE } else { ZERO }),
            (self.player_at_exit, if exit { ONE } else { ZERO }),
        ] {
            ports.set_sense(id, value)?;
        }
        Ok(())
    }

    fn apply(&mut self, outbox: Outbox<'_>) {
        for sample in outbox.signals() {
            if sample.path == self.gate_path {
                self.last_gate = sample.value;
            } else if sample.path == self.bridge_path {
                self.last_bridge = sample.value;
            }
        }
        self.emits += outbox
            .emits()
            .iter()
            .filter(|emit| emit.cmd == self.transition)
            .count();
        self.tick = self.tick.wrapping_add(1);
    }
}

fn shrine_host(rt: &Runtime) -> ShrineHost {
    ShrineHost {
        tick: 0,
        crate_on_pad: rt.sense_id("crate_on_sun_pad").unwrap(),
        player_on_pad: rt.sense_id("player_on_moon_pad").unwrap(),
        relic_placed: rt.sense_id("relic_placed").unwrap(),
        bridge_lever: rt.sense_id("bridge_lever").unwrap(),
        player_at_exit: rt.sense_id("player_at_exit").unwrap(),
        gate_path: rt.path_id("shrine.gate.open").unwrap(),
        bridge_path: rt.path_id("shrine.bridge.target").unwrap(),
        transition: rt.cmd_id("world.request_transition").unwrap(),
        last_gate: ZERO,
        last_bridge: ZERO,
        emits: 0,
    }
}

#[divan::bench]
fn tier_d_sample_loom_apply(bencher: Bencher) {
    let weave = d01_shrine_chamber_weave().unwrap();
    let mut rt = Runtime::bind(weave, BindOpts::default()).unwrap();
    let mut host = shrine_host(&rt);
    for _ in 0..4 {
        tick_once(&mut host, &mut rt).unwrap();
    }
    assert_eq!(host.last_gate, ONE);
    assert_eq!(host.last_bridge, ZERO);
    assert_eq!(host.emits, 1);

    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        tick_once(&mut host, &mut rt).unwrap();
        black_box((host.tick, host.last_gate, host.last_bridge, host.emits));
    });
}

fn main() {
    divan::main();
}
