use super::super::super::{ServerConfig, WorldRepository};
use super::{governance_request, guest};
use tarrowyn_protocol::{GovernanceAction, PublicAction};

#[test]
fn governance_rejects_unbounded_or_controlled_proposal_targets() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository, "phase4-input-validation");
    for (request_id, target) in [
        ("long-proposal-target", "x".repeat(81)),
        ("controlled-proposal-target", "North\nroad".to_owned()),
    ] {
        let mut request = governance_request(GovernanceAction::Propose, request_id);
        request.public_action = Some(PublicAction::RepairRoad);
        request.target = Some(target);
        let error = repository
            .governance(&session.account_token, request)
            .expect_err("malformed proposal target should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, "invalid_proposal_target");
    }
}
