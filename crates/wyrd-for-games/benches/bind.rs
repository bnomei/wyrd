//! Bind / load-path cost (validate + topo + buffers) — not per-frame.

#[path = "common.rs"]
mod common;

use common::{
    bind_deep_owned, chain_not_weave, expand_monostable_once, small_authored_weave,
    weave_with_monostable_include,
};
use divan::counter::ItemsCount;
use divan::{black_box, Bencher};
use wyrd::{BindOpts, Runtime};

/// Clone a small authored weave without validation or binding.
#[divan::bench]
fn clone_small_weave(bencher: Bencher) {
    let weave = small_authored_weave();
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        black_box(weave.clone());
    });
}

/// Bind a pre-cloned small weave; input cloning is outside the timed region.
#[divan::bench]
fn bind_small_precloned(bencher: Bencher) {
    let weave = small_authored_weave();
    bencher
        .counter(ItemsCount::new(1u64))
        .with_inputs(|| weave.clone())
        .bench_local_values(|weave| {
            let rt = Runtime::bind(black_box(weave), BindOpts::default()).unwrap();
            black_box(rt);
        });
}

/// Clone and bind a small weave as one asset-load operation.
#[divan::bench]
fn clone_and_bind_small_weave(bencher: Bencher) {
    let weave = small_authored_weave();
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        let rt = Runtime::bind(black_box(weave.clone()), BindOpts::default()).unwrap();
        black_box(rt);
    });
}

/// Bind a pre-cloned deep Not chain (validate depth + topo).
#[divan::bench(args = [16, 64, 128])]
fn bind_not_chain_precloned(bencher: Bencher, n: usize) {
    let weave = chain_not_weave(n);
    bencher
        .counter(ItemsCount::new(1u64))
        .with_inputs(|| weave.clone())
        .bench_local_values(|weave| {
            let rt = bind_deep_owned(black_box(weave));
            black_box(rt);
        });
}

/// Expand monostable pattern only (no Runtime).
#[divan::bench]
fn expand_pattern_monostable(bencher: Bencher) {
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        black_box(expand_monostable_once());
    });
}

/// Bind a pre-cloned weave that already included monostable.
#[divan::bench]
fn bind_preincluded_monostable_precloned(bencher: Bencher) {
    let weave = weave_with_monostable_include();
    bencher
        .counter(ItemsCount::new(1u64))
        .with_inputs(|| weave.clone())
        .bench_local_values(|weave| {
            let rt = Runtime::bind(black_box(weave), BindOpts::default()).unwrap();
            black_box(rt);
        });
}

/// Full include+build+bind each sample (authoring reload cost).
#[divan::bench]
fn include_build_bind_monostable(bencher: Bencher) {
    bencher.counter(ItemsCount::new(1u64)).bench_local(|| {
        let weave = weave_with_monostable_include();
        let rt = Runtime::bind(black_box(weave), BindOpts::default()).unwrap();
        black_box(rt);
    });
}

fn main() {
    divan::main();
}
