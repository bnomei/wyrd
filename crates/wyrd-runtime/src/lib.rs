//! Runtime: bind → sample → loom → outbox.
//!
//! Tutorial recipes live in [`cookbook`] (Tier A → B → C).

#![no_std]
#![forbid(unsafe_code)]

extern crate no_std_compat as std;

mod bind;
pub mod cookbook;
mod host;
mod kind_tag;
mod loom;
mod outbox;

pub use bind::{BindOpts, Runtime};
pub use host::{
    append_commands, outbox_to_commands, tick_once, Host, HostCommand, NullHost, ScriptedHost,
};
pub use outbox::{Emit, Outbox, PortWriter, SignalOutSample};

pub use wyrd_core::{
    from_count, is_truthy, CmdId, HostPathId, HostTime, KnotId, PortSlot, Result, Seed, Signal,
    WyrdError, ONE, ZERO,
};
pub use wyrd_graph::{
    validate, validate_report, Budget, BudgetWarning, KnotKind, ValidateReport, Weave,
};
