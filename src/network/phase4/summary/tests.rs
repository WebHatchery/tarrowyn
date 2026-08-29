use super::*;

#[test]
fn lease_remaining_uses_a_visible_day_then_hour_countdown() {
    assert_eq!(lease_remaining(172_800, 0), "2d left");
    assert_eq!(lease_remaining(3_600, 0), "1h left");
    assert_eq!(lease_remaining(0, 0), "0h left");
}

#[test]
fn local_household_status_is_visible_in_the_settlement_summary() {
    let mut client = Phase4Client::new();
    client.households = Some(tarrowyn_protocol::HouseholdsResponse {
        households: vec![tarrowyn_protocol::HouseholdRecord {
            household_id: "bellweather".to_owned(),
            household_name: "The Bellweather household".to_owned(),
            members: Vec::new(),
            home: "The Hearth settlement".to_owned(),
            needs: Vec::new(),
            work: "Milling and healing".to_owned(),
            service_quality: 72,
            demand: 60,
            housing: 70,
            safety: 62,
            food: 68,
            competition: 20,
            status: tarrowyn_protocol::HouseholdLifeStatus::ReducedService,
            clue: "The road needs care.".to_owned(),
            last_decision_tick: 4,
        }],
        cursor: 4,
    });

    assert!(render(&client)
        .lines()
        .next()
        .is_some_and(|line| line.contains("Local life reduced service") && line.contains("72%")));
}
