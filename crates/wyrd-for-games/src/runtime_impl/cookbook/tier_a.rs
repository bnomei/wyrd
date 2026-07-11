//! Tier A — foundations (short graphs, rustdoc-friendly).
//!
//! Each `run_*` is the CI entry point. **Examples** below show the full Weave
//! so `cargo doc --open` displays the graph, not only a function call.

#![allow(clippy::result_large_err)] // CookbookError intentionally preserves context.

use super::helpers::{bind_default, sample_loom, signal_out_truthy, tick_senses};
use super::Result;
use crate::authoring::{ValidationError, Weave};
use crate::foundation::{from_count, HostTime, KnotKind, SignalDomain, ONE, ZERO};
use crate::runtime_impl::host::ScriptedHost;

fn duplicate_knot_id<'a>(
    failure_at: Option<&str>,
    target: &str,
    default: &'a str,
    duplicate: &'a str,
) -> &'a str {
    if failure_at == Some(target) {
        duplicate
    } else {
        default
    }
}

/// A01: Constant(ONE) → Not → SignalOut (falsey).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a01_hello_invert().unwrap();
/// ```
pub fn run_a01_hello_invert() -> Result<()> {
    run_a01_hello_invert_with(None)
}

fn run_a01_hello_invert_with(failure_at: Option<&str>) -> Result<()> {
    let mut b = Weave::builder("a01")?;
    let k_c = b.knot("c", KnotKind::constant(ONE, SignalDomain::Bool))?;
    let k_n = b.knot("n", KnotKind::not())?;
    let k_o = b.knot(
        duplicate_knot_id(failure_at, "a01.output", "o", "c"),
        KnotKind::signal_out("debug.inverted", SignalDomain::Bool),
    )?;
    let from = b.output(&k_c, "out")?;
    let to = b.input(&k_n, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_n, "out")?;
    let to = b.input(&k_o, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;
    let mut rt = bind_default(&weave)?;
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    assert!(
        !signal_out_truthy(&rt, "debug.inverted"),
        "Not of ONE is falsey"
    );
    Ok(())
}

/// A02: Two plates → And → `door.open` request (host owns the door).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a02_two_plate_and().unwrap();
/// ```
pub fn run_a02_two_plate_and() -> Result<()> {
    run_a02_two_plate_and_with(None)
}

fn run_a02_two_plate_and_with(failure_at: Option<&str>) -> Result<()> {
    let mut b = Weave::builder("a02")?;
    let pa = b.knot("plate_a", KnotKind::signal_in(SignalDomain::Bool))?;
    let pb = b.knot("plate_b", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_both = b.knot("both", KnotKind::and2())?;
    let from = b.output(&pa, "out")?;
    let to = b.input(&k_both, "in_0")?;
    b.connect(from, to)?;
    let from = b.output(&pb, "out")?;
    let to = b.input(&k_both, "in_1")?;
    b.connect(from, to)?;
    let k_door = b.knot(
        duplicate_knot_id(failure_at, "a02.door", "door", "plate_a"),
        KnotKind::signal_out("door.open", SignalDomain::Bool),
    )?;
    let from = b.output(&k_both, "out")?;
    let to = b.input(&k_door, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;

    let mut rt = bind_default(&weave)?;
    let a = rt.sense_id("plate_a").expect("plate_a");
    let b_id = rt.sense_id("plate_b").expect("plate_b");

    sample_loom(&mut rt, 0, &[(a, ONE), (b_id, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "door.open"));

    sample_loom(&mut rt, 1, &[(a, ONE), (b_id, ONE)])?;
    assert!(signal_out_truthy(&rt, "door.open"));
    Ok(())
}

/// A03: Dense bind path — `set_sense` + loom + outbox (no Host trait).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a03_bind_sample_loom().unwrap();
/// ```
pub fn run_a03_bind_sample_loom() -> Result<()> {
    let mut b = Weave::builder("a03")?;
    let k_in = b.knot("in", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_n = b.knot("n", KnotKind::not())?;
    let k_o = b.knot("o", KnotKind::signal_out("y", SignalDomain::Bool))?;
    let from = b.output(&k_in, "out")?;
    let to = b.input(&k_n, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_n, "out")?;
    let to = b.input(&k_o, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;
    let mut rt = bind_default(&weave)?;
    let id = rt.sense_id("in").expect("in");
    sample_loom(&mut rt, 0, &[(id, ZERO)])?;
    assert!(signal_out_truthy(&rt, "y"), "Not of ZERO is truthy");
    sample_loom(&mut rt, 1, &[(id, ONE)])?;
    assert!(!signal_out_truthy(&rt, "y"));
    Ok(())
}

/// A04: [`ScriptedHost`](crate::ScriptedHost) + [`tick_once`](crate::tick_once) over two frames.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a04_host_tick_once().unwrap();
/// ```
pub fn run_a04_host_tick_once() -> Result<()> {
    let mut b = Weave::builder("a04")?;
    let k_in = b.knot("in", KnotKind::signal_in(SignalDomain::Bool))?;
    let k_o = b.knot("o", KnotKind::signal_out("lamp", SignalDomain::Bool))?;
    let from = b.output(&k_in, "out")?;
    let to = b.input(&k_o, "in")?;
    b.connect(from, to)?;
    let weave = b.build()?;
    let mut rt = bind_default(&weave)?;
    let id = rt.sense_id("in").expect("in");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &[(id, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "lamp"));

    tick_senses(&mut host, &mut rt, &[(id, ONE)])?;
    assert!(signal_out_truthy(&rt, "lamp"));
    Ok(())
}

/// A05: Validate rejects inverted Map in-range (`InvalidParam`).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a05_validate_fails().unwrap();
/// ```
pub fn run_a05_validate_fails() -> Result<()> {
    run_a05_validate_fails_with(None, false)
}

fn run_a05_validate_fails_with(failure_at: Option<&str>, valid_map: bool) -> Result<()> {
    let mut b = Weave::builder("a05")?;
    let k_c = b.knot("c", KnotKind::constant(ONE, SignalDomain::Level))?;
    let k_map = b.knot(
        duplicate_knot_id(failure_at, "a05.map", "map", "c"),
        KnotKind::Map {
            domain: SignalDomain::Level,
            in_min: from_count(if valid_map { 0 } else { 5 }),
            in_max: from_count(1),
            out_min: ZERO,
            out_max: ONE,
        },
    )?;
    let k_out = b.knot("out", KnotKind::signal_out("y", SignalDomain::Level))?;
    let from = b.output(&k_c, "out")?;
    let to = b.input(&k_map, "in")?;
    b.connect(from, to)?;
    let from = b.output(&k_map, "out")?;
    let to = b.input(&k_out, "in")?;
    b.connect(from, to)?;
    match b.build() {
        Err(ValidationError::InvalidParameter { .. }) => Ok(()),
        other => panic!("expected invalid Map range, got {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a01_duplicate_output_propagates_the_real_builder_error() {
        assert!(run_a01_hello_invert_with(Some("a01.output")).is_err());
    }

    #[test]
    fn a02_duplicate_door_propagates_the_real_builder_error() {
        assert!(run_a02_two_plate_and_with(Some("a02.door")).is_err());
    }

    #[test]
    fn a05_duplicate_map_propagates_the_real_builder_error() {
        assert!(run_a05_validate_fails_with(Some("a05.map"), false).is_err());
    }

    #[test]
    #[should_panic(expected = "expected invalid Map range")]
    fn a05_valid_map_reaches_the_diagnostic_branch() {
        let _ = run_a05_validate_fails_with(None, true);
    }
}
