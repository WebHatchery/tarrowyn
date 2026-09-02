use super::{
    footprint_tiles, preview_for_footprint, stage_definition, RepositoryState, ServerConfig,
    MAX_CONDITION, MAX_PROPERTIES, MAX_PROPERTY_ID_CHARS,
};
use std::collections::HashSet;
use tarrowyn_protocol::{
    FoundationPropertyAction, FoundationPropertyRequest, FoundationPropertyState, Position,
};

pub(crate) fn movement_blocked(state: &RepositoryState, position: Position) -> bool {
    state.foundation_properties.iter().any(|property| {
        footprint_tiles(property.anchor, stage_definition(property.stage).footprint)
            .contains(&position)
    })
}

pub(crate) fn restore_properties(
    mut properties: Vec<FoundationPropertyState>,
) -> Vec<FoundationPropertyState> {
    properties.truncate(MAX_PROPERTIES);
    for property in &mut properties {
        property.revision = property.revision.max(1);
        property.condition = property.condition.clamp(1, MAX_CONDITION);
        property.storage_capacity = stage_definition(property.stage).storage_capacity;
    }
    properties
}

pub(crate) fn remove_account(state: &mut RepositoryState, account_id: &str) {
    state
        .foundation_properties
        .retain(|property| property.owner_account_id != account_id);
    for identity in state.identities.values_mut() {
        identity.foundation_property_results.clear();
    }
}

pub(crate) fn migrate_account(
    state: &mut RepositoryState,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    for property in &mut state.foundation_properties {
        migrate_property(property, old_account_id, new_account_id, new_display_name);
    }
    for identity in state.identities.values_mut() {
        for response in identity.foundation_property_results.values_mut() {
            for property in &mut response.projection.properties {
                if property.owner_account_id == old_account_id {
                    property.owner_account_id = new_account_id.to_owned();
                    property.owner_name = new_display_name.to_owned();
                }
            }
            if let Some(property) = response.projection.own_property.as_mut() {
                migrate_property(property, old_account_id, new_account_id, new_display_name);
            }
        }
    }
}

fn migrate_property(
    property: &mut FoundationPropertyState,
    old_account_id: &str,
    new_account_id: &str,
    new_display_name: &str,
) {
    if property.owner_account_id == old_account_id {
        property.owner_account_id = new_account_id.to_owned();
        property.owner_name = new_display_name.to_owned();
    }
}

pub(crate) fn integrity_ok(
    state: &RepositoryState,
    config: &ServerConfig,
    account_ids: &HashSet<&str>,
) -> bool {
    if state.foundation_properties.len() > MAX_PROPERTIES
        || state.next_property == 0
        || state
            .foundation_properties
            .iter()
            .map(|property| property.property_id.as_str())
            .collect::<HashSet<_>>()
            .len()
            != state.foundation_properties.len()
    {
        return false;
    }
    let mut owners = HashSet::new();
    for property in &state.foundation_properties {
        let definition = stage_definition(property.stage);
        let owner_identity_key = state.identities.iter().find_map(|(key, identity)| {
            (identity.account_id == property.owner_account_id).then_some(key.as_str())
        });
        if property.property_id.trim().is_empty()
            || property.property_id.chars().count() > MAX_PROPERTY_ID_CHARS
            || property.property_id.chars().any(char::is_control)
            || !account_ids.contains(property.owner_account_id.as_str())
            || !owners.insert(property.owner_account_id.as_str())
            || property.owner_name.trim().is_empty()
            || property.revision == 0
            || !(1..=MAX_CONDITION).contains(&property.condition)
            || property.storage_capacity != definition.storage_capacity
            || property.storage.total_items() > property.storage_capacity
            || owner_identity_key.is_none_or(|identity_key| {
                !preview_for_footprint(
                    state,
                    identity_key,
                    &FoundationPropertyRequest {
                        request_id: "integrity-check".to_owned(),
                        action: FoundationPropertyAction::PreviewPlacement,
                        property_id: Some(property.property_id.clone()),
                        anchor: Some(property.anchor),
                        entrance: Some(property.entrance),
                        access: None,
                        resource: None,
                        amount: 0,
                    },
                    config,
                    definition.footprint,
                    Some(property.property_id.as_str()),
                )
                .accepted
            })
        {
            return false;
        }
    }
    true
}
