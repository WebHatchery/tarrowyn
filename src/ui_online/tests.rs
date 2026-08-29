use super::*;

#[test]
fn recovery_risk_label_stays_compact_without_losing_the_seed_rule() {
    assert_eq!(
        recovery_risk_label("At most one carried seed is risked on knockout."),
        "1 carried seed"
    );
    assert_eq!(
        recovery_risk_label("A carried tool may be damaged."),
        "carried item"
    );
}
