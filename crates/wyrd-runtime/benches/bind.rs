//! Bind / load-path cost (validate + topo + buffers) — not per-frame.

#[path = "common.rs"]
mod common;

use common::{
    bind_deep, chain_not_weave, expand_monostable_once, monostable_pattern, small_authored_weave,
    weave_with_monostable_include,
};
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd_graph::expand_pattern;
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

// --- P3: pattern expand + include + bind ---

/// Expand monostable pattern only (no Runtime).
#[divan::bench]
fn expand_pattern_monostable(bencher: Bencher) {
    let p = monostable_pattern();
    let n = expand_monostable_once() as u64;
    bencher
        .counter(ItemsCount::new(n.max(1)))
        .bench_local(|| {
            let (knots, threads, exp) = expand_pattern("hold1", black_box(&p)).unwrap();
            black_box((knots.len(), threads.len(), exp.port_in("start").is_ok()));
        });
}

/// Bind a weave that already included monostable (bind only; weave built outside).
#[divan::bench]
fn bind_after_pattern_include(bencher: Bencher) {
    let weave = weave_with_monostable_include();
    let knots = weave.knots.len() as u64;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            let rt = Runtime::bind(black_box(&weave), BindOpts::default()).unwrap();
            black_box(rt);
        });
}

/// Full include+build+bind each sample (authoring reload cost).
#[divan::bench]
fn include_build_bind_monostable(bencher: Bencher) {
    let knots = weave_with_monostable_include().knots.len() as u64;
    bencher
        .counter(ItemsCount::new(knots))
        .bench_local(|| {
            let weave = weave_with_monostable_include();
            let rt = Runtime::bind(black_box(&weave), BindOpts::default()).unwrap();
            black_box(rt);
        });
}

fn main() {
    divan::main();
}
