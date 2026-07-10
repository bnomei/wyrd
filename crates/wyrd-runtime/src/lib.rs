//! Executable runtime: bind a validated Weave, sample senses, loom, read outbox.
//!
//! Lifecycle: [`Runtime::bind`] consumes a [`wyrd_graph::Weave`] into dense
//! buffers and intern tables. Each frame: host writes senses via
//! [`PortWriter`], [`Runtime::begin_frame`] + [`Runtime::loom`] settle once,
//! host applies [`Outbox`] (`SignalOut` / `EmitCommand`). No engine types cross
//! this boundary — dense `SenseId` / `HostPathId` / `CmdId` only on the hot path.
//!
//! Tutorial recipes live in [`cookbook`] (Tier A → B → C); they are pedagogy,
//! not hot-path API.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
extern crate no_std_compat as std;

mod bind;
pub mod cookbook;
mod error;
mod handles;
mod host;
mod kind_tag;
mod loom;
mod outbox;

pub use bind::{BindOpts, Runtime};
pub use error::{BindError, CookbookError, HandleError};
pub use handles::{CmdId, HostPathId, KnotHandle, SenseId};
pub use host::{
    append_commands, outbox_to_commands, tick_once, Host, HostCommand, NullHost, ScriptedHost,
};
pub use outbox::{Emit, Outbox, PortWriter, SignalOutSample};

pub use wyrd_core::{from_count, is_truthy, HostTime, KnotId, PortSlot, Seed, Signal, ONE, ZERO};
pub use wyrd_graph::{
    validate, validate_report, Budget, BudgetWarning, KnotKind, ValidateReport, Weave,
};
