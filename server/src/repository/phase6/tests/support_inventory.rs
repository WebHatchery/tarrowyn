use super::super::super::ServerConfig;
use super::super::super::WorldRepository;
use tarrowyn_protocol::{GuestSessionRequest, SupportRepairAction, SupportRepairRequest};

#[test]
fn support_repair_normalizes_every_persisted_inventory_counter() {
    let repository = WorldRepository::new(ServerConfig {
        support_operator_accounts: vec!["dev-account-1".to_owned()],
        ..ServerConfig::default()
    });
    let operator = repository
        .guest_session(GuestSessionRequest {
            client_key: Some("repair-inventory-operator".to_owned()),
            reset: false,
        })
        .unwrap()
        .data;
    {
        let mut state = repository.state.lock().unwrap();
        let identity = state
            .identities
            .get_mut("repair-inventory-operator")
            .expect("operator identity exists");
        identity.inventory.wheat = 20_000;
        identity.inventory.turnips = 20_000;
        identity.inventory.moonberries = 20_000;
        identity.inventory.seeds = 20_000;
        identity.inventory.bandages = 20_000;
    }
    let request = SupportRepairRequest {
        request_id: "repair-inventory-all-fields".to_owned(),
        action: SupportRepairAction::NormalizeInventory,
        account_id: Some(operator.account_id.clone()),
        target_id: None,
        note: "Clamp every persisted inventory counter to the support ceiling.".to_owned(),
    };
    let repaired = repository
        .support_repair(&operator.account_token, request.clone())
        .unwrap()
        .data;
    assert!(repaired.accepted);
    {
        let state = repository.state.lock().unwrap();
        let inventory = state
            .identities
            .get("repair-inventory-operator")
            .expect("operator identity remains present")
            .inventory;
        assert_eq!(inventory.wheat, 9_999);
        assert_eq!(inventory.turnips, 9_999);
        assert_eq!(inventory.moonberries, 9_999);
        assert_eq!(inventory.seeds, 9_999);
        assert_eq!(inventory.bandages, 9_999);
    }
    assert_eq!(
        repository
            .support_repair(&operator.account_token, request)
            .unwrap()
            .data,
        repaired
    );
}
