//! Host I/O surfaces for one loom frame: sense writes and act outbox.
//!
//! [`PortWriter`] is the only supported way to feed `SignalIn` senses on the
//! hot path. [`Outbox`] exposes dense `SignalOut` samples, capped emits, and
//! the exact number of emits rejected by the cap for host apply after settle.

use crate::foundation::{Signal, SignalDomain, ONE, ZERO};

use crate::runtime_impl::bind::Runtime;
use crate::runtime_impl::error::HandleError;
use crate::runtime_impl::handles::{CmdId, HostPathId, SenseId};

/// One `SignalOut` level written during the last loom.
#[derive(Copy, Clone, Debug)]
pub struct SignalOutSample {
    /// Dense id for the `SignalOut` host path (resolve once after bind).
    pub path: HostPathId,
    /// Signal level written by the loom this frame.
    pub value: Signal,
}

/// One `EmitCommand` entry written since the last [`Runtime::begin_frame`]
/// (subject to the per-frame emit cap).
#[derive(Copy, Clone, Debug)]
pub struct Emit {
    /// Dense id for the interned `EmitCommand` name.
    pub cmd: CmdId,
    /// Optional payload sampled from the emit knot's `payload` port.
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
        let domain = match self.rt.knots.get(i).map(|k| &k.kind) {
            Some(crate::foundation::KnotKind::SignalIn { domain }) => *domain,
            _ => return Err(HandleError::InvalidSense { sense: id }),
        };
        if !domain_value_is_valid(domain, value) {
            return Err(HandleError::DomainValue { sense: id, domain });
        }
        self.rt.sense_values[i] = value;
        Ok(())
    }
}

#[inline]
fn domain_value_is_valid(domain: SignalDomain, value: Signal) -> bool {
    match domain {
        SignalDomain::Bool => value == ZERO || value == ONE,
        SignalDomain::Level => {
            #[cfg(feature = "signal-f32")]
            {
                value.is_finite()
            }
            #[cfg(feature = "signal-i32")]
            {
                true
            }
        }
        SignalDomain::Count => {
            #[cfg(feature = "signal-f32")]
            {
                value.is_finite()
                    && value >= i32::MIN as f32
                    && value < 2_147_483_648.0
                    && value == (value as i32) as f32
            }
            #[cfg(feature = "signal-i32")]
            {
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::authoring::Weave;
    use crate::foundation::{KnotKind, ONE};

    use crate::runtime_impl::bind::{BindOpts, Runtime};

    #[test]
    fn domain_values_are_checked_at_the_host_boundary() {
        assert!(domain_value_is_valid(SignalDomain::Bool, ZERO));
        assert!(domain_value_is_valid(SignalDomain::Bool, ONE));
        assert!(!domain_value_is_valid(
            SignalDomain::Bool,
            crate::foundation::from_count(2)
        ));

        #[cfg(feature = "signal-f32")]
        {
            assert!(domain_value_is_valid(SignalDomain::Level, 0.25));
            assert!(!domain_value_is_valid(SignalDomain::Level, f32::NAN));
            assert!(domain_value_is_valid(SignalDomain::Count, 42.0));
            assert!(!domain_value_is_valid(SignalDomain::Count, 1.5));
            assert!(!domain_value_is_valid(
                SignalDomain::Count,
                -2_147_483_904.0,
            ));
            assert!(!domain_value_is_valid(SignalDomain::Count, 2_147_483_648.0));
        }

        #[cfg(feature = "signal-i32")]
        {
            assert!(domain_value_is_valid(SignalDomain::Level, i32::MAX));
            assert!(domain_value_is_valid(SignalDomain::Count, i32::MIN));
        }
    }

    #[test]
    fn set_sense_oob_returns_error_without_mutation() {
        let mut b = Weave::builder("x").unwrap();
        let _k_c = b
            .knot("c", KnotKind::constant(ONE, SignalDomain::Bool))
            .unwrap();
        let weave = b.build().unwrap();
        let mut rt = Runtime::bind(weave.clone(), BindOpts::default()).unwrap();
        let mut other_builder = Weave::builder("other").unwrap();
        let _sense = other_builder
            .knot("sense", KnotKind::signal_in(SignalDomain::Bool))
            .unwrap();
        let other = Runtime::bind(other_builder.build().unwrap(), BindOpts::default()).unwrap();
        let invalid = other.sense_id("sense").unwrap();
        assert_eq!(
            rt.port_writer().set_sense(invalid, ONE),
            Err(HandleError::ForeignRuntime { handle: "sense" })
        );
        assert!(rt.outbox().signals().is_empty());
        assert!(rt.outbox().emits().is_empty());
    }

    #[test]
    fn set_sense_rejects_same_runtime_non_sense_and_domain_values() {
        let mut b = Weave::builder("domain-errors").unwrap();
        let _constant = b
            .knot("constant", KnotKind::constant(ONE, SignalDomain::Bool))
            .unwrap();
        let _bool_sense = b
            .knot("bool", KnotKind::signal_in(SignalDomain::Bool))
            .unwrap();
        let _count_sense = b
            .knot("count", KnotKind::signal_in(SignalDomain::Count))
            .unwrap();
        let mut rt = Runtime::bind(b.build().unwrap(), BindOpts::default()).unwrap();

        // Knot insertion order is dense at bind: constant=0, bool=1, count=2.
        let constant_as_sense = SenseId::new(rt.owner, 0);
        assert_eq!(
            rt.port_writer().set_sense(constant_as_sense, ONE),
            Err(HandleError::InvalidSense {
                sense: constant_as_sense
            })
        );

        let bool_id = SenseId::new(rt.owner, 1);
        assert_eq!(
            rt.port_writer()
                .set_sense(bool_id, crate::foundation::from_count(2)),
            Err(HandleError::DomainValue {
                sense: bool_id,
                domain: SignalDomain::Bool,
            })
        );

        #[cfg(feature = "signal-f32")]
        {
            let count_id = SenseId::new(rt.owner, 2);
            assert_eq!(
                rt.port_writer().set_sense(count_id, 1.5),
                Err(HandleError::DomainValue {
                    sense: count_id,
                    domain: SignalDomain::Count,
                })
            );
        }

        #[cfg(feature = "signal-i32")]
        {
            let count_id = SenseId::new(rt.owner, 2);
            rt.port_writer().set_sense(count_id, 1).unwrap();
        }
    }
}
