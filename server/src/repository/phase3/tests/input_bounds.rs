use super::*;
use tarrowyn_protocol::{
    ContractAction, ContractRequest, ExpeditionAction, ExpeditionRequest, ExpeditionRole,
};

#[test]
fn contract_selector_rejects_unbounded_or_controlled_ids() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository);

    for contract_id in ["x".repeat(161), "contract\nwith-control".to_owned()] {
        let error = repository
            .contract(
                &session.account_token,
                ContractRequest {
                    request_id: format!("contract-input-{}", contract_id.len()),
                    action: ContractAction::Accept,
                    contract_id,
                },
            )
            .expect_err("invalid contract selector should be rejected");
        assert_eq!(error.status, 400);
        assert_eq!(error.error.code, "invalid_contract_id");
    }
}

#[test]
fn expedition_selector_rejects_unbounded_or_controlled_ids() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = guest(&repository);
    let error = repository
        .expedition(
            &session.account_token,
            ExpeditionRequest {
                request_id: "expedition-input".to_owned(),
                action: ExpeditionAction::Join,
                expedition_id: Some("expedition\nwith-control".to_owned()),
                role: Some(ExpeditionRole::Scout),
                food: 0,
                tools: 0,
                materials: 0,
                safety: 0,
                outpost_name: None,
            },
        )
        .expect_err("invalid expedition selector should be rejected");
    assert_eq!(error.status, 400);
    assert_eq!(error.error.code, "invalid_expedition_id");
}
