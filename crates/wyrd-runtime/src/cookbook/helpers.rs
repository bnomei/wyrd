//! Shared asserts and tick drivers for tutorial recipes.

use crate::host::{tick_once, ScriptedHost};
use crate::BindOpts;
use crate::Runtime;
use wyrd_core::{is_truthy, HostTime, KnotId, Result, Signal, ZERO};
use wyrd_graph::Weave;

/// True if SignalOut path is present and truthy this frame.
pub fn signal_out_truthy(rt: &Runtime, path: &str) -> bool {
    let Some(pid) = rt.path_id(path) else {
        return false;
    };
    rt.outbox()
        .signals()
        .iter()
        .any(|s| s.path == pid && is_truthy(s.value))
}

/// Signal value on path, or ZERO if missing.
pub fn signal_out_value(rt: &Runtime, path: &str) -> Signal {
    let Some(pid) = rt.path_id(path) else {
        return ZERO;
    };
    rt.outbox()
        .signals()
        .iter()
        .find(|s| s.path == pid)
        .map(|s| s.value)
        .unwrap_or(ZERO)
}

/// Emit count for command name this frame.
pub fn emit_count(rt: &Runtime, cmd_name: &str) -> usize {
    rt.outbox()
        .emits()
        .iter()
        .filter(|e| rt.cmd_name(e.cmd) == cmd_name)
        .count()
}

/// Bind with default opts (no Random seed required).
pub fn bind_default(weave: &Weave) -> Result<Runtime> {
    Runtime::bind(weave, BindOpts::default())
}

/// One loom after setting senses (no Host trait).
pub fn sample_loom(
    rt: &mut Runtime,
    weave: &Weave,
    tick: u64,
    senses: &[(KnotId, Signal)],
) -> Result<()> {
    rt.begin_frame(HostTime { tick });
    {
        let mut w = rt.port_writer();
        for &(id, v) in senses {
            w.set_sense(id, v);
        }
    }
    rt.loom(weave)
}

/// Push one frame of sense values and `tick_once`.
pub fn tick_senses(
    host: &mut ScriptedHost,
    rt: &mut Runtime,
    weave: &Weave,
    senses: &[(KnotId, Signal)],
) -> Result<()> {
    host.push_frame(senses.iter().copied());
    tick_once(host, rt, weave)
}
