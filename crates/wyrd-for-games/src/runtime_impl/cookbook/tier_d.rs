//! Tier D — chamber-scale composition with host-owned world effects.

#![allow(clippy::result_large_err)]

use super::helpers::{bind_default, emit_count, signal_out_truthy, signal_out_value, tick_senses};
use super::Result;
use crate::authoring::{BuildError, Weave};
use crate::foundation::{from_level, FlagPriority, KnotKind, SignalDomain, ONE, ZERO};
use crate::runtime_impl::host::ScriptedHost;
use crate::weave;

/// Declarative topology for the shrine chamber.
pub fn d01_shrine_chamber_weave() -> core::result::Result<Weave, BuildError> {
    weave! {
        id: "shrine-chamber";
        knots {
            crate_on_sun_pad = KnotKind::signal_in(SignalDomain::Bool);
            player_on_moon_pad = KnotKind::signal_in(SignalDomain::Bool);
            relic_placed = KnotKind::signal_in(SignalDomain::Bool);
            pads_ready = KnotKind::and2(); shrine_ready = KnotKind::and2(); unlock_edge = KnotKind::rising_from_zero();
            gate_latch = KnotKind::flag(FlagPriority::ResetWins, false); gate_open = KnotKind::signal_out("shrine.gate.open", SignalDomain::Bool);
            bridge_lever = KnotKind::signal_in(SignalDomain::Level); bridge_target = KnotKind::map(ZERO, ONE, ZERO, from_level(8.0), SignalDomain::Level); bridge_out = KnotKind::signal_out("shrine.bridge.target", SignalDomain::Level);
            player_at_exit = KnotKind::signal_in(SignalDomain::Bool); exit_ready = KnotKind::and2(); exit_edge = KnotKind::rising_from_zero(); transition = KnotKind::emit_command("world.request_transition");
        }
        threads {
            crate_on_sun_pad.out -> pads_ready.in_0; player_on_moon_pad.out -> pads_ready.in_1; pads_ready.out -> shrine_ready.in_0; relic_placed.out -> shrine_ready.in_1; shrine_ready.out -> unlock_edge.in; unlock_edge.out -> gate_latch.set; gate_latch.out -> gate_open.in;
            bridge_lever.out -> bridge_target.in; bridge_target.out -> bridge_out.in;
            gate_latch.out -> exit_ready.in_0; player_at_exit.out -> exit_ready.in_1; exit_ready.out -> exit_edge.in; exit_edge.out -> transition.trigger;
        }
    }
}

/// D01: Shrine chamber — multi-object latch, mover target, and edge-only transition request.
///
/// A host samples a crate and player on pads plus a relic on its pedestal. Once all
/// are present, the graph latches `"shrine.gate.open"`. A separate level selects
/// the continuous `"shrine.bridge.target"`; the host performs movement and collision.
/// Later, entering the exit emits one `"world.request_transition"` command on a
/// rising edge. The host owns world effects, room transitions, and persisted progress.
///
/// # Examples
///
/// ```
/// wyrd::cookbook::tier_d::run_d01_shrine_chamber().unwrap();
/// ```
pub fn run_d01_shrine_chamber() -> Result<()> {
    let weave = d01_shrine_chamber_weave()?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_duplicate_knot_rejected(weave: &Weave, id: &str) {
        let kind = weave
            .knots()
            .iter()
            .find(|knot| knot.id == id)
            .expect("recipe owns the requested knot")
            .kind
            .clone();
        let mut builder = Weave::builder("cookbook-duplicate").expect("valid builder id");
        builder.knot(id, kind.clone()).expect("first knot is valid");
        assert!(matches!(
            builder.knot(id, kind),
            Err(BuildError::DuplicateKnotId { .. })
        ));
    }

    #[test]
    fn d01_keeps_duplicate_knot_diagnostics() {
        let weave = d01_shrine_chamber_weave().unwrap();
        for id in [
            "player_on_moon_pad",
            "gate_open",
            "bridge_target",
            "bridge_out",
            "transition",
        ] {
            assert_duplicate_knot_rejected(&weave, id);
        }
    }

    #[test]
    fn d01_rejects_a_foreign_sense_id() {
        let weave = d01_shrine_chamber_weave().unwrap();
        let mut runtime = bind_default(&weave).unwrap();
        let foreign_runtime = bind_default(&d01_shrine_chamber_weave().unwrap()).unwrap();
        let foreign = foreign_runtime
            .sense_id("crate_on_sun_pad")
            .expect("foreign recipe sense");
        let mut host = ScriptedHost::new();

        assert!(tick_senses(&mut host, &mut runtime, &[(foreign, ZERO)]).is_err());
    }
}
