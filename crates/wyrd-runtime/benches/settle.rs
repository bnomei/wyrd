//! Divan settle benches (step 5.1).

use divan::{black_box, Bencher};
use wyrd_core::{HostTime, KnotKind, ONE};
use wyrd_graph::Weave;
use wyrd_runtime::{BindOpts, Runtime};

fn chain_not(n: usize) -> (Weave, Runtime) {
    let mut b = Weave::builder("chain");
    let (b2, mut prev) = b.knot("c0", KnotKind::constant(ONE)).unwrap();
    b = b2;
    for i in 0..n {
        let id = format!("n{i}");
        let (b2, kid) = b.knot(&id, KnotKind::not()).unwrap();
        b = b2;
        let prev_name = if i == 0 {
            "c0".to_string()
        } else {
            format!("n{}", i - 1)
        };
        let port = if i == 0 { "out" } else { "out" };
        // Not: in/out — Constant out → first not in
        let from_port = if i == 0 { "out" } else { "out" };
        b = b.wire_named(&prev_name, from_port, &id, "in");
        let _ = (kid, port);
        prev = kid;
    }
    let (b2, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
    let last = if n == 0 {
        "c0".to_string()
    } else {
        format!("n{}", n - 1)
    };
    let weave = b2
        .wire_named(&last, "out", "out", "in")
        .build()
        .unwrap();
    let rt = Runtime::bind(&weave, BindOpts::default()).unwrap();
    let _ = prev;
    (weave, rt)
}

#[divan::bench(args = [16, 64, 256])]
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
