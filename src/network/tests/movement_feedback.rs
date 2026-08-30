use super::*;

#[test]
fn movement_success_notice_names_the_authoritative_destination() {
    assert_eq!(
        super::super::commands::movement_success_notice(Position { x: 4, y: 7 }),
        "Moved to tile (4, 7)."
    );
}
