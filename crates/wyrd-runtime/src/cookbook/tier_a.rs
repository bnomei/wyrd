//! Tier A — foundations (short graphs, rustdoc-friendly).
//!
//! Each `run_*` is the CI entry point. **Examples** below show the full Weave
//! so `cargo doc --open` displays the graph, not only a function call.

use super::helpers::{bind_default, sample_loom, signal_out_truthy, tick_senses};
use crate::host::ScriptedHost;
use wyrd_core::{from_count, HostTime, KnotKind, Result, WyrdError, ONE, ZERO};
use wyrd_graph::{validate, Budget, Weave};

/// A01: Constant(ONE) → Not → SignalOut (falsey).
///
/// # Examples
///
/// ```
/// use wyrd_core::{HostTime, KnotKind, ONE};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::cookbook::helpers::{bind_default, signal_out_truthy};
///
/// // Constant ──out──► Not ──out──► SignalOut("debug.inverted")
/// let (b, _) = Weave::builder("a01")
///     .knot("c", KnotKind::constant(ONE))
///     .unwrap();
/// let (b, _) = b.knot("n", KnotKind::not()).unwrap();
/// let (b, _) = b.knot("o", KnotKind::signal_out("debug.inverted")).unwrap();
/// let weave = b
///     .wire_named("c", "out", "n", "in")
///     .wire_named("n", "out", "o", "in")
///     .build()
///     .unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// rt.begin_frame(HostTime { tick: 0 });
/// rt.loom(&weave).unwrap();
/// assert!(!signal_out_truthy(&rt, "debug.inverted"));
/// ```
pub fn run_a01_hello_invert() -> Result<()> {
    let (b, _) = Weave::builder("a01").knot("c", KnotKind::constant(ONE))?;
    let (b, _) = b.knot("n", KnotKind::not())?;
    let (b, _) = b.knot("o", KnotKind::signal_out("debug.inverted"))?;
    let weave = b
        .wire_named("c", "out", "n", "in")
        .wire_named("n", "out", "o", "in")
        .build()?;
    let mut rt = bind_default(&weave)?;
    rt.begin_frame(HostTime { tick: 0 });
    rt.loom(&weave)?;
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
/// use wyrd_core::{KnotKind, ONE, ZERO};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::cookbook::helpers::{bind_default, sample_loom, signal_out_truthy};
///
/// // plate_a ──┐
/// //           ├──► And ──► SignalOut("door.open")
/// // plate_b ──┘
/// let (b, pa) = Weave::builder("a02")
///     .knot("plate_a", KnotKind::signal_in())
///     .unwrap();
/// let (b, pb) = b.knot("plate_b", KnotKind::signal_in()).unwrap();
/// let (b, _) = b.and2("both", pa, pb).unwrap(); // wires out→in_0, out→in_1
/// let (b, _) = b.knot("door", KnotKind::signal_out("door.open")).unwrap();
/// let weave = b.wire_named("both", "out", "door", "in").build().unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let a = rt.sense_id("plate_a").unwrap();
/// let b_id = rt.sense_id("plate_b").unwrap();
///
/// sample_loom(&mut rt, &weave, 0, &[(a, ONE), (b_id, ZERO)]).unwrap();
/// assert!(!signal_out_truthy(&rt, "door.open"));
///
/// sample_loom(&mut rt, &weave, 1, &[(a, ONE), (b_id, ONE)]).unwrap();
/// assert!(signal_out_truthy(&rt, "door.open"));
/// ```
pub fn run_a02_two_plate_and() -> Result<()> {
    let (b, pa) = Weave::builder("a02").knot("plate_a", KnotKind::signal_in())?;
    let (b, pb) = b.knot("plate_b", KnotKind::signal_in())?;
    let (b, _) = b.and2("both", pa, pb)?;
    let (b, _) = b.knot("door", KnotKind::signal_out("door.open"))?;
    let weave = b.wire_named("both", "out", "door", "in").build()?;

    let mut rt = bind_default(&weave)?;
    let a = rt.sense_id("plate_a").expect("plate_a");
    let b_id = rt.sense_id("plate_b").expect("plate_b");

    sample_loom(&mut rt, &weave, 0, &[(a, ONE), (b_id, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "door.open"));

    sample_loom(&mut rt, &weave, 1, &[(a, ONE), (b_id, ONE)])?;
    assert!(signal_out_truthy(&rt, "door.open"));
    Ok(())
}

/// A03: Dense bind path — `set_sense` + loom + outbox (no Host trait).
///
/// # Examples
///
/// ```
/// use wyrd_core::{KnotKind, ONE, ZERO};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::cookbook::helpers::{bind_default, sample_loom, signal_out_truthy};
///
/// // SignalIn ──► Not ──► SignalOut("y")
/// let (b, _) = Weave::builder("a03")
///     .knot("in", KnotKind::signal_in())
///     .unwrap();
/// let (b, _) = b.knot("n", KnotKind::not()).unwrap();
/// let (b, _) = b.knot("o", KnotKind::signal_out("y")).unwrap();
/// let weave = b
///     .wire_named("in", "out", "n", "in")
///     .wire_named("n", "out", "o", "in")
///     .build()
///     .unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let id = rt.sense_id("in").unwrap();
/// sample_loom(&mut rt, &weave, 0, &[(id, ZERO)]).unwrap();
/// assert!(signal_out_truthy(&rt, "y")); // Not of ZERO
/// sample_loom(&mut rt, &weave, 1, &[(id, ONE)]).unwrap();
/// assert!(!signal_out_truthy(&rt, "y"));
/// ```
pub fn run_a03_bind_sample_loom() -> Result<()> {
    let (b, _) = Weave::builder("a03").knot("in", KnotKind::signal_in())?;
    let (b, _) = b.knot("n", KnotKind::not())?;
    let (b, _) = b.knot("o", KnotKind::signal_out("y"))?;
    let weave = b
        .wire_named("in", "out", "n", "in")
        .wire_named("n", "out", "o", "in")
        .build()?;
    let mut rt = bind_default(&weave)?;
    let id = rt.sense_id("in").expect("in");
    sample_loom(&mut rt, &weave, 0, &[(id, ZERO)])?;
    assert!(signal_out_truthy(&rt, "y"), "Not of ZERO is truthy");
    sample_loom(&mut rt, &weave, 1, &[(id, ONE)])?;
    assert!(!signal_out_truthy(&rt, "y"));
    Ok(())
}

/// A04: [`ScriptedHost`](crate::ScriptedHost) + [`tick_once`](crate::tick_once) over two frames.
///
/// # Examples
///
/// ```
/// use wyrd_core::{KnotKind, ONE, ZERO};
/// use wyrd_graph::Weave;
/// use wyrd_runtime::ScriptedHost;
/// use wyrd_runtime::cookbook::helpers::{bind_default, signal_out_truthy, tick_senses};
///
/// // SignalIn ──► SignalOut("lamp")
/// let (b, _) = Weave::builder("a04")
///     .knot("in", KnotKind::signal_in())
///     .unwrap();
/// let (b, _) = b.knot("o", KnotKind::signal_out("lamp")).unwrap();
/// let weave = b.wire_named("in", "out", "o", "in").build().unwrap();
///
/// let mut rt = bind_default(&weave).unwrap();
/// let id = rt.sense_id("in").unwrap();
/// let mut host = ScriptedHost::new();
///
/// tick_senses(&mut host, &mut rt, &weave, &[(id, ZERO)]).unwrap();
/// assert!(!signal_out_truthy(&rt, "lamp"));
/// tick_senses(&mut host, &mut rt, &weave, &[(id, ONE)]).unwrap();
/// assert!(signal_out_truthy(&rt, "lamp"));
/// ```
pub fn run_a04_host_tick_once() -> Result<()> {
    let (b, _) = Weave::builder("a04").knot("in", KnotKind::signal_in())?;
    let (b, _) = b.knot("o", KnotKind::signal_out("lamp"))?;
    let weave = b.wire_named("in", "out", "o", "in").build()?;
    let mut rt = bind_default(&weave)?;
    let id = rt.sense_id("in").expect("in");
    let mut host = ScriptedHost::new();

    tick_senses(&mut host, &mut rt, &weave, &[(id, ZERO)])?;
    assert!(!signal_out_truthy(&rt, "lamp"));

    tick_senses(&mut host, &mut rt, &weave, &[(id, ONE)])?;
    assert!(signal_out_truthy(&rt, "lamp"));
    Ok(())
}

/// A05: Validate rejects inverted Map in-range (`InvalidParam`).
///
/// # Examples
///
/// ```
/// use wyrd_core::{from_count, KnotKind, WyrdError, ONE, ZERO};
/// use wyrd_graph::{validate, Budget, Weave};
///
/// let (b, _) = Weave::builder("a05")
///     .knot("c", KnotKind::constant(ONE))
///     .unwrap();
/// let (b, _) = b
///     .knot(
///         "map",
///         KnotKind::Map {
///             in_min: from_count(5), // inverted: min > max
///             in_max: from_count(1),
///             out_min: ZERO,
///             out_max: ONE,
///         },
///     )
///     .unwrap();
/// let (b, _) = b.knot("out", KnotKind::signal_out("y")).unwrap();
/// let weave = b
///     .wire_named("c", "out", "map", "in")
///     .wire_named("map", "out", "out", "in")
///     .build()
///     .unwrap();
///
/// assert!(matches!(
///     validate(&weave, &Budget::default()),
///     Err(WyrdError::InvalidParam)
/// ));
/// ```
pub fn run_a05_validate_fails() -> Result<()> {
    let (b, _) = Weave::builder("a05").knot("c", KnotKind::constant(ONE))?;
    let (b, _) = b.knot(
        "map",
        KnotKind::Map {
            in_min: from_count(5),
            in_max: from_count(1),
            out_min: ZERO,
            out_max: ONE,
        },
    )?;
    let (b, _) = b.knot("out", KnotKind::signal_out("y"))?;
    let weave = b
        .wire_named("c", "out", "map", "in")
        .wire_named("map", "out", "out", "in")
        .build()?;
    match validate(&weave, &Budget::default()) {
        Err(WyrdError::InvalidParam) => Ok(()),
        other => panic!("expected InvalidParam, got {other:?}"),
    }
}
