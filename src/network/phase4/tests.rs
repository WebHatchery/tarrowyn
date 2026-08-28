use super::*;
use tarrowyn_protocol::ProfessionAction;

#[test]
fn crafting_challenge_moves_across_a_wide_target() {
    let mut client = Phase4Client::new();
    client.begin_crafting("service-order-1");
    let before = client.crafting_view().unwrap();
    advance_crafting(&mut client.crafting, 1.0);
    let after = client.crafting_view().unwrap();
    assert!(after.0 > before.0);
    assert_eq!(after.1, 0.38);
    assert_eq!(after.2, 0.66);
}

#[test]
fn crafting_tap_becomes_a_bounded_completion_request() {
    let mut client = Phase4Client::new();
    client.begin_crafting("service-order-2");
    advance_crafting(&mut client.crafting, 1.15);
    assert!(client.submit_crafting("craft-1".to_owned()));
    let Some(Phase4Command::Profession(request)) = client.commands.pop_front() else {
        panic!("crafting should queue a profession completion");
    };
    assert_eq!(request.action, ProfessionAction::CompleteOrder);
    assert_eq!(request.order_id.as_deref(), Some("service-order-2"));
    assert!(request.timing_score.is_some_and(|score| score <= 100));
}
