//! Shared asserts and tick drivers for tutorial recipes.

#![allow(clippy::result_large_err)] // CookbookError intentionally preserves context.

use crate::authoring::Weave;
use crate::foundation::{is_truthy, HostTime, Signal};
use crate::runtime_impl::host::{tick_once, ScriptedHost};
use crate::BindOpts;
use crate::{BindError, HandleError, Runtime};

use crate::SenseId;

/// Returns whether a bound `SignalOut` emitted a truthy value this frame.
///
/// A legitimate falsey sample is returned as `false`; it is not confused with
/// a missing sample.
///
/// # Panics
///
/// Panics if `path` is not bound to a `SignalOut`, or if the current frame has
/// no sample for that path (for example, because [`Runtime::loom`] has not run).
#[track_caller]
pub fn signal_out_truthy(rt: &Runtime, path: &str) -> bool {
    is_truthy(signal_out_value(rt, path))
}

/// Returns the sample emitted by a bound `SignalOut` this frame.
///
/// A legitimate zero sample is returned as-is; it is not confused with a
/// missing sample.
///
/// # Panics
///
/// Panics if `path` is not bound to a `SignalOut`, or if the current frame has
/// no sample for that path (for example, because [`Runtime::loom`] has not run).
#[track_caller]
pub fn signal_out_value(rt: &Runtime, path: &str) -> Signal {
    let pid = rt
        .path_id(path)
        .unwrap_or_else(|| panic!("SignalOut path `{path}` is not bound"));
    rt.outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| s.value)
        .unwrap_or_else(|| {
            panic!(
                "SignalOut path `{path}` has no sample in the current frame; call Runtime::loom first"
            )
        })
}

/// Emit count for command name this frame.
pub fn emit_count(rt: &Runtime, cmd_name: &str) -> usize {
    rt.outbox()
        .emits()
        .iter()
        .filter(|e| rt.cmd_name(e.cmd) == Ok(cmd_name))
        .count()
}

/// Bind with default opts (no Random seed required).
pub fn bind_default(weave: &Weave) -> Result<Runtime, BindError> {
    Runtime::bind(weave.clone(), BindOpts::default())
}

/// One loom after setting senses (no Host trait).
pub fn sample_loom(
    rt: &mut Runtime,
    tick: u64,
    senses: &[(SenseId, Signal)],
) -> Result<(), HandleError> {
    rt.begin_frame(HostTime { tick });
    {
        let mut w = rt.port_writer();
        for &(id, v) in senses {
            w.set_sense(id, v)?;
        }
    }
    rt.loom();
    Ok(())
}

/// Push one frame of sense values and `tick_once`.
pub fn tick_senses(
    host: &mut ScriptedHost,
    rt: &mut Runtime,
    senses: &[(SenseId, Signal)],
) -> Result<(), HandleError> {
    host.push_frame(senses.iter().copied());
    tick_once(host, rt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::foundation::{KnotKind, SignalDomain, ZERO};

    fn runtime_with_zero_output() -> Runtime {
        let mut builder = Weave::builder("strict-signal-out").unwrap();
        let constant = builder
            .knot("zero", KnotKind::constant(ZERO, SignalDomain::Bool))
            .unwrap();
        let output = builder
            .knot(
                "output",
                KnotKind::signal_out("known.output", SignalDomain::Bool),
            )
            .unwrap();
        let from = builder.output(&constant, "out").unwrap();
        let to = builder.input(&output, "in").unwrap();
        builder.connect(from, to).unwrap();
        bind_default(&builder.build().unwrap()).unwrap()
    }

    #[test]
    fn emitted_zero_remains_a_present_falsey_sample() {
        let mut rt = runtime_with_zero_output();
        rt.begin_frame(HostTime { tick: 0 });
        rt.loom();

        assert_eq!(signal_out_value(&rt, "known.output"), ZERO);
        assert!(!signal_out_truthy(&rt, "known.output"));
    }

    #[test]
    #[should_panic(expected = "SignalOut path `unknown.output` is not bound")]
    fn unknown_signal_out_path_panics() {
        let rt = runtime_with_zero_output();
        let _ = signal_out_value(&rt, "unknown.output");
    }

    #[test]
    #[should_panic(expected = "SignalOut path `known.output` has no sample in the current frame")]
    fn missing_current_frame_sample_panics() {
        let mut rt = runtime_with_zero_output();
        rt.begin_frame(HostTime { tick: 0 });
        rt.loom();
        rt.begin_frame(HostTime { tick: 1 });

        let _ = signal_out_value(&rt, "known.output");
    }
}
