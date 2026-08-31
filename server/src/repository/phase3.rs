use super::*;
use crate::config::ServerConfig;
use crate::repository::models::RepositoryState;
use tarrowyn_protocol::{
    AdventurerContract, ChronicleEntry, ChronicleResponse, ChronicleSummary, ClaimStatus,
    CombatAction, CombatOutcome, CombatRequest, CombatResponse, ContractAction, ContractRequest,
    ContractResponse, ContractStatus, ContractsResponse, ExpeditionMember, ExpeditionRole,
    ExpeditionStatus, FrontierEvent, HouseholdStatus, OpportunitiesResponse, PlayerProjection,
    Position, WeaponKind, WorldEvent,
};

const CONTRACT_ID: &str = "brambleback-watch";
const MAX_CHRONICLE_SUMMARY_KINDS: usize = 12;
mod state;
#[cfg(test)]
pub(crate) use state::MAX_CHRONICLE;
pub(super) use state::MAX_EXPEDITION_MEMBERS;
pub(super) use state::{
    archive_excess, fresh, normalize_opportunity_score, trim_expedition_members, ContractProgress,
    Phase3Response, Phase3State,
};

#[cfg(test)]
mod tests;

pub(super) fn tick(state: &mut RepositoryState, config: &ServerConfig) {
    for progress in state.phase3.contracts.values_mut() {
        if progress.status == ContractStatus::Cooldown && progress.available_at_tick <= state.tick {
            progress.status = ContractStatus::Available;
            progress.progress = 0;
        }
    }
    if state.phase3.zone.threat_active {
        state.phase3.unmet_demand_ticks = state.phase3.unmet_demand_ticks.saturating_add(1);
        state.phase3.poor_condition_ticks = state.phase3.poor_condition_ticks.saturating_add(1);
        state.phase3.zone.price_modifier_percent = 20;
    } else {
        state.phase3.poor_condition_ticks = 0;
        state.phase3.zone.price_modifier_percent = 0;
    }
    let mut transition = None;
    let threat_active = state.phase3.zone.threat_active;
    let unmet_demand_ticks = state.phase3.unmet_demand_ticks;
    let poor_condition_ticks = state.phase3.poor_condition_ticks;
    if let Some(household) = state.phase3.households.first_mut() {
        household.opportunity_score = if threat_active {
            household.opportunity_score.saturating_sub(1).max(0)
        } else {
            household.opportunity_score.saturating_add(2).min(100)
        };
        if unmet_demand_ticks >= 3 && household.status == HouseholdStatus::Travelling {
            household.status = HouseholdStatus::Candidate;
            household.clue = "The mender may stay if the road demand remains real.".to_owned();
            transition = Some(("arrival candidate", household.clue.clone()));
        } else if unmet_demand_ticks >= 5 && household.status == HouseholdStatus::Candidate {
            household.status = HouseholdStatus::Arrived;
            household.clue =
                "The Maren household has opened a small repair bench at the Hearth.".to_owned();
            transition = Some(("household arrival", household.clue.clone()));
        } else if poor_condition_ticks >= 12 && household.status == HouseholdStatus::Arrived {
            household.status = HouseholdStatus::Departed;
            household.clue =
                "The repair bench is shuttered; the Marens left for safer roads.".to_owned();
            transition = Some(("household departure", household.clue.clone()));
        }
    }
    if let Some((kind, clue)) = transition {
        record(
            state,
            kind,
            "A household decision reaches the Hearth",
            &clue,
        );
        add_notice(state, "household", &clue);
        if let Some(household) = state.phase3.households.first().cloned() {
            push_event(
                state,
                WorldEvent::Frontier(FrontierEvent::Opportunity(household)),
            );
        }
    }
    let mut reclaimed = None;
    if let Some(claim) = state.phase3.claim.as_mut() {
        if claim.status == ClaimStatus::Active
            && state.tick.saturating_sub(claim.last_active_tick) > claim.reclaim_after_ticks
        {
            claim.status = ClaimStatus::Reclaimed;
            reclaimed = Some(claim.clone());
        }
    }
    if let Some(claim) = reclaimed {
        record(
            state,
            "claim reclaimed",
            "An unattended homestead returns to the frontier",
            "A lease went quiet long enough for the settlement to reclaim it.",
        );
        push_event(state, WorldEvent::Frontier(FrontierEvent::Claim(claim)));
    }
    let expedition_on_road = state
        .phase3
        .expedition
        .as_ref()
        .is_some_and(|expedition| expedition.status == ExpeditionStatus::Launched);
    if expedition_on_road && state.tick.is_multiple_of(4) {
        add_notice(
            state,
            "expedition",
            "The pioneer party is on the road; return to the registry to resolve its fate.",
        );
    }
    let _ = config;
}

pub(super) fn movement_blocked(phase: &Phase3State, next: Position) -> bool {
    phase.zone.threat_active && next.x > phase.zone.position.x
}

pub(super) fn harvest_price_bonus(phase: &Phase3State, base: u32) -> u32 {
    if phase.zone.price_modifier_percent <= 0 {
        base
    } else {
        base.saturating_mul(100 + phase.zone.price_modifier_percent as u32) / 100
    }
}

pub(super) fn rumours(phase: &Phase3State) -> Vec<String> {
    let mut rumours = vec![phase.zone.rumour.clone()];
    if let Some(household) = phase.households.first() {
        rumours.push(household.clue.clone());
    }
    if let Some(expedition) = &phase.expedition {
        rumours.push(format!(
            "{} is {:?} beyond the first road.",
            expedition.outpost_name, expedition.status
        ));
    }
    rumours
}

impl WorldRepository {
    pub fn contracts(
        &self,
        token: &str,
    ) -> Result<ApiResponse<ContractsResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: ContractsResponse {
                contracts: vec![contract_view(&state, &key)],
                cursor: state.cursor,
            },
        })
    }

    pub fn contract(
        &self,
        token: &str,
        request: ContractRequest,
    ) -> Result<ApiResponse<ContractResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let contract_id = validate_bounded_text(
            &request.contract_id,
            160,
            "invalid_contract_id",
            "A contract selector must be bounded and contain no control characters.",
        )?;
        let cache_key = cache_key(&key, &request.request_id);
        if let Some(Phase3Response::Contract(response)) =
            state.phase3.request_results.get(&cache_key)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let mut response = ContractResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            contract: contract_view(&state, &key),
            player: projection_for(&state, &key),
            reason: None,
        };
        state
            .phase3
            .contracts
            .entry(key.clone())
            .or_insert(ContractProgress {
                progress: 0,
                status: ContractStatus::Available,
                completion_count: 0,
                available_at_tick: 0,
            });
        let current = state
            .phase3
            .contracts
            .get(&key)
            .expect("contract exists")
            .clone();
        if contract_id != CONTRACT_ID {
            response.reason = Some("That contract is not written in the tavern ledger.".to_owned());
        } else {
            match request.action {
                ContractAction::Accept if current.status == ContractStatus::Available => {
                    let progress = state
                        .phase3
                        .contracts
                        .get_mut(&key)
                        .expect("contract exists");
                    progress.status = ContractStatus::Accepted;
                    progress.progress = 0;
                    response.accepted = true;
                    record(
                        &mut state,
                        "contract accepted",
                        "The Brambleback watch is taken up",
                        "An adventurer has accepted the repeatable watch contract.",
                    );
                }
                ContractAction::Progress if current.status == ContractStatus::Accepted => {
                    let position = state
                        .identities
                        .get(&key)
                        .expect("identity exists")
                        .position;
                    if position.x < 10 {
                        response.reason =
                            Some("Take the north road before reporting field work.".to_owned());
                    } else {
                        let progress = state
                            .phase3
                            .contracts
                            .get_mut(&key)
                            .expect("contract exists");
                        let required_progress =
                            crate::content::contract_template(CONTRACT_ID).required_progress;
                        progress.progress =
                            progress.progress.saturating_add(1).min(required_progress);
                        response.accepted = true;
                        super::skills::record_practice(&mut state, &key, "navigation");
                    }
                }
                ContractAction::Report
                    if current.status == ContractStatus::Accepted
                        && current.progress
                            >= crate::content::contract_template(CONTRACT_ID).required_progress =>
                {
                    super::skills::record_practice(&mut state, &key, "survival");
                    let template = crate::content::contract_template(CONTRACT_ID);
                    let reward = template
                        .reward_gold
                        .saturating_add(current.completion_count.saturating_mul(2));
                    let identity = state.identities.get_mut(&key).expect("identity exists");
                    identity.gold = identity.gold.saturating_add(reward);
                    identity.skill = identity.skill.saturating_add(1);
                    identity.reputation = identity.reputation.saturating_add(1);
                    let available_at_tick = state.tick.saturating_add(2);
                    let progress = state
                        .phase3
                        .contracts
                        .get_mut(&key)
                        .expect("contract exists");
                    progress.status = ContractStatus::Cooldown;
                    progress.available_at_tick = available_at_tick;
                    progress.completion_count = progress.completion_count.saturating_add(1);
                    response.accepted = true;
                    record(
                        &mut state,
                        "contract reported",
                        "The watcher's report is entered in the chronicle",
                        "The tavern paid for a completed Brambleback watch.",
                    );
                }
                _ => response.reason = Some(contract_reason(current.status, request.action)),
            }
        }
        response.contract = contract_view(&state, &key);
        response.player = projection_for(&state, &key);
        state
            .phase3
            .request_results
            .insert(cache_key, Phase3Response::Contract(response.clone()));
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }

    pub fn combat(
        &self,
        token: &str,
        request: CombatRequest,
    ) -> Result<ApiResponse<CombatResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        let key = authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let cache_key = cache_key(&key, &request.request_id);
        if let Some(Phase3Response::Combat(response)) = state.phase3.request_results.get(&cache_key)
        {
            return Ok(ApiResponse {
                meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let mut response = CombatResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            outcome: None,
            monster: state.phase3.zone.monster,
            player: projection_for(&state, &key),
            zone: state.phase3.zone.clone(),
            recovery_prompt: None,
            reason: None,
        };
        let position = state
            .identities
            .get(&key)
            .expect("identity exists")
            .position;
        let knocked_out = state
            .identities
            .get(&key)
            .expect("identity exists")
            .knocked_out;
        if knocked_out {
            response.reason =
                Some("You are knocked out; tap Self, Rescuer, or Healer below.".to_owned());
        } else if position.manhattan_distance(state.phase3.zone.position) > 2 {
            response.reason =
                Some("Stand near Whisperwood Edge before facing the threat.".to_owned());
        } else if !state.phase3.zone.threat_active {
            response.reason = Some("The Brambleback threat is already quiet.".to_owned());
        } else {
            response.accepted = true;
            match request.action {
                CombatAction::Retreat => response.outcome = Some(CombatOutcome::Retreated),
                CombatAction::Strike if request.weapon == WeaponKind::IronSword => {
                    state.phase3.zone.threat_active = false;
                    state.phase3.zone.road_open = true;
                    state.phase3.zone.monster_health = 0;
                    state.phase3.zone.price_modifier_percent = 0;
                    let identity = state.identities.get_mut(&key).expect("identity exists");
                    identity.gold = identity.gold.saturating_add(5);
                    identity.skill = identity.skill.saturating_add(2);
                    response.outcome = Some(CombatOutcome::Victory);
                    record(
                        &mut state,
                        "threat defeated",
                        "The north road opens after the Brambleback falls",
                        "A prepared adventurer cleared Whisperwood Edge and steadied the road.",
                    );
                    add_notice(
                        &mut state,
                        "frontier",
                        "The Brambleback is down; road travel and prices begin to recover.",
                    );
                    let zone = state.phase3.zone.clone();
                    push_event(
                        &mut state,
                        WorldEvent::Frontier(FrontierEvent::Threat(zone)),
                    );
                }
                CombatAction::Strike => {
                    let identity = state.identities.get_mut(&key).expect("identity exists");
                    identity.knocked_out = true;
                    identity.injuries = identity.injuries.saturating_add(1).min(3);
                    identity.recovery_cost = 3;
                    identity.position = Position { x: 8, y: 5 };
                    if identity.inventory.seeds > 0 {
                        identity.inventory.seeds -= 1;
                    }
                    response.outcome = Some(CombatOutcome::KnockedOut);
                    response.recovery_prompt = Some(
                        "You were knocked out. Tap Self, Rescuer, or Healer; stored goods remain safe."
                            .to_owned(),
                    );
                    record(
                        &mut state,
                        "knockout",
                        "The Brambleback sends a traveller back to the Hearth",
                        "The road is still dangerous; a small carried loss marks the retreat.",
                    );
                    let presence_event = {
                        let identity = state.identities.get(&key).expect("identity exists");
                        WorldEvent::Presence(presence(identity, state.tick, true))
                    };
                    push_event(&mut state, presence_event);
                }
            }
        }
        response.player = projection_for(&state, &key);
        response.zone = state.phase3.zone.clone();
        state
            .phase3
            .request_results
            .insert(cache_key, Phase3Response::Combat(response.clone()));
        record_command_outcome(&mut state, response.accepted);
        self.persist(&state);
        Ok(ApiResponse {
            meta: meta(state.tick, Some(request.request_id), Some(state.cursor)),
            data: response,
        })
    }

    pub fn chronicle(
        &self,
        token: &str,
        since: u64,
    ) -> Result<ApiResponse<ChronicleResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        authenticate(&mut state, token, &self.config)?;
        super::validate_event_cursor(&state, since, "chronicle")?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: ChronicleResponse {
                entries: state
                    .phase3
                    .chronicle
                    .iter()
                    .filter(|entry| entry.cursor > since)
                    .cloned()
                    .collect(),
                summary: chronicle_summary(&state.phase3.chronicle_archive, since),
                cursor: state.cursor,
            },
        })
    }

    pub fn opportunities(
        &self,
        token: &str,
    ) -> Result<ApiResponse<OpportunitiesResponse>, RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state);
        authenticate(&mut state, token, &self.config)?;
        Ok(ApiResponse {
            meta: meta(state.tick, None, Some(state.cursor)),
            data: OpportunitiesResponse {
                opportunities: state.phase3.households.clone(),
                cursor: state.cursor,
            },
        })
    }
}

pub(super) fn cache_key(account: &str, request_id: &str) -> String {
    format!("{account}:{request_id}")
}

pub(super) fn is_request_cache_for_identity(
    key: &str,
    identity_key: &str,
    response: &Phase3Response,
) -> bool {
    key == format!("{identity_key}:{}", response_request_id(response))
}

fn response_request_id(response: &Phase3Response) -> &str {
    match response {
        Phase3Response::Contract(response) => &response.request_id,
        Phase3Response::Combat(response) => &response.request_id,
        Phase3Response::Recovery(response) => &response.request_id,
        Phase3Response::Claim(response) => &response.request_id,
        Phase3Response::Expedition(response) => &response.request_id,
    }
}

fn projection_for(state: &RepositoryState, key: &str) -> PlayerProjection {
    super::player_projection(state, key)
}

fn contract_view(state: &RepositoryState, key: &str) -> AdventurerContract {
    let template = crate::content::contract_template(CONTRACT_ID);
    let progress = state
        .phase3
        .contracts
        .get(key)
        .cloned()
        .unwrap_or(ContractProgress {
            progress: 0,
            status: ContractStatus::Available,
            completion_count: 0,
            available_at_tick: 0,
        });
    AdventurerContract {
        contract_id: template.id,
        title: template.title,
        description: template.description,
        target: template.target,
        progress: progress.progress,
        required_progress: template.required_progress,
        reward_gold: template
            .reward_gold
            .saturating_add(progress.completion_count.saturating_mul(2)),
        status: progress.status,
        completion_count: progress.completion_count,
        available_at_tick: progress.available_at_tick,
    }
}

fn contract_reason(status: ContractStatus, action: ContractAction) -> String {
    match (status, action) {
        (ContractStatus::Cooldown, _) => {
            "The contract is repeatable, but the tavern needs a moment before posting it again."
                .to_owned()
        }
        (ContractStatus::Accepted, ContractAction::Accept) => {
            "You already carry this contract.".to_owned()
        }
        (ContractStatus::Available, ContractAction::Progress) => {
            "Accept the contract at the tavern first.".to_owned()
        }
        (ContractStatus::Accepted, ContractAction::Report) => {
            "Three signs are needed before the tavern can pay the report.".to_owned()
        }
        _ => "That contract action is not available yet.".to_owned(),
    }
}

pub(super) fn has_roles(members: &[ExpeditionMember]) -> bool {
    [
        ExpeditionRole::Scout,
        ExpeditionRole::Farmer,
        ExpeditionRole::Builder,
    ]
    .into_iter()
    .all(|role| members.iter().any(|member| member.role == role))
}

pub(super) fn frontier_event(state: &mut RepositoryState, event: FrontierEvent) {
    push_event(state, WorldEvent::Frontier(event));
}

pub(super) fn record(state: &mut RepositoryState, kind: &str, title: &str, text: &str) {
    let event_id = format!("chronicle-{}", state.phase3.next_event_id);
    state.phase3.next_event_id = state.phase3.next_event_id.saturating_add(1);
    let mut entry = ChronicleEntry {
        event_id,
        kind: kind.to_owned(),
        title: title.to_owned(),
        text: text.to_owned(),
        created_tick: state.tick,
        cursor: 0,
    };
    let cursor = push_event(state, WorldEvent::Chronicle(entry.clone()));
    entry.cursor = cursor;
    if let Some(EventRecord {
        event: WorldEvent::Chronicle(stored),
        ..
    }) = state.events.back_mut()
    {
        *stored = entry.clone();
    }
    state.phase3.chronicle.push_back(entry);
    archive_excess(&mut state.phase3);
}

pub(super) fn chronicle_entries<'a>(
    phase: &'a Phase3State,
) -> impl Iterator<Item = &'a ChronicleEntry> + 'a {
    phase.chronicle_archive.iter().chain(phase.chronicle.iter())
}

pub(super) fn chronicle_summary(
    entries: &[ChronicleEntry],
    since: u64,
) -> Option<ChronicleSummary> {
    let entries: Vec<&ChronicleEntry> = entries
        .iter()
        .filter(|entry| entry.cursor > since)
        .collect();
    let first = entries.first()?;
    let last = entries.last()?;
    let mut kinds = Vec::new();
    for kind in entries.iter().map(|entry| entry.kind.as_str()) {
        if kinds.len() < MAX_CHRONICLE_SUMMARY_KINDS
            && !kinds.iter().any(|existing| existing == kind)
        {
            kinds.push(kind.to_owned());
        }
    }
    let highlights = entries
        .iter()
        .rev()
        .take(3)
        .rev()
        .map(|entry| entry.title.clone())
        .collect();
    Some(ChronicleSummary {
        from_tick: first.created_tick,
        to_tick: last.created_tick,
        from_cursor: first.cursor,
        to_cursor: last.cursor,
        entry_count: entries.len().min(u32::MAX as usize) as u32,
        kinds,
        highlights,
    })
}
