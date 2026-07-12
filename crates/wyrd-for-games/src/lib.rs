//! Engine-neutral signal-graph game logic for Wyrd.
//!
//! Author an immutable [`Weave`], bind it to dense ids with [`Runtime`], then
//! sample host senses and settle it once per frame. Use [`core`], [`graph`],
//! and [`runtime`] when an explicit layer namespace is clearer.

#![no_std]
#![forbid(unsafe_code)]

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
}

/// Runtime binding, host integration, output collection, and cookbook recipes.
pub mod runtime {
    pub use crate::runtime_impl::cookbook;
    pub use crate::runtime_impl::{
        append_commands, outbox_to_commands, tick_once, BindError, BindOpts, CmdId, CookbookError,
        Emit, HandleError, Host, HostCommand, HostPathId, KnotHandle, NullHost, Outbox, PortWriter,
        Recipe, RecipeEndpoint, RecipeError, RecipeInstance, RecipeResolveError, Runtime, Scenario,
        ScenarioError, ScriptedHost, SenseId, SignalOutSample,
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

#[cfg(feature = "serde-ron")]
pub use authoring::{from_ron, to_ron, RonCodecError};

#[cfg(feature = "serde-json")]
pub use authoring::{from_json, to_json, JsonCodecError};

pub use runtime_impl::cookbook;
pub use runtime_impl::{
    append_commands, outbox_to_commands, tick_once, BindError, BindOpts, CmdId, CookbookError,
    Emit, EmitCommandManifest, HandleError, Host, HostCommand, HostPathId, NullHost, Outbox,
    PortWriter, Recipe, RecipeEndpoint, RecipeError, RecipeInstance, RecipeManifest,
    RecipeResolveError, Runtime, Scenario, ScenarioError, ScriptedHost, SenseId, SignalInManifest,
    SignalOutManifest, SignalOutSample,
};
