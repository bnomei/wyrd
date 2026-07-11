//! Compat: first five Weaves (Tier B) — bodies live in `wyrd::cookbook::tier_b`.

use wyrd::cookbook::tier_b;

#[test]
fn recipe_monostable_pattern() {
    tier_b::run_b01_monostable_pattern().unwrap();
}

#[test]
fn recipe_two_plate_door() {
    tier_b::run_b02_two_plate_door().unwrap();
}

#[test]
fn recipe_flag_toggle() {
    tier_b::run_b03_flag_toggle().unwrap();
}

#[test]
fn recipe_counter_threshold() {
    tier_b::run_b04_counter_threshold().unwrap();
}

#[test]
fn recipe_delayed_pulse() {
    tier_b::run_b05_delayed_pulse().unwrap();
}
