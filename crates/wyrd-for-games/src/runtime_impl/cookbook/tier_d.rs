//! Tier D — chamber-scale composition with host-owned world effects.
//!
//! The recipe combines several local mechanisms into one readable rule circuit. The host still
//! owns spatial queries, the gate and bridge entities, saved progression, and the room transition.

#![allow(clippy::result_large_err)] // CookbookError intentionally preserves context.

use super::helpers::{bind_default, emit_count, signal_out_truthy, signal_out_value, tick_senses};
use super::Result;
use crate::runtime_impl::host::ScriptedHost;
use crate::foundation::{from_level, FlagPriority, KnotKind, SignalDomain, ONE, ZERO};
use crate::authoring::Weave;

/// D01: Shrine chamber — multi-object latch, mover target, and edge-only transition request.
///
/// A host samples three spatial facts: a crate on one pad, a player on another, and a relic on its
/// pedestal. Once all three are present, the graph latches `"shrine.gate.open"`. A separate level
/// selects the continuous `"shrine.bridge.target"`; the host performs movement and collision. When
/// the player later enters the exit while the gate is latched, the graph emits one
/// `"world.request_transition"` command on the rising edge.
///
/// This is a room-scale mechanism, not a room engine: carry the resulting progression into the
/// next room through host-owned save state and sample it back as a normal sense.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_d::run_d01_shrine_chamber().unwrap();
/// ```
pub fn run_d01_shrine_chamber() -> Result<()> {
    let mut b = Weave::builder("shrine-chamber")?;

    let crate_on_pad = b.knot("crate_on_sun_pad", KnotKind::signal_in(SignalDomain::Bool))?;
    let player_on_pad = b.knot(
        "player_on_moon_pad",
        KnotKind::signal_in(SignalDomain::Bool),
    )?;
    let relic_placed = b.knot("relic_placed", KnotKind::signal_in(SignalDomain::Bool))?;
    let pads_ready = b.knot("pads_ready", KnotKind::and2())?;
    let shrine_ready = b.knot("shrine_ready", KnotKind::and2())?;
    let unlock_edge = b.knot("unlock_edge", KnotKind::rising_from_zero())?;
    let gate_latch = b.knot("gate_latch", KnotKind::flag(FlagPriority::ResetWins, false))?;
    let gate_open = b.knot(
        "gate_open",
        KnotKind::signal_out("shrine.gate.open", SignalDomain::Bool),
    )?;

    let bridge_lever = b.knot("bridge_lever", KnotKind::signal_in(SignalDomain::Level))?;
    let bridge_target = b.knot(
        "bridge_target",
        KnotKind::map(ZERO, ONE, ZERO, from_level(8.0), SignalDomain::Level),
    )?;
    let bridge_out = b.knot(
        "bridge_out",
        KnotKind::signal_out("shrine.bridge.target", SignalDomain::Level),
    )?;

    let player_at_exit = b.knot("player_at_exit", KnotKind::signal_in(SignalDomain::Bool))?;
    let exit_ready = b.knot("exit_ready", KnotKind::and2())?;
    let exit_edge = b.knot("exit_edge", KnotKind::rising_from_zero())?;
    let transition = b.knot(
        "transition",
        KnotKind::emit_command("world.request_transition"),
    )?;

    let from = b.output(&crate_on_pad, "out")?;
    let to = b.input(&pads_ready, "in_0")?;
    b.connect(from, to)?;
    let from = b.output(&player_on_pad, "out")?;
    let to = b.input(&pads_ready, "in_1")?;
    b.connect(from, to)?;
    let from = b.output(&pads_ready, "out")?;
    let to = b.input(&shrine_ready, "in_0")?;
    b.connect(from, to)?;
    let from = b.output(&relic_placed, "out")?;
    let to = b.input(&shrine_ready, "in_1")?;
    b.connect(from, to)?;
    let from = b.output(&shrine_ready, "out")?;
    let to = b.input(&unlock_edge, "in")?;
    b.connect(from, to)?;
    let from = b.output(&unlock_edge, "out")?;
    let to = b.input(&gate_latch, "set")?;
    b.connect(from, to)?;
    let from = b.output(&gate_latch, "out")?;
    let to = b.input(&gate_open, "in")?;
    b.connect(from, to)?;

    let from = b.output(&bridge_lever, "out")?;
    let to = b.input(&bridge_target, "in")?;
    b.connect(from, to)?;
    let from = b.output(&bridge_target, "out")?;
    let to = b.input(&bridge_out, "in")?;
    b.connect(from, to)?;

    let from = b.output(&gate_latch, "out")?;
    let to = b.input(&exit_ready, "in_0")?;
    b.connect(from, to)?;
    let from = b.output(&player_at_exit, "out")?;
    let to = b.input(&exit_ready, "in_1")?;
    b.connect(from, to)?;
    let from = b.output(&exit_ready, "out")?;
    let to = b.input(&exit_edge, "in")?;
    b.connect(from, to)?;
    let from = b.output(&exit_edge, "out")?;
    let to = b.input(&transition, "trigger")?;
    b.connect(from, to)?;

    let weave = b.build()?;
    let mut rt = bind_default(&weave)?;
    let crate_on_pad = rt.sense_id("crate_on_sun_pad").expect("crate sense");
    let player_on_pad = rt.sense_id("player_on_moon_pad").expect("player pad sense");
    let relic_placed = rt.sense_id("relic_placed").expect("relic sense");
    let bridge_lever = rt.sense_id("bridge_lever").expect("bridge sense");
    let player_at_exit = rt.sense_id("player_at_exit").expect("exit sense");
    let mut host = ScriptedHost::new();

    tick_senses(
        &mut host,
        &mut rt,
        &[
            (crate_on_pad, ZERO),
            (player_on_pad, ZERO),
            (relic_placed, ZERO),
            (bridge_lever, ZERO),
            (player_at_exit, ZERO),
        ],
    )?;
    assert!(!signal_out_truthy(&rt, "shrine.gate.open"));
    assert_eq!(signal_out_value(&rt, "shrine.bridge.target"), ZERO);
    assert_eq!(emit_count(&rt, "world.request_transition"), 0);

    tick_senses(
        &mut host,
        &mut rt,
        &[
            (crate_on_pad, ONE),
            (player_on_pad, ONE),
            (relic_placed, ZERO),
            (bridge_lever, ONE),
            (player_at_exit, ZERO),
        ],
    )?;
    assert!(!signal_out_truthy(&rt, "shrine.gate.open"));
    assert_eq!(
        signal_out_value(&rt, "shrine.bridge.target"),
        from_level(8.0)
    );

    tick_senses(
        &mut host,
        &mut rt,
        &[
            (crate_on_pad, ONE),
            (player_on_pad, ONE),
            (relic_placed, ONE),
            (bridge_lever, ONE),
            (player_at_exit, ZERO),
        ],
    )?;
    assert!(signal_out_truthy(&rt, "shrine.gate.open"));
    assert_eq!(emit_count(&rt, "world.request_transition"), 0);

    // The gate remains latched after the original arrangement changes; the bridge remains a
    // separate continuous contract controlled by the host's lever observation.
    tick_senses(
        &mut host,
        &mut rt,
        &[
            (crate_on_pad, ZERO),
            (player_on_pad, ZERO),
            (relic_placed, ZERO),
            (bridge_lever, ZERO),
            (player_at_exit, ZERO),
        ],
    )?;
    assert!(signal_out_truthy(&rt, "shrine.gate.open"));
    assert_eq!(signal_out_value(&rt, "shrine.bridge.target"), ZERO);

    tick_senses(
        &mut host,
        &mut rt,
        &[
            (crate_on_pad, ZERO),
            (player_on_pad, ZERO),
            (relic_placed, ZERO),
            (bridge_lever, ZERO),
            (player_at_exit, ONE),
        ],
    )?;
    assert_eq!(emit_count(&rt, "world.request_transition"), 1);

    // Staying in the exit does not repeat a one-shot transition request.
    tick_senses(
        &mut host,
        &mut rt,
        &[
            (crate_on_pad, ZERO),
            (player_on_pad, ZERO),
            (relic_placed, ZERO),
            (bridge_lever, ZERO),
            (player_at_exit, ONE),
        ],
    )?;
    assert_eq!(emit_count(&rt, "world.request_transition"), 0);
    Ok(())
}
