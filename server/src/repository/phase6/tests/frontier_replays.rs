use super::*;
use tarrowyn_protocol::{
    AccountDeletionRequest, AuthLinkRequest, ClaimAction, ClaimRequest, ClaimStatus,
    ExpeditionAction, ExpeditionRequest, ExpeditionRole, ExpeditionStatus, GuestSessionRequest,
};

#[test]
fn account_deletion_anonymises_frontier_replays_kept_by_another_identity() {
    let repository = WorldRepository::new(ServerConfig::default());
    let owner_guest = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("frontier-replay-owner".to_owned()),
            reset: false,
        })
        .expect("owner guest session")
        .data;
    let owner = repository
        .auth_link(
            &owner_guest.account_token,
            AuthLinkRequest {
                request_id: "frontier-replay-owner-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "frontier-replay-owner-subject".to_owned(),
                display_name: Some("Frontier owner".to_owned()),
            },
        )
        .expect("owner link")
        .data;
    let observer = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("frontier-replay-observer".to_owned()),
            reset: false,
        })
        .expect("observer guest session")
        .data;

    let claim_request = ClaimRequest {
        request_id: "frontier-replay-claim-request".to_owned(),
        action: ClaimAction::Request,
    };
    repository
        .claim(&owner.session.account_token, claim_request)
        .expect("claim request");
    let claim_inspect = ClaimRequest {
        request_id: "frontier-replay-claim-inspect".to_owned(),
        action: ClaimAction::Inspect,
    };
    let inspected_claim = repository
        .claim(&observer.account_token, claim_inspect.clone())
        .expect("claim inspection")
        .data
        .claim
        .expect("inspected claim");
    assert_eq!(inspected_claim.owner_account_id, owner.account_id);

    repository
        .expedition(
            &owner.session.account_token,
            ExpeditionRequest {
                request_id: "frontier-replay-expedition-announce".to_owned(),
                action: ExpeditionAction::Announce,
                expedition_id: None,
                role: Some(ExpeditionRole::Scout),
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect("expedition announcement");
    let expedition_join = ExpeditionRequest {
        request_id: "frontier-replay-expedition-join".to_owned(),
        action: ExpeditionAction::Join,
        expedition_id: Some("pioneer-1".to_owned()),
        role: Some(ExpeditionRole::Farmer),
        food: 0,
        tools: 0,
        materials: 0,
        safety: 0,
        outpost_name: None,
    };
    let joined_expedition = repository
        .expedition(&observer.account_token, expedition_join.clone())
        .expect("expedition join")
        .data
        .expedition
        .expect("joined expedition");
    assert!(joined_expedition
        .members
        .iter()
        .any(|member| member.account_id == owner.account_id));

    repository
        .account_delete(
            &owner.session.account_token,
            AccountDeletionRequest {
                request_id: "frontier-replay-owner-delete".to_owned(),
                account_id: owner.account_id.clone(),
            },
        )
        .expect("schedule owner deletion");
    repository.tick();

    let claim_replay = repository
        .claim(&observer.account_token, claim_inspect)
        .expect("claim replay")
        .data
        .claim
        .expect("replayed claim");
    assert_eq!(claim_replay.owner_account_id, "former-resident");
    assert_eq!(claim_replay.owner_name, "Former resident");
    assert_eq!(claim_replay.status, ClaimStatus::Abandoned);

    let expedition_replay = repository
        .expedition(&observer.account_token, expedition_join)
        .expect("expedition replay")
        .data
        .expedition
        .expect("replayed expedition");
    assert!(!expedition_replay
        .members
        .iter()
        .any(|member| member.account_id == owner.account_id));
    assert_eq!(expedition_replay.leader_account_id, observer.account_id);
    assert_eq!(expedition_replay.members.len(), 1);
    assert_eq!(expedition_replay.status, ExpeditionStatus::Planning);
    assert!(repository.ops_health().data.ready);
}
