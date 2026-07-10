//! Host I/O surfaces for one loom frame: sense writes and act outbox.
//!
//! [`PortWriter`] is the only supported way to feed `SignalIn` senses on the
//! hot path. [`Outbox`] exposes dense `SignalOut` samples and capped emits for
//! host apply after settle.

use wyrd_core::{CmdId, HostPathId, SenseId, Signal};

use crate::bind::Runtime;
use crate::error::HandleError;

/// One `SignalOut` level written during the last loom.
#[derive(Copy, Clone, Debug)]
pub struct SignalOutSample {
    pub path: HostPathId,
    pub value: Signal,
}

/// One `EmitCommand` entry written during the last loom (subject to emit cap).
#[derive(Copy, Clone, Debug)]
pub struct Emit {
    pub cmd: CmdId,
    pub payload: Signal,
}

/// Borrowed view of this frame's acts after loom.
pub struct Outbox<'a> {
    pub(crate) signals: &'a [SignalOutSample],
    pub(crate) emits: &'a [Emit],
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
        let i = usize::from(id);
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
        let invalid = SenseId::try_from(999usize).unwrap();
        assert_eq!(
            rt.port_writer().set_sense(invalid, ONE),
            Err(HandleError::InvalidSense { sense: invalid })
        );
        assert!(rt.outbox().signals().is_empty());
        assert!(rt.outbox().emits().is_empty());
    }
}
