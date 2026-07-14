//! Engine-neutral signal-graph game logic for Wyrd.
//!
//! Author an immutable [`Weave`], bind it to dense ids with [`Runtime`], then
//! sample host senses and settle it once per frame. Use [`core`], [`graph`],
//! and [`runtime`] when an explicit layer namespace is clearer.
//!
//! The crate root re-exports the common authoring and runtime vocabulary. The
//! layer modules provide the same contracts grouped by intent. Implementation
//! modules stay private so downstream code cannot depend on bind tables or
//! runtime storage details:
//!
//! ```compile_fail
//! use wyrd::runtime_impl::Runtime;
//! ```
//!
//! ```compile_fail
//! use wyrd::authoring::WeaveBuilder;
//! ```
//!
//! Start with the ordered, executable [`examples`], beginning at
//! [`examples::tier_a::a01_hello_invert`].

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

extern crate alloc;
extern crate no_std_compat as std;

/// Allocation types used by exported declarative authoring macros.
///
/// This is public only so macro expansions remain portable to `no_std + alloc`
/// callers; it is not part of the supported hand-written API.
#[doc(hidden)]
pub mod __private {
    pub use crate::std::string::String;
    pub use crate::std::vec::Vec;
}

mod authoring;
mod foundation;
mod runtime_impl;

pub mod examples;

/// Shared signal, id, port, and knot-catalog vocabulary.
pub mod core {
    pub use crate::foundation::signal_ops;
    pub use crate::foundation::{
        from_count, from_level, is_truthy, port_domain, port_slot, ports_of, CalcOp, CompareOp,
        FlagPriority, HostTime, KnotId, KnotKind, NumericPath, PortDir, PortDomain, PortInfo,
        PortSlot, Seed, Signal, SignalDomain, ThreadId, TimerMode, ONE, ZERO,
    };
}

/// Weave authoring, validation, patterns, and serialization codecs.
pub mod graph {
    #[cfg(feature = "serde-json")]
    pub use crate::authoring::{from_json, to_json, JsonCodecError};
    #[cfg(feature = "serde-ron")]
    pub use crate::authoring::{from_ron, to_ron, RonCodecError};
    pub use crate::authoring::{
        slot_of, validate, validate_report, Bool, BoolWire, Budget, BudgetWarning, BuildError,
        ComposeError, Composer, Count, CountWire, InputPort, KnotDef, KnotHandle, Level, LevelWire,
        NumericWireDomain, OutputPort, Pattern, PatternDef, PatternExportDef, PatternInstance,
        PortRefDef, ThreadDef, ThresholdWires, ValidateReport, ValidationError, Weave,
        WeaveBuilder, WeaveDef, Wire, WireDomain,
    };
    pub use crate::runtime_impl::{
        EmitCommandManifest, RecipeManifest, SignalInManifest, SignalOutManifest,
    };
    /// JSON Schema traits and macros for recipe tooling.
    ///
    /// Available only with the opt-in `schema` feature, which also enables
    /// `std` and `serde` while preserving default and no_std dependencies.
    #[cfg(feature = "schema")]
    pub use schemars::{schema_for, JsonSchema};
}

/// Runtime binding, host integration, and output collection.
pub mod runtime {
    pub use crate::runtime_impl::{
        append_commands, outbox_to_commands, tick_once, BindError, BindOpts, BindRestoreError,
        CmdId, Emit, HandleError, Host, HostCommand, HostPathId, KnotHandle, NullHost, Outbox,
        PortWriter, PresetError, Recipe, RecipeEndpoint, RecipeError, RecipeInstance,
        RecipeResolveError, RestoreError, Runtime, RuntimePreset, RuntimePresetEntry, RuntimeState,
        RuntimeStateEntry, RuntimeStateReport, Scenario, ScenarioError, ScriptedHost, SenseId,
        SignalOutSample, RUNTIME_STATE_FORMAT_VERSION,
    };
}

pub use foundation::signal_ops;
pub use foundation::{
    from_count, from_level, is_truthy, port_domain, port_slot, ports_of, CalcOp, CompareOp,
    FlagPriority, HostTime, KnotId, KnotKind, NumericPath, PortDir, PortDomain, PortInfo, PortSlot,
    Seed, Signal, SignalDomain, ThreadId, TimerMode, ONE, ZERO,
};

pub use authoring::{
    slot_of, validate, validate_report, Bool, BoolWire, Budget, BudgetWarning, BuildError,
    ComposeError, Composer, Count, CountWire, InputPort, KnotDef, Level, LevelWire,
    NumericWireDomain, OutputPort, Pattern, PatternDef, PatternExportDef, PatternInstance,
    PortRefDef, ThreadDef, ThresholdWires, ValidateReport, ValidationError, Weave, WeaveBuilder,
    WeaveDef, Wire, WireDomain,
};

/// JSON Schema traits and macros for serializable graph and recipe types.
/// Enable the opt-in `schema` feature before importing these exports.
#[cfg(feature = "schema")]
pub use schemars::{schema_for, JsonSchema};

#[cfg(feature = "serde-ron")]
pub use authoring::{from_ron, to_ron, RonCodecError};
#[cfg(feature = "serde-ron")]
pub use runtime_impl::{runtime_state_from_ron, runtime_state_to_ron, RuntimeStateRonCodecError};

#[cfg(feature = "serde-json")]
pub use authoring::{from_json, to_json, JsonCodecError};
#[cfg(feature = "serde-json")]
pub use runtime_impl::{
    runtime_state_from_json, runtime_state_to_json, RuntimeStateJsonCodecError,
};

pub use runtime_impl::{
    append_commands, outbox_to_commands, tick_once, BindError, BindOpts, BindRestoreError, CmdId,
    Emit, EmitCommandManifest, HandleError, Host, HostCommand, HostPathId, NullHost, Outbox,
    PortWriter, PresetError, Recipe, RecipeEndpoint, RecipeError, RecipeInstance, RecipeManifest,
    RecipeResolveError, RestoreError, Runtime, RuntimePreset, RuntimePresetEntry, RuntimeState,
    RuntimeStateEntry, RuntimeStateReport, Scenario, ScenarioError, ScriptedHost, SenseId,
    SignalInManifest, SignalOutManifest, SignalOutSample, RUNTIME_STATE_FORMAT_VERSION,
};
