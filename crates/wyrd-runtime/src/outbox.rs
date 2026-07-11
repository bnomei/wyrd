//! Host I/O surfaces for one loom frame: sense writes and act outbox.
//!
//! [`PortWriter`] is the only supported way to feed `SignalIn` senses on the
//! hot path. [`Outbox`] exposes dense `SignalOut` samples, capped emits, and
//! the exact number of emits rejected by the cap for host apply after settle.

use wyrd_core::Signal;

use crate::bind::Runtime;
use crate::error::HandleError;
use crate::handles::{CmdId, HostPathId, SenseId};

/// One `SignalOut` level written during the last loom.
#[derive(Copy, Clone, Debug)]
pub struct SignalOutSample {
    pub path: HostPathId,
    pub value: Signal,
}

/// One `EmitCommand` entry written since the last [`Runtime::begin_frame`]
/// (subject to the per-frame emit cap).
#[derive(Copy, Clone, Debug)]
pub struct Emit {
    pub cmd: CmdId,
    pub payload: Signal,
}

/// Borrowed view of this frame's acts and dropped-emit telemetry after loom.
///
/// Retained emits stay in loom write order. [`Self::dropped_emits`] reports
/// how many later emits the configured cap rejected; both are reset by the
/// next [`Runtime::begin_frame`].
pub struct Outbox<'a> {
    pub(crate) signals: &'a [SignalOutSample],
    pub(crate) emits: &'a [Emit],
    pub(crate) dropped_emits: usize,
}

impl Outbox<'_> {
    /// SignalOut samples in loom write order.
    pub fn signals(&self) -> &[SignalOutSample] {
        self.signals
    }

    /// EmitCommand entries in loom write order (capped at bind).
    pub fn emits(&self) -> &[Emit] {
        self.emits
    }

    /// Number of emits rejected by the configured cap in this frame.
    ///
    /// The count saturates at [`usize::MAX`] and resets on
    /// [`Runtime::begin_frame`].
    pub fn dropped_emits(&self) -> usize {
        self.dropped_emits
    }
}

/// Mutable host handle for writing dense sense values before loom.
pub struct PortWriter<'a> {
    pub(crate) rt: &'a mut Runtime,
}

impl PortWriter<'_> {
    /// Write a host sense by dense id (must be a `SignalIn` knot).
    ///
    /// # Errors
    ///
    /// Returns [`HandleError::InvalidSense`] if the id is out of range or not a sense.
    #[inline]
    pub fn set_sense(&mut self, id: SenseId, value: Signal) -> Result<(), HandleError> {
        if id.owner != self.rt.owner {
            return Err(HandleError::ForeignRuntime { handle: "sense" });
        }
        let i = usize::from(id.index);
        if !matches!(
            self.rt.knots.get(i).map(|k| &k.kind),
            Some(wyrd_core::KnotKind::SignalIn)
        ) {
            return Err(HandleError::InvalidSense { sense: id });
        }
        self.rt.sense_values[i] = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wyrd_core::{KnotKind, ONE};
    use wyrd_graph::Weave;

    use crate::bind::{BindOpts, Runtime};

    #[test]
    fn set_sense_oob_returns_error_without_mutation() {
        let mut b = Weave::builder("x").unwrap();
        let _k_c = b.knot("c", KnotKind::constant(ONE)).unwrap();
        let weave = b.build().unwrap();
        let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
        let mut other_builder = Weave::builder("other").unwrap();
        let _sense = other_builder.knot("sense", KnotKind::signal_in()).unwrap();
        let other = Runtime::bind(other_builder.build().unwrap(), BindOpts::default()).unwrap();
        let invalid = other.sense_id("sense").unwrap();
        assert_eq!(
            rt.port_writer().set_sense(invalid, ONE),
            Err(HandleError::ForeignRuntime { handle: "sense" })
        );
        assert!(rt.outbox().signals().is_empty());
        assert!(rt.outbox().emits().is_empty());
    }
}
