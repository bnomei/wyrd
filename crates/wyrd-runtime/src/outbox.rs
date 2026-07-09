use wyrd_core::{CmdId, HostPathId, KnotId, Signal};

use crate::bind::Runtime;

#[derive(Copy, Clone, Debug)]
pub struct SignalOutSample {
    pub path: HostPathId,
    pub value: Signal,
}

#[derive(Copy, Clone, Debug)]
pub struct Emit {
    pub cmd: CmdId,
    pub payload: Signal,
}

pub struct Outbox<'a> {
    pub(crate) signals: &'a [SignalOutSample],
    pub(crate) emits: &'a [Emit],
}

impl Outbox<'_> {
    pub fn signals(&self) -> &[SignalOutSample] {
        self.signals
    }
    pub fn emits(&self) -> &[Emit] {
        self.emits
    }
}

pub struct PortWriter<'a> {
    pub(crate) rt: &'a mut Runtime,
}

impl PortWriter<'_> {
    /// Hot path: dense KnotId only (D-id-space).
    #[inline]
    pub fn set_sense(&mut self, id: KnotId, value: Signal) {
        let i = id.0 as usize;
        if let Some(slot) = self.rt.sense_values.get_mut(i) {
            *slot = value;
        }
    }
}
