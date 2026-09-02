use super::super::models::{trim_replay_cache, RepositoryState};
use tarrowyn_protocol::{
    ApiResponse, FoundationResourceKind, FoundationStorehouseAction,
    FoundationStorehouseCompletion, FoundationStorehouseContribution,
    FoundationStorehouseContributionInput, FoundationStorehouseRequest,
    FoundationStorehouseResponse, FoundationStorehouseStage, FoundationStorehouseState,
    InfrastructureKind,
};

pub(crate) const MAX_CONTRIBUTIONS: usize = 64;
const FORMER_RESIDENT: &str = "former-resident";

impl super::super::WorldRepository {
    pub fn foundation_storehouse(
        &self,
        token: &str,
        request: FoundationStorehouseRequest,
    ) -> Result<ApiResponse<FoundationStorehouseResponse>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        let identity_key = super::super::authenticate(&mut state, token, &self.config)?;
        super::super::validate_request_id(&request.request_id)?;
        let landmark_id = super::super::validate_bounded_text(
            &request.landmark_id,
            160,
            "invalid_storehouse_landmark",
            "The storehouse landmark must be a bounded First Beacon fixture ID.",
        )?;
        if let Some(previous) = state
            .identities
            .get(&identity_key)
            .and_then(|identity| {
                identity
                    .foundation_storehouse_results
                    .get(&request.request_id)
            })
            .cloned()
        {
            return Ok(ApiResponse {
                meta: super::super::meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: previous,
            });
        }

        let mut response = FoundationStorehouseResponse {
            request_id: request.request_id.clone(),
            action: request.action,
            accepted: false,
            storehouse: state.foundation_activity.storehouse.clone(),
            player: super::super::player_projection(&state, &identity_key),
            reason: None,
        };
        let Some(position) = storehouse_landmark_position(&state, &landmark_id) else {
            response.reason = Some(
                "Use Mara, the First Beacon noticeboard, or the marked storehouse site.".to_owned(),
            );
            return finish(self, &mut state, identity_key, response);
        };
        let player_position = state
            .identities
            .get(&identity_key)
            .expect("identity exists")
            .position;
        if player_position.manhattan_distance(position) > 1 {
            response.reason = Some("Walk beside that storehouse landmark first.".to_owned());
            return finish(self, &mut state, identity_key, response);
        }

        match request.action {
            FoundationStorehouseAction::Inspect => {
                if request.contribution.is_some() {
                    response.reason =
                        Some("Inspection does not accept a contribution payload.".to_owned());
                } else {
                    response.accepted = true;
                }
            }
            FoundationStorehouseAction::Contribute => {
                if landmark_id == state.foundation_activity.storehouse.noticeboard_landmark_id {
                    response.reason = Some(
                        "Read the need here, then contribute beside Mara or the storehouse site."
                            .to_owned(),
                    );
                } else if state.foundation_activity.storehouse.completion.is_some() {
                    response.reason =
                        Some("The First Beacon storehouse is already built.".to_owned());
                } else if state.phase4.infrastructure.iter().any(|record| {
                    record.infrastructure_id
                        == state
                            .foundation_activity
                            .storehouse
                            .operational_infrastructure_id
                }) {
                    response.reason = Some(
                        "The infrastructure registry already contains this storehouse; support must reconcile the project ledger."
                            .to_owned(),
                    );
                } else if state.foundation_activity.storehouse.contributions.len()
                    >= MAX_CONTRIBUTIONS
                {
                    response.reason = Some(
                        "The bounded contribution ledger is full; support must archive it before more work."
                            .to_owned(),
                    );
                } else if let Some(input) = request.contribution {
                    apply_contribution(&mut state, &identity_key, input, &mut response);
                } else {
                    response.reason = Some("Choose materials or gold to contribute.".to_owned());
                }
            }
        }
        if response.accepted {
            super::journey::record_storehouse(
                &mut state,
                &identity_key,
                request.action,
                &request.request_id,
            );
        }
        finish(self, &mut state, identity_key, response)
    }
}

fn storehouse_landmark_position(
    state: &RepositoryState,
    landmark_id: &str,
) -> Option<tarrowyn_protocol::Position> {
    let project = &state.foundation_activity.storehouse;
    if ![
        project.builder_landmark_id.as_str(),
        project.noticeboard_landmark_id.as_str(),
        project.site_landmark_id.as_str(),
    ]
    .contains(&landmark_id)
    {
        return None;
    }
    crate::content::foundation_baseline()
        .landmarks
        .iter()
        .find(|landmark| landmark.id == landmark_id)
        .map(|landmark| landmark.position)
}

fn apply_contribution(
    state: &mut RepositoryState,
    identity_key: &str,
    input: FoundationStorehouseContributionInput,
    response: &mut FoundationStorehouseResponse,
) {
    let (kind, credited_units) = match input {
        FoundationStorehouseContributionInput::Material { kind, amount } => {
            if amount == 0 || amount > 99 {
                response.reason = Some("Contribute between 1 and 99 materials at once.".to_owned());
                return;
            }
            if !is_storehouse_material(kind) {
                response.reason = Some("This storehouse needs timber or stone.".to_owned());
                return;
            }
            let remaining = remaining_units(&state.foundation_activity.storehouse, kind);
            if amount > remaining {
                response.reason = Some(format!(
                    "Only {remaining} more {} can be credited.",
                    resource_label(kind)
                ));
                return;
            }
            let inventory = &state
                .identities
                .get(identity_key)
                .expect("identity exists")
                .inventory;
            if inventory_amount(inventory, kind) < amount {
                response.reason = Some(format!(
                    "You do not carry {amount} {}.",
                    resource_label(kind)
                ));
                return;
            }
            (kind, amount)
        }
        FoundationStorehouseContributionInput::Gold { toward, amount } => {
            if amount == 0 || amount > 10_000 {
                response.reason = Some("Contribute between 1 and 10,000 gold at once.".to_owned());
                return;
            }
            let Some(rate) = gold_rate(&state.foundation_activity.storehouse, toward) else {
                response.reason =
                    Some("Gold can replace only the listed timber or stone.".to_owned());
                return;
            };
            if amount % rate != 0 {
                response.reason = Some(format!(
                    "Mara credits {} at exactly {rate} gold per unit.",
                    resource_label(toward)
                ));
                return;
            }
            let units = amount / rate;
            let remaining = remaining_units(&state.foundation_activity.storehouse, toward);
            if units > remaining {
                response.reason = Some(format!(
                    "Only {remaining} more {} units can be funded.",
                    resource_label(toward)
                ));
                return;
            }
            if state
                .identities
                .get(identity_key)
                .expect("identity exists")
                .gold
                < amount
            {
                response.reason = Some(format!("You do not carry {amount} gold."));
                return;
            }
            (toward, units)
        }
    };

    let account_id = state
        .identities
        .get(identity_key)
        .expect("identity exists")
        .account_id
        .clone();
    let display_name = state
        .identities
        .get(identity_key)
        .expect("identity exists")
        .display_name
        .clone();
    let input_label = match input {
        FoundationStorehouseContributionInput::Material { amount, .. } => {
            format!("{amount} {}", resource_label(kind))
        }
        FoundationStorehouseContributionInput::Gold { amount, .. } => format!(
            "{amount} gold toward {credited_units} {}",
            resource_label(kind)
        ),
    };
    match input {
        FoundationStorehouseContributionInput::Material { amount, .. } => {
            *inventory_amount_mut(
                &mut state
                    .identities
                    .get_mut(identity_key)
                    .expect("identity exists")
                    .inventory,
                kind,
            ) -= amount;
        }
        FoundationStorehouseContributionInput::Gold { amount, .. } => {
            state
                .identities
                .get_mut(identity_key)
                .expect("identity exists")
                .gold -= amount;
        }
    }
    let new_stage = {
        let project = &mut state.foundation_activity.storehouse;
        project.revision = project.revision.saturating_add(1);
        project
            .contributions
            .push(FoundationStorehouseContribution {
                contribution_id: format!("storehouse-contribution-{}", project.revision),
                account_id,
                input,
                credited_kind: kind,
                credited_units,
                contributed_tick: state.tick,
            });
        project.current_stage = stage_for(project);
        project.current_stage
    };
    super::super::phase4::record(
        state,
        "storehouse contribution",
        "A resident supplies the First Beacon storehouse",
        &format!("{display_name} contributes {input_label} to Mara's project."),
    );
    if new_stage == FoundationStorehouseStage::Operational {
        complete_storehouse(state);
    }
    response.accepted = true;
}

fn complete_storehouse(state: &mut RepositoryState) {
    let project = &mut state.foundation_activity.storehouse;
    let mut contributors = Vec::new();
    for contribution in &project.contributions {
        if !contributors.contains(&contribution.account_id) {
            contributors.push(contribution.account_id.clone());
        }
    }
    let infrastructure_id = project.operational_infrastructure_id.clone();
    project.completion = Some(FoundationStorehouseCompletion {
        completed_tick: state.tick,
        contributor_account_ids: contributors,
        operational_infrastructure_id: infrastructure_id.clone(),
    });
    state
        .phase4
        .infrastructure
        .push(super::super::phase4::infrastructure(
            &infrastructure_id,
            "First Beacon storehouse",
            InfrastructureKind::PublicBuilding,
            tarrowyn_protocol::Position { x: 6, y: 7 },
            100,
            1,
            85,
            "Mara's completed storehouse keeps shared settlement goods dry and usable.",
        ));
    super::super::phase4::retain_recent(
        &mut state.phase4.infrastructure,
        super::super::phase4::MAX_INFRASTRUCTURE_RECORDS,
    );
    super::super::phase4::record(
        state,
        "storehouse completed",
        "The First Beacon raises its first shared storehouse",
        "Mara opens a weatherproof public storehouse built from attributed player contributions.",
    );
}

fn finish(
    repository: &super::super::WorldRepository,
    state: &mut RepositoryState,
    identity_key: String,
    mut response: FoundationStorehouseResponse,
) -> Result<ApiResponse<FoundationStorehouseResponse>, super::super::RepositoryError> {
    response.storehouse = state.foundation_activity.storehouse.clone();
    response.player = super::super::player_projection(state, &identity_key);
    let account_id = response.player.account_id.clone();
    super::super::phase6::audit_command(
        state,
        &account_id,
        "foundation.storehouse",
        &response.storehouse.project_id,
        response.accepted,
        "A proximity-checked storehouse inspection or contribution was recorded.",
    );
    let request_id = response.request_id.clone();
    let cache = &mut state
        .identities
        .get_mut(&identity_key)
        .expect("identity exists")
        .foundation_storehouse_results;
    cache.insert(request_id.clone(), response.clone());
    trim_replay_cache(cache);
    super::super::record_command_outcome(state, response.accepted);
    repository.persist(state)?;
    Ok(ApiResponse {
        meta: super::super::meta(state.tick, Some(request_id), Some(state.cursor)),
        data: response,
    })
}

pub(crate) fn stage_for(project: &FoundationStorehouseState) -> FoundationStorehouseStage {
    project
        .stages
        .iter()
        .filter(|gate| {
            gate.credited_units_required
                .iter()
                .all(|required| credited_units(project, required.kind) >= required.amount)
        })
        .map(|gate| gate.stage)
        .next_back()
        .unwrap_or(FoundationStorehouseStage::SiteMarked)
}

pub(super) fn remaining_units(
    project: &FoundationStorehouseState,
    kind: FoundationResourceKind,
) -> u32 {
    project
        .requirements
        .iter()
        .find(|requirement| requirement.kind == kind)
        .map(|requirement| {
            requirement
                .units_required
                .saturating_sub(credited_units(project, kind))
        })
        .unwrap_or(0)
}

fn credited_units(project: &FoundationStorehouseState, kind: FoundationResourceKind) -> u32 {
    project
        .contributions
        .iter()
        .filter(|contribution| contribution.credited_kind == kind)
        .fold(0_u32, |total, contribution| {
            total.saturating_add(contribution.credited_units)
        })
}

fn gold_rate(project: &FoundationStorehouseState, kind: FoundationResourceKind) -> Option<u32> {
    project
        .requirements
        .iter()
        .find(|requirement| requirement.kind == kind)
        .map(|requirement| requirement.gold_per_unit)
}

fn is_storehouse_material(kind: FoundationResourceKind) -> bool {
    matches!(
        kind,
        FoundationResourceKind::Timber | FoundationResourceKind::Stone
    )
}

fn resource_label(kind: FoundationResourceKind) -> &'static str {
    match kind {
        FoundationResourceKind::Timber => "timber",
        FoundationResourceKind::Stone => "stone",
        FoundationResourceKind::IronOre => "iron ore",
    }
}

fn inventory_amount(inventory: &tarrowyn_protocol::Inventory, kind: FoundationResourceKind) -> u32 {
    match kind {
        FoundationResourceKind::Timber => inventory.timber,
        FoundationResourceKind::Stone => inventory.stone,
        FoundationResourceKind::IronOre => inventory.iron_ore,
    }
}

fn inventory_amount_mut(
    inventory: &mut tarrowyn_protocol::Inventory,
    kind: FoundationResourceKind,
) -> &mut u32 {
    match kind {
        FoundationResourceKind::Timber => &mut inventory.timber,
        FoundationResourceKind::Stone => &mut inventory.stone,
        FoundationResourceKind::IronOre => &mut inventory.iron_ore,
    }
}

pub(crate) fn migrate_account(state: &mut RepositoryState, old: &str, new: &str) {
    replace_account_in_project(&mut state.foundation_activity.storehouse, old, new);
    for identity in state.identities.values_mut() {
        for response in identity.foundation_storehouse_results.values_mut() {
            replace_account_in_project(&mut response.storehouse, old, new);
        }
    }
}

pub(crate) fn remove_account(state: &mut RepositoryState, account_id: &str) {
    migrate_account(state, account_id, FORMER_RESIDENT);
}

fn replace_account_in_project(project: &mut FoundationStorehouseState, old: &str, new: &str) {
    for contribution in &mut project.contributions {
        if contribution.account_id == old {
            contribution.account_id = new.to_owned();
        }
    }
    if let Some(completion) = project.completion.as_mut() {
        for contributor in &mut completion.contributor_account_ids {
            if contributor == old {
                *contributor = new.to_owned();
            }
        }
        let mut unique = Vec::new();
        completion.contributor_account_ids.retain(|account| {
            if unique.contains(account) {
                false
            } else {
                unique.push(account.clone());
                true
            }
        });
    }
}

pub(crate) fn interaction_message(
    project: &FoundationStorehouseState,
    interaction_id: &str,
) -> Option<String> {
    if project.completion.is_some() {
        return match interaction_id {
            "speak-with-builder" => Some(
                "Mara: The First Beacon storehouse is open. Our shared goods finally have a dry roof."
                    .to_owned(),
            ),
            "read-local-needs" => Some(
                "LOCAL NEED COMPLETE — The First Beacon storehouse is operational.".to_owned(),
            ),
            "inspect-storehouse-site" => Some(
                "The completed storehouse stands here as a permanent public structure.".to_owned(),
            ),
            _ => None,
        };
    }
    let timber = remaining_units(project, FoundationResourceKind::Timber);
    let stone = remaining_units(project, FoundationResourceKind::Stone);
    match interaction_id {
        "speak-with-builder" => Some(format!(
            "Mara: The first storehouse still needs {timber} timber and {stone} stone. The noticeboard lists exact gold substitutes; bring goods or gold to me or the site."
        )),
        "read-local-needs" => Some(format!(
            "LOCAL NEED — First storehouse: {timber} timber (2 gold each) and {stone} stone (3 gold each) remain."
        )),
        "inspect-storehouse-site" => Some(format!(
            "{} — {timber} timber and {stone} stone remain before the storehouse can open.",
            project
                .stages
                .iter()
                .find(|gate| gate.stage == project.current_stage)
                .map(|gate| gate.visible_label.as_str())
                .unwrap_or("Marked storehouse site")
        )),
        _ => None,
    }
}
