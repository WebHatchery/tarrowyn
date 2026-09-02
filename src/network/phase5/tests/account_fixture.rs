pub(super) fn account_response(guest_fixture: bool) -> tarrowyn_protocol::AccountResponse {
    let account_id = if guest_fixture {
        "guest-1"
    } else {
        "account-1"
    };
    let character_id = if guest_fixture {
        "guest-character-1"
    } else {
        "character-1"
    };
    let display_name = if guest_fixture {
        "Guest"
    } else {
        "Linked traveller"
    };
    tarrowyn_protocol::AccountResponse {
        account_id: account_id.to_owned(),
        provider: if guest_fixture {
            "development-guest"
        } else {
            "webhatchery-identity-oidc"
        }
        .to_owned(),
        character_id: character_id.to_owned(),
        display_name: display_name.to_owned(),
        guest_fixture,
        privacy_policy_version: "2026-01".to_owned(),
        retention_note: "retained until deletion".to_owned(),
        session_expires_at_tick: 100,
        character: tarrowyn_protocol::PlayerProjection {
            account_id: account_id.to_owned(),
            character_id: character_id.to_owned(),
            display_name: display_name.to_owned(),
            position: tarrowyn_protocol::Position { x: 8, y: 6 },
            gold: 10,
            field_tool_condition: 20,
            field_tool_kind: tarrowyn_protocol::FoundationFieldToolKind::Crude,
            field_weather: tarrowyn_protocol::FieldWeather::Clear,
            field_pest_pressure: 0,
            animal_condition: 10,
            animal_max_condition: 10,
            skill: 1,
            reputation: 0,
            adventurer_rank: tarrowyn_protocol::AdventurerRank::Unproven,
            adventurer_credentials: Vec::new(),
            inventory: tarrowyn_protocol::Inventory::default(),
            weapon: tarrowyn_protocol::WeaponKind::IronSword,
            knocked_out: false,
            injuries: 0,
            recovery_cost: 0,
        },
    }
}
