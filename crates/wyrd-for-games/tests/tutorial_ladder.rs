//! Ordered tutorial ladder: Tier A → B → C.
//!
//! ```bash
//! cargo test -p wyrd-for-games --test tutorial_ladder
//! ```

use wyrd::cookbook::{tier_a, tier_b, tier_c, tier_d};

#[test]
fn a01_hello_invert() {
    tier_a::run_a01_hello_invert().unwrap();
}
#[test]
fn a02_two_plate_and() {
    tier_a::run_a02_two_plate_and().unwrap();
}
#[test]
fn a03_bind_sample_loom() {
    tier_a::run_a03_bind_sample_loom().unwrap();
}
#[test]
fn a04_host_tick_once() {
    tier_a::run_a04_host_tick_once().unwrap();
}
#[test]
fn a05_validate_fails() {
    tier_a::run_a05_validate_fails().unwrap();
}

#[test]
fn b01_monostable_pattern() {
    tier_b::run_b01_monostable_pattern().unwrap();
}
#[test]
fn b02_two_plate_door() {
    tier_b::run_b02_two_plate_door().unwrap();
}
#[test]
fn b03_flag_toggle() {
    tier_b::run_b03_flag_toggle().unwrap();
}
#[test]
fn b04_counter_threshold() {
    tier_b::run_b04_counter_threshold().unwrap();
}
#[test]
fn b05_delayed_pulse() {
    tier_b::run_b05_delayed_pulse().unwrap();
}

#[test]
fn c01_multi_switch_latch() {
    tier_c::run_c01_multi_switch_latch().unwrap();
}
#[test]
fn c02_timed_hold() {
    tier_c::run_c02_timed_hold().unwrap();
}
#[test]
fn c03_press_n_then_window() {
    tier_c::run_c03_press_n_then_window().unwrap();
}
#[test]
fn c04_button_cooldown() {
    tier_c::run_c04_button_cooldown().unwrap();
}
#[test]
fn c05_axis_digital() {
    tier_c::run_c05_axis_digital().unwrap();
}
#[test]
fn c06_map_remap() {
    tier_c::run_c06_map_remap().unwrap();
}
#[test]
fn c07_digitize_steps() {
    tier_c::run_c07_digitize_steps().unwrap();
}
#[test]
fn c08_on_start_once() {
    tier_c::run_c08_on_start_once().unwrap();
}
#[test]
fn c09_emit_once() {
    tier_c::run_c09_emit_once().unwrap();
}
#[test]
fn c10_or_any_of_keys() {
    tier_c::run_c10_or_any_of_keys().unwrap();
}

#[test]
fn d01_shrine_chamber() {
    tier_d::run_d01_shrine_chamber().unwrap();
}
