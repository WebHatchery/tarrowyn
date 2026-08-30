use super::*;
use tarrowyn_protocol::{AuthLinkResponse, AuthSession};

#[test]
fn linking_discards_in_flight_phase_four_guest_ledgers() {
    let mut client = Phase4Client::new();
    client.pending_governance = Some(Pending::failed("guest governance still in flight"));
    client.pending_claims = Some(Pending::failed("guest claims still in flight"));
    client.pending_professions = Some(Pending::failed("guest professions still in flight"));
    client.pending_knowledge = Some(Pending::failed("guest knowledge still in flight"));
    client.pending_skills = Some(Pending::failed("guest skills still in flight"));
    client.pending_households = Some(Pending::failed("guest households still in flight"));
    client.pending_combat = Some(Pending::failed("guest combat still in flight"));
    client.skills = Some(SkillsResponse {
        skills: Vec::new(),
        lessons: Vec::new(),
        cursor: 8,
    });
    client.crafting = Some(CraftingChallenge {
        order_id: "guest-order".to_owned(),
        progress: 0.5,
        direction: 1.0,
        target_start: 0.4,
        target_end: 0.6,
    });
    client.own_account_id = Some("guest-account".to_owned());
    client
        .regional
        .prime_linked_account_for_test(AuthLinkResponse {
            request_id: "link-phase4-race".to_owned(),
            provider: "webhatchery-identity-oidc".to_owned(),
            account_id: "account-1".to_owned(),
            character_id: "character-1".to_owned(),
            display_name: "Linked traveller".to_owned(),
            session: AuthSession {
                account_token: "prod-session-1".to_owned(),
                refresh_token: "prod-refresh-1".to_owned(),
                expires_in_seconds: 900,
                expires_at_tick: 3600,
            },
            linked_guest: true,
        });

    let linked = client
        .take_linked_account(Some("guest-key"))
        .expect("the linked response should be forwarded");

    assert_eq!(linked.account_id, "account-1");
    assert!(client.pending_governance.is_none());
    assert!(client.pending_claims.is_none());
    assert!(client.pending_professions.is_none());
    assert!(client.pending_knowledge.is_none());
    assert!(client.pending_skills.is_none());
    assert!(client.pending_households.is_none());
    assert!(client.pending_combat.is_none());
    assert!(client.skills.is_none());
    assert!(client.crafting.is_none());
    assert!(client.commands.is_empty());
    assert!(client.own_account_id.is_none());
}
