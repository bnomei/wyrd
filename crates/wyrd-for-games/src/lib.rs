//! Engine-neutral signal-graph game logic for Wyrd.
//!
//! Author an immutable [`Weave`], bind it to dense ids with [`Runtime`], then
//! sample host senses and settle it once per frame. Use [`core`], [`graph`],
//! and [`runtime`] when an explicit layer namespace is clearer.

#![no_std]
#![forbid(unsafe_code)]

mod authoring;
mod foundation;
mod runtime_impl;

/// Shared signal, id, port, and knot-catalog vocabulary.
pub mod core {
    pub use crate::foundation::{
        from_count, from_level, is_truthy, port_domain, port_slot, ports_of, CalcOp, CompareOp,
        FlagPriority, HostTime, KnotId, KnotKind, NumericPath, PortDir, PortDomain, PortInfo,
        PortSlot, Seed, Signal, SignalDomain, ThreadId, TimerMode, ONE, ZERO,
    };
    pub use crate::foundation::signal_ops;
}

/// Weave authoring, validation, patterns, and serialization codecs.
pub mod graph {
    pub use crate::authoring::{
        slot_of, validate, validate_report, Budget, BudgetWarning, BuildError, InputPort, KnotDef,
        KnotHandle, OutputPort, Pattern, PatternDef, PatternExportDef, PatternInstance, PortRefDef,
        ThreadDef, ValidateReport, ValidationError, Weave, WeaveBuilder, WeaveDef,
    };
    #[cfg(feature = "serde-json")]
    pub use crate::authoring::{from_json, to_json, JsonCodecError};
    #[cfg(feature = "serde-ron")]
    pub use crate::authoring::{from_ron, to_ron, RonCodecError};
}

/// Runtime binding, host integration, output collection, and cookbook recipes.
pub mod runtime {
    pub use crate::runtime_impl::{
        append_commands, outbox_to_commands, tick_once, BindError, BindOpts, CmdId,
        CookbookError, Emit, HandleError, Host, HostCommand, HostPathId, KnotHandle, NullHost,
        Outbox, PortWriter, Runtime, ScriptedHost, SenseId, SignalOutSample,
    };
    pub use crate::runtime_impl::cookbook;
}

pub use foundation::{
    from_count, from_level, is_truthy, port_domain, port_slot, ports_of, CalcOp, CompareOp,
    FlagPriority, HostTime, KnotId, KnotKind, NumericPath, PortDir, PortDomain, PortInfo,
    PortSlot, Seed, Signal, SignalDomain, ThreadId, TimerMode, ONE, ZERO,
};
pub use foundation::signal_ops;

pub use authoring::{
    slot_of, validate, validate_report, Budget, BudgetWarning, BuildError, InputPort, KnotDef,
    OutputPort, Pattern, PatternDef, PatternExportDef, PatternInstance, PortRefDef, ThreadDef,
    ValidateReport, ValidationError, Weave, WeaveBuilder, WeaveDef,
};

#[cfg(feature = "serde-ron")]
pub use authoring::{from_ron, to_ron, RonCodecError};

#[cfg(feature = "serde-json")]
pub use authoring::{from_json, to_json, JsonCodecError};

pub use runtime_impl::{
    append_commands, outbox_to_commands, tick_once, BindError, BindOpts, CmdId, CookbookError,
    Emit, HandleError, Host, HostCommand, HostPathId, NullHost, Outbox, PortWriter, Runtime,
    ScriptedHost, SenseId, SignalOutSample,
};
pub use runtime_impl::cookbook;
