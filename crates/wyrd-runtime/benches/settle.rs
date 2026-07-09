//! Divan settle benches (step 5.1).

use divan::{black_box, Bencher};
use wyrd_core::{HostTime, KnotKind, ONE};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

/// Constant → Not × n → SignalOut
fn chain_not(n: usize) -> (Weave, Runtime) {
    let (mut b, _) = Weave::builder("chain")
        .knot("c0", KnotKind::constant(ONE))
        .unwrap();
    let mut prev = "c0".to_string();
    for i in 0..n {
        let id = format!("n{i}");
        let (b2, _) = b.knot(&id, KnotKind::not()).unwrap();
        b = b2.wire_named(&prev, "out", &id, "in");
        prev = id;
    }
    let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let weave = b.wire_named(&prev, "out", "out", "in").build().unwrap();
    let rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    (weave, rt)
}

// Budget hard max knots is 256 (constant + n Nots + out).
#[divan::bench(args = [16, 64, 128])]
fn settle_not_chain(bencher: Bencher, n: usize) {
    let (weave, mut rt) = chain_not(n);
    bencher.bench_local(|| {
        rt.begin_frame(HostTime { tick: 0 });
        rt.loom(black_box(&weave)).unwrap();
        black_box(rt.outbox().signals().len());
    });
}

#[divan::bench]
fn settle_and_door(bencher: Bencher) {
    let (b, pa) = Weave::builder("door")
        .knot("plate_a", KnotKind::signal_in())
        .unwrap();
    let (b, pb) = b.knot("plate_b", KnotKind::signal_in()).unwrap();
    let (b, _) = b.and2("both", pa, pb).unwrap();
    let (b, _) = b.knot("door", KnotKind::signal_out("door.open")).unwrap();
    let weave = b.wire_named("both", "out", "door", "in").build().unwrap();
    let mut rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let a = rt.sense_id("plate_a").unwrap();
    let b_id = rt.sense_id("plate_b").unwrap();
    bencher.bench_local(|| {
        rt.begin_frame(HostTime { tick: 1 });
        {
            let mut w = rt.port_writer();
            w.set_sense(a, ONE);
            w.set_sense(b_id, ONE);
        }
        rt.loom(black_box(&weave)).unwrap();
        black_box(rt.outbox().signals().len());
    });
}

fn main() {
    divan::main();
}
