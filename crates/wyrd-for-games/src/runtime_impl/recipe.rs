//! Reusable recipe contracts over validated weaves and bound runtime ports.

use std::string::String;
use std::vec::Vec;

use crate::authoring::{BuildError, Weave};
use crate::foundation::{KnotKind, NumericPath, SignalDomain};
use crate::runtime_impl::error::{RecipeError, RecipeResolveError};
use crate::runtime_impl::{BindOpts, Runtime};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A reusable graph with statically named, runtime-resolved ports.
///
/// `Ports` is application-defined and normally contains dense handle types such
/// as [`crate::SenseId`], [`crate::HostPathId`], and [`crate::CmdId`].
pub trait Recipe: Sized {
    /// Typed runtime handles required by this recipe's host.
    type Ports;

    /// Construct this recipe's validated weave.
    fn weave() -> Result<Weave, BuildError>;

    /// Resolve this recipe's typed ports from its freshly bound runtime.
    ///
    /// Implementations should use [`Runtime::required_sense`],
    /// [`Runtime::required_path`], and [`Runtime::required_command`] so a
    /// missing or incompatible endpoint retains its contextual name.
    fn resolve_ports(runtime: &Runtime) -> Result<Self::Ports, RecipeResolveError>;

    /// Build, bind, and resolve this recipe using default bind options.
    fn bind() -> Result<RecipeInstance<Self>, RecipeError> {
        Self::bind_with(BindOpts::default())
    }

    /// Build, bind, and resolve this recipe with explicit runtime bind options.
    fn bind_with(opts: BindOpts) -> Result<RecipeInstance<Self>, RecipeError> {
        let runtime = Runtime::bind(Self::weave()?, opts)?;
        let ports = Self::resolve_ports(&runtime)?;
        Ok(RecipeInstance { runtime, ports })
    }

    /// Derive this recipe's deterministic endpoint manifest from its weave.
    fn manifest() -> Result<RecipeManifest, RecipeError> {
        Ok(RecipeManifest::from_weave(&Self::weave()?))
    }
}

/// One bound [`Recipe`] with its runtime and typed resolved ports.
pub struct RecipeInstance<R: Recipe> {
    runtime: Runtime,
    ports: R::Ports,
}

impl<R: Recipe> RecipeInstance<R> {
    /// Borrow the executable runtime for advanced host integration.
    pub fn runtime(&self) -> &Runtime {
        &self.runtime
    }

    /// Mutably borrow the executable runtime for frame setup and loom.
    pub fn runtime_mut(&mut self) -> &mut Runtime {
        &mut self.runtime
    }

    /// Borrow the recipe's typed, runtime-owned ports.
    pub fn ports(&self) -> &R::Ports {
        &self.ports
    }

    /// Split this instance into its runtime and typed ports.
    pub fn into_parts(self) -> (Runtime, R::Ports) {
        (self.runtime, self.ports)
    }
}

/// Deterministic, tooling-facing summary of a validated recipe topology.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RecipeManifest {
    /// Validated weave id.
    pub weave_id: String,
    /// Numeric path declared by the weave.
    pub numeric: NumericPath,
    /// Host-writable `SignalIn` endpoints, ordered by knot id.
    pub signal_inputs: Vec<SignalInManifest>,
    /// Host-applied `SignalOut` endpoints, ordered by path then knot id.
    pub signal_outputs: Vec<SignalOutManifest>,
    /// Host-applied `EmitCommand` endpoints, ordered by name then knot id.
    pub emit_commands: Vec<EmitCommandManifest>,
}

impl RecipeManifest {
    /// Extract a manifest from an already validated weave.
    pub fn from_weave(weave: &Weave) -> Self {
        let mut signal_inputs = Vec::new();
        let mut signal_outputs = Vec::new();
        let mut emit_commands = Vec::new();

        for knot in weave.knots() {
            match &knot.kind {
                KnotKind::SignalIn { domain } => signal_inputs.push(SignalInManifest {
                    knot: knot.id.clone(),
                    domain: *domain,
                }),
                KnotKind::SignalOut { path, domain } => signal_outputs.push(SignalOutManifest {
                    knot: knot.id.clone(),
                    path: path.clone(),
                    domain: *domain,
                }),
                KnotKind::EmitCommand { name } => emit_commands.push(EmitCommandManifest {
                    knot: knot.id.clone(),
                    name: name.clone(),
                }),
                _ => {}
            }
        }

        signal_inputs.sort_by(|left, right| left.knot.cmp(&right.knot));
        signal_outputs.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| left.knot.cmp(&right.knot))
        });
        emit_commands.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.knot.cmp(&right.knot))
        });

        Self {
            weave_id: String::from(weave.id()),
            numeric: weave.numeric(),
            signal_inputs,
            signal_outputs,
            emit_commands,
        }
    }
}

/// A named host-writable `SignalIn` endpoint in a [`RecipeManifest`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SignalInManifest {
    /// Author knot id used to resolve the [`crate::SenseId`].
    pub knot: String,
    /// Required host signal domain.
    pub domain: SignalDomain,
}

/// A named host-applied `SignalOut` endpoint in a [`RecipeManifest`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct SignalOutManifest {
    /// Author knot id associated with this output path.
    pub knot: String,
    /// Host path resolved to a [`crate::HostPathId`] after bind.
    pub path: String,
    /// Signal domain sent to the host.
    pub domain: SignalDomain,
}

/// A named host-applied `EmitCommand` endpoint in a [`RecipeManifest`].
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EmitCommandManifest {
    /// Author knot id associated with this command.
    pub knot: String,
    /// Command name resolved to a [`crate::CmdId`] after bind.
    pub name: String,
}
