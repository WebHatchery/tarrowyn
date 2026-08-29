use super::*;

#[test]
fn lease_remaining_uses_a_visible_day_then_hour_countdown() {
    assert_eq!(lease_remaining(172_800, 0), "2d left");
    assert_eq!(lease_remaining(3_600, 0), "1h left");
    assert_eq!(lease_remaining(0, 0), "0h left");
}
