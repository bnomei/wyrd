//! Bind / load-path cost (validate + topo + buffers) — not per-frame.

#[path = "common.rs"]
mod common;

use common::{bind_deep, chain_not_weave, small_authored_weave};
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd_runtime::{BindOpts, Runtime};

/// Bind a small Map/Not weave (asset-style load unit).
#[divan::bench]
fn bind_small(bencher: Bencher) {
    let weave = small_authored_weave();
    let knots = weave.knots.len() as u64;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            let rt = Runtime::bind(black_box(&weave), BindOpts::default()).unwrap();
            black_box(rt);
        });
}

/// Bind a deep Not chain (validate depth + topo).
#[divan::bench(args = [16, 64, 128])]
fn bind_not_chain(bencher: Bencher, n: usize) {
    let weave = chain_not_weave(n);
    let knots = weave.knots.len() as u64;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            let rt = bind_deep(black_box(&weave));
            black_box(rt);
        });
}

fn main() {
    divan::main();
}
