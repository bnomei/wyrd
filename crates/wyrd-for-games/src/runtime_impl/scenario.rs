//! Closure-scoped typed execution scenarios for [`crate::Recipe`] examples.
//!
//! A scenario owns one bound recipe instance and advances it one frame at a
//! time. Frame writes and expectations take non-capturing selectors over the
//! current recipe's ports. Consequently a closure cannot pass a port captured
//! from another bound instance into a frame by accident.

use crate::foundation::{is_truthy, HostTime, Signal};
use crate::runtime_impl::error::ScenarioError;
use crate::runtime_impl::handles::{CmdId, HostPathId, SenseId};
use crate::runtime_impl::recipe::{Recipe, RecipeInstance};
use std::string::String;

/// A deterministic, closure-scoped runner for one bound [`Recipe`].
///
/// Use [`Scenario::run`] to bind and exercise a recipe without exposing raw
/// frame plumbing. Applications that need their own host loop can keep using
/// [`crate::Runtime`] and [`crate::ScriptedHost`] directly.
pub struct Scenario<R: Recipe> {
    instance: RecipeInstance<R>,
    next_tick: u64,
    last_tick: Option<u64>,
}

impl<R: Recipe> Scenario<R> {
    /// Bind `R`, run `scenario`, and drop the bound runtime when it returns.
    pub fn run<T>(
        run: impl FnOnce(&mut Self) -> Result<T, ScenarioError>,
    ) -> Result<T, ScenarioError> {
        let mut scenario = Self {
            instance: R::bind()?,
            next_tick: 0,
            last_tick: None,
        };
        run(&mut scenario)
    }

    /// Configure typed input writes and loom one deterministic frame.
    ///
    /// `write` receives a frame value whose selectors are always evaluated
    /// against this scenario's ports. Selectors are function pointers rather
    /// than arbitrary closures, preventing captured handles from another
    /// recipe instance from entering the frame.
    pub fn frame(
        &mut self,
        write: impl FnOnce(&mut Frame<'_, R>) -> Result<(), ScenarioError>,
    ) -> Result<(), ScenarioError> {
        let tick = self.next_tick;
        self.instance.runtime_mut().begin_frame(HostTime { tick });
        {
            let (ports, writer) = self.instance.port_writer_with_ports();
            let mut frame = Frame { ports, writer };
            write(&mut frame)?;
        }
        self.instance.runtime_mut().loom();
        self.last_tick = Some(tick);
        self.next_tick = self.next_tick.wrapping_add(1);
        Ok(())
    }

    /// Assert that the selected output is truthy in the last completed frame.
    pub fn expect_truthy(&self, path: fn(&R::Ports) -> HostPathId) -> Result<(), ScenarioError> {
        let path = path(self.instance.ports());
        let actual = self.signal_value(path)?;
        if is_truthy(actual) {
            Ok(())
        } else {
            Err(ScenarioError::ExpectedTruthy {
                path: self.path_name(path)?,
                actual,
                tick: self.current_tick(),
            })
        }
    }

    /// Assert that the selected output equals `expected` in the last frame.
    pub fn expect_value(
        &self,
        path: fn(&R::Ports) -> HostPathId,
        expected: Signal,
    ) -> Result<(), ScenarioError> {
        let path = path(self.instance.ports());
        let actual = self.signal_value(path)?;
        if actual == expected {
            Ok(())
        } else {
            Err(ScenarioError::UnexpectedSignal {
                path: self.path_name(path)?,
                expected,
                actual,
                tick: self.current_tick(),
            })
        }
    }

    /// Assert that the selected command emitted exactly `expected` times.
    pub fn expect_emits(
        &self,
        command: fn(&R::Ports) -> CmdId,
        expected: usize,
    ) -> Result<(), ScenarioError> {
        let command = command(self.instance.ports());
        let actual = self
            .instance
            .runtime()
            .outbox()
            .emits()
            .iter()
            .filter(|emit| emit.cmd == command)
            .count();
        if actual == expected {
            Ok(())
        } else {
            Err(ScenarioError::UnexpectedEmits {
                command: self.command_name(command)?,
                expected,
                actual,
                tick: self.current_tick(),
            })
        }
    }

    fn signal_value(&self, path: HostPathId) -> Result<Signal, ScenarioError> {
        self.instance
            .runtime()
            .outbox()
            .signals()
            .iter()
            .find(|sample| sample.path == path)
            .map(|sample| sample.value)
            .ok_or_else(|| ScenarioError::MissingSignal {
                path: self
                    .path_name(path)
                    .unwrap_or_else(|_| String::from("<invalid path>")),
                tick: self.current_tick(),
            })
    }

    fn path_name(&self, path: HostPathId) -> Result<String, ScenarioError> {
        Ok(String::from(self.instance.runtime().path_name(path)?))
    }

    fn command_name(&self, command: CmdId) -> Result<String, ScenarioError> {
        Ok(String::from(self.instance.runtime().cmd_name(command)?))
    }

    fn current_tick(&self) -> u64 {
        self.last_tick.unwrap_or(self.next_tick)
    }
}

/// Typed writes permitted while configuring one [`Scenario`] frame.
pub struct Frame<'a, R: Recipe> {
    ports: &'a R::Ports,
    writer: crate::PortWriter<'a>,
}

impl<R: Recipe> Frame<'_, R> {
    /// Write one recipe-owned `SignalIn` for this frame.
    ///
    /// The selector must be a non-capturing function over this recipe's ports,
    /// so a frame closure cannot reuse a handle from another recipe instance.
    pub fn set(
        &mut self,
        sense: fn(&R::Ports) -> SenseId,
        value: Signal,
    ) -> Result<(), ScenarioError> {
        self.writer.set_sense(sense(self.ports), value)?;
        Ok(())
    }
}
