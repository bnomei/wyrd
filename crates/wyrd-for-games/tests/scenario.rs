use wyrd::{
    weave, BuildError, CmdId, HostPathId, KnotKind, Recipe, RecipeResolveError, Scenario,
    ScenarioError, SenseId, SignalDomain, Weave, ONE, ZERO,
};

struct Ports {
    button: SenseId,
    lamp: HostPathId,
    ping: CmdId,
}

struct ButtonRecipe;

impl Recipe for ButtonRecipe {
    type Ports = Ports;

    fn weave() -> Result<Weave, BuildError> {
        weave! {
            id: "scenario-button";
            knots {
                button = KnotKind::signal_in(SignalDomain::Bool);
                lamp = KnotKind::signal_out("lamp", SignalDomain::Bool);
                ping = KnotKind::emit_command("ping");
            }
            threads {
                button.out -> lamp.in;
                button.out -> ping.trigger;
            }
        }
    }

    fn resolve_ports(runtime: &wyrd::Runtime) -> Result<Self::Ports, RecipeResolveError> {
        Ok(Ports {
            button: runtime.required_sense("button")?,
            lamp: runtime.required_path("lamp")?,
            ping: runtime.required_command("ping")?,
        })
    }
}

#[test]
fn scenario_reports_missing_output_before_a_frame() {
    let error =
        Scenario::<ButtonRecipe>::run(|scenario| scenario.expect_truthy(|ports| ports.lamp))
            .expect_err("no frame has produced a lamp sample");

    assert!(matches!(
        error,
        ScenarioError::MissingSignal { path, tick: 0 } if path == "lamp"
    ));
}

#[test]
fn scenario_distinguishes_falsey_values_and_drives_multiple_frames() {
    Scenario::<ButtonRecipe>::run(|scenario| {
        scenario.frame(|frame| frame.set(|ports| ports.button, ZERO))?;
        scenario.expect_value(|ports| ports.lamp, ZERO)?;
        scenario.expect_emits(|ports| ports.ping, 0)?;

        scenario.frame(|frame| frame.set(|ports| ports.button, ONE))?;
        scenario.expect_truthy(|ports| ports.lamp)?;
        scenario.expect_emits(|ports| ports.ping, 1)?;

        scenario.frame(|frame| frame.set(|ports| ports.button, ONE))?;
        scenario.expect_truthy(|ports| ports.lamp)?;
        scenario.expect_emits(|ports| ports.ping, 0)?;
        Ok(())
    })
    .unwrap();
}

#[test]
fn scenario_value_errors_name_the_path_and_frame() {
    let error = Scenario::<ButtonRecipe>::run(|scenario| {
        scenario.frame(|frame| frame.set(|ports| ports.button, ZERO))?;
        scenario.expect_value(|ports| ports.lamp, ONE)
    })
    .expect_err("falsey lamp should not match ONE");

    assert!(matches!(
        error,
        ScenarioError::UnexpectedSignal { path, expected, actual, tick: 0 }
            if path == "lamp" && expected == ONE && actual == ZERO
    ));
}
