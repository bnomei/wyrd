//! Public declarative cookbook topology remains buildable independently of its scenarios.

use wyrd::cookbook::{tier_a, tier_b, tier_c, tier_d};

#[test]
fn public_recipe_weaves_build() {
    assert!(tier_a::a01_hello_invert_weave().is_ok());
    assert!(tier_a::a02_two_plate_and_weave().is_ok());
    assert!(tier_a::a03_bind_sample_loom_weave().is_ok());
    assert!(tier_a::a04_host_tick_once_weave().is_ok());
    assert!(tier_b::b02_two_plate_door_weave().is_ok());
    assert!(tier_b::b03_flag_toggle_weave().is_ok());
    assert!(tier_b::b04_counter_threshold_weave().is_ok());
    assert!(tier_b::b05_delayed_pulse_weave().is_ok());
    assert!(tier_c::c01_multi_switch_latch_weave().is_ok());
    assert!(tier_c::c02_timed_hold_weave().is_ok());
    assert!(tier_c::c03_press_n_then_window_weave().is_ok());
    assert!(tier_c::c04_button_cooldown_weave().is_ok());
    assert!(tier_c::c05_axis_digital_weave().is_ok());
    assert!(tier_c::c06_map_remap_weave().is_ok());
    assert!(tier_c::c07_digitize_steps_weave().is_ok());
    assert!(tier_c::c08_on_start_once_weave().is_ok());
    assert!(tier_c::c09_emit_once_weave().is_ok());
    assert!(tier_c::c10_or_any_of_keys_weave().is_ok());
    assert!(tier_c::c11_composer_weave().is_ok());
    assert!(tier_d::d01_shrine_chamber_weave().is_ok());
}
