#![allow(dead_code)] // Each integration-test crate uses only the helpers it needs.

use wyrd::{is_truthy, Runtime, Signal};

#[track_caller]
pub(crate) fn signal_out_truthy(runtime: &Runtime, path: &str) -> bool {
    is_truthy(signal_out_value(runtime, path))
}

#[track_caller]
pub(crate) fn signal_out_value(runtime: &Runtime, path: &str) -> Signal {
    let path_id = runtime
        .path_id(path)
        .unwrap_or_else(|| panic!("SignalOut path `{path}` is not bound"));

    runtime
        .outbox()
        .signals()
        .iter()
        .find(|signal| signal.path == path_id)
        .map(|signal| signal.value)
        .unwrap_or_else(|| {
            panic!(
                "SignalOut path `{path}` has no sample in the current frame; call Runtime::loom first"
            )
        })
}
