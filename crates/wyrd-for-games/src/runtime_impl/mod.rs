//! Executable runtime: bind a validated Weave, sample senses, loom, read outbox.
//!
//! Lifecycle: [`Runtime::bind`] consumes a [`crate::Weave`] into dense
//! buffers and intern tables. Each frame: host writes senses via
//! [`PortWriter`], [`Runtime::begin_frame`] + [`Runtime::loom`] settle once,
//! host applies [`Outbox`] (`SignalOut` / `EmitCommand`). No engine types cross
//! this boundary — dense `SenseId` / `HostPathId` / `CmdId` only on the hot path.
//!
//! Tutorial recipes live in [`cookbook`] (Tier A → B → C); they are pedagogy,
//! not hot-path API.

pub(crate) mod bind;
pub mod cookbook;
pub(crate) mod error;
pub(crate) mod handles;
pub(crate) mod host;
pub(crate) mod kind_tag;
pub(crate) mod loom;
pub(crate) mod outbox;
pub(crate) mod recipe;
pub(crate) mod runtime_state;
pub(crate) mod scenario;

pub use bind::{BindOpts, Runtime};
pub use error::{
    BindError, CookbookError, HandleError, RecipeEndpoint, RecipeError, RecipeResolveError,
    RestoreError, ScenarioError,
};
pub use handles::{CmdId, HostPathId, KnotHandle, SenseId};
pub use host::{
    append_commands, outbox_to_commands, tick_once, Host, HostCommand, NullHost, ScriptedHost,
};
pub use outbox::{Emit, Outbox, PortWriter, SignalOutSample};
pub use recipe::{
    EmitCommandManifest, Recipe, RecipeInstance, RecipeManifest, SignalInManifest,
    SignalOutManifest,
};
pub use runtime_state::{RuntimeState, RUNTIME_STATE_FORMAT_VERSION};
pub use scenario::Scenario;
