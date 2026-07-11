//! Tier A — foundations (short graphs, rustdoc-friendly).

#![allow(clippy::result_large_err)]

use super::helpers::{bind_default, sample_loom, signal_out_truthy, tick_senses};
use super::Result;
use crate::authoring::{BuildError, ValidationError, Weave};
use crate::foundation::{from_count, HostTime, KnotKind, SignalDomain, ONE, ZERO};
use crate::runtime_impl::host::ScriptedHost;
use crate::weave;

/// Topology for A01: Constant(ONE) → Not → SignalOut.
pub fn a01_hello_invert_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "a01";
        knots {
            c = KnotKind::constant(ONE, SignalDomain::Bool);
            n = KnotKind::not();
            o = KnotKind::signal_out("debug.inverted", SignalDomain::Bool);
        }
        threads { c.out -> n.in; n.out -> o.in; }
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
    let weave = a01_hello_invert_weave()?;
    let mut rt = bind_default(&weave)?;
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom();
    assert!(
        !signal_out_truthy(&rt, "debug.inverted"),
        "Not of ONE is falsey"
    );
    Ok(())
}

/// Topology for A02: two plates → And → `door.open`.
pub fn a02_two_plate_and_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "a02";
        knots {
            plate_a = KnotKind::signal_in(SignalDomain::Bool);
            plate_b = KnotKind::signal_in(SignalDomain::Bool);
            both = KnotKind::and2();
            door = KnotKind::signal_out("door.open", SignalDomain::Bool);
        }
        threads { plate_a.out -> both.in_0; plate_b.out -> both.in_1; both.out -> door.in; }
    }
}

/// A02: Two plates → And → `door.open` request (host owns the door).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a02_two_plate_and().unwrap();
/// ```
pub fn run_a02_two_plate_and() -> Result<()> {
    let weave = a02_two_plate_and_weave()?;
    let mut rt = bind_default(&weave)?;
    let a = rt.sense_id("plate_a").expect("plate_a");
    let b = rt.sense_id("plate_b").expect("plate_b");
    sample_loom(&mut rt, 0, &[(a, ONE), (b, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "door.open"));
    sample_loom(&mut rt, 1, &[(a, ONE), (b, ONE)])?;
    assert!(signal_out_truthy(&rt, "door.open"));
    Ok(())
}

/// Topology for A03: input → Not → output.
pub fn a03_bind_sample_loom_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "a03";
        knots {
            input as "in" = KnotKind::signal_in(SignalDomain::Bool);
            n = KnotKind::not();
            o = KnotKind::signal_out("y", SignalDomain::Bool);
        }
        threads { input.out -> n.in; n.out -> o.in; }
    }
}

/// A03: Dense bind path — `set_sense` + loom + outbox (no Host trait).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a03_bind_sample_loom().unwrap();
/// ```
pub fn run_a03_bind_sample_loom() -> Result<()> {
    let weave = a03_bind_sample_loom_weave()?;
    let mut rt = bind_default(&weave)?;
    let id = rt.sense_id("in").expect("in");
    sample_loom(&mut rt, 0, &[(id, ZERO)])?;
    assert!(signal_out_truthy(&rt, "y"), "Not of ZERO is truthy");
    sample_loom(&mut rt, 1, &[(id, ONE)])?;
    assert!(!signal_out_truthy(&rt, "y"));
    Ok(())
}

/// Topology for A04: input → `lamp` output.
pub fn a04_host_tick_once_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "a04";
        knots { input as "in" = KnotKind::signal_in(SignalDomain::Bool); o = KnotKind::signal_out("lamp", SignalDomain::Bool); }
        threads { input.out -> o.in; }
    }
}

/// A04: [`ScriptedHost`](crate::ScriptedHost) + [`tick_once`](crate::tick_once) over two frames.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a04_host_tick_once().unwrap();
/// ```
pub fn run_a04_host_tick_once() -> Result<()> {
    let weave = a04_host_tick_once_weave()?;
    let mut rt = bind_default(&weave)?;
    let id = rt.sense_id("in").expect("in");
    let mut host = ScriptedHost::new();
    tick_senses(&mut host, &mut rt, &[(id, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "lamp"));
    tick_senses(&mut host, &mut rt, &[(id, ONE)])?;
    assert!(signal_out_truthy(&rt, "lamp"));
    Ok(())
}

/// Topology for A05, deliberately invalid because its Map range is inverted.
pub fn a05_validate_fails_weave() -> core::result::Result<Weave, BuildError> {
    a05_validate_fails_weave_with(false)
}

fn a05_validate_fails_weave_with(valid_map: bool) -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "a05";
        knots {
            c = KnotKind::constant(ONE, SignalDomain::Level);
            map = KnotKind::Map { domain: SignalDomain::Level, in_min: from_count(if valid_map { 0 } else { 5 }), in_max: from_count(1), out_min: ZERO, out_max: ONE };
            o = KnotKind::signal_out("y", SignalDomain::Level);
        }
        threads { c.out -> map.in; map.out -> o.in; }
    }
}

/// A05: Validate rejects inverted Map in-range (`InvalidParameter`).
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_a::run_a05_validate_fails().unwrap();
/// ```
pub fn run_a05_validate_fails() -> Result<()> {
    run_a05_validate_fails_with(false)
}

fn run_a05_validate_fails_with(valid_map: bool) -> Result<()> {
    match a05_validate_fails_weave_with(valid_map) {
        Err(BuildError::Validation(ValidationError::InvalidParameter { .. })) => Ok(()),
        other => panic!("expected invalid Map range, got {other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a05_invalid_map_preserves_the_validation_error() {
        assert!(matches!(
            a05_validate_fails_weave(),
            Err(BuildError::Validation(
                ValidationError::InvalidParameter { .. }
            ))
        ));
    }

    #[test]
    #[should_panic(expected = "expected invalid Map range")]
    fn a05_valid_map_reaches_the_diagnostic_branch() {
        let _ = run_a05_validate_fails_with(true);
    }
}
