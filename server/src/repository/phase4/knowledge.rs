use super::{account_id, cache_key, record, validate_optional_identifier, validate_request_id};
use tarrowyn_protocol::{
    ApiResponse, KnowledgeAction, KnowledgeRequest, KnowledgeResponse, KnowledgeState,
};

impl super::super::WorldRepository {
    pub fn knowledge(
        &self,
        token: &str,
        request: KnowledgeRequest,
    ) -> Result<ApiResponse<KnowledgeResponse>, super::super::RepositoryError> {
        let mut state = self.state.lock().expect("world repository lock poisoned");
        self.expire_and_persist_sessions(&mut state)?;
        let key = super::super::authenticate(&mut state, token, &self.config)?;
        validate_request_id(&request.request_id)?;
        let knowledge_id = validate_optional_identifier(
            request.knowledge_id.as_deref(),
            "invalid_knowledge_id",
            "A knowledge selector must be bounded and contain no control characters.",
        )?;
        let target_account_id = validate_optional_identifier(
            request.target_account_id.as_deref(),
            "invalid_target_account_id",
            "A target account selector must be bounded and contain no control characters.",
        )?;
        if request.action == KnowledgeAction::Inspect {
            return Ok(ApiResponse {
                meta: super::super::meta(
                    state.tick,
                    Some(request.request_id.clone()),
                    Some(state.cursor),
                ),
                data: KnowledgeResponse {
                    request_id: request.request_id,
                    accepted: true,
                    knowledge: view(&state, &key),
                    message: "The guild archive is open to inspection.".to_owned(),
                    reason: None,
                },
            });
        }
        let actor_id = account_id(&state, &key);
        let cache = cache_key(&actor_id, &request.request_id);
        if let Some(super::Phase4Response::Knowledge(response)) =
            state.phase4.request_results.get(&cache)
        {
            return Ok(ApiResponse {
                meta: super::super::meta(state.tick, Some(request.request_id), Some(state.cursor)),
                data: response.clone(),
            });
        }
        let mut response = KnowledgeResponse {
            request_id: request.request_id.clone(),
            accepted: false,
            knowledge: view(&state, &key),
            message: "The guild archive is open to inspection.".to_owned(),
            reason: None,
        };
        match request.action {
            KnowledgeAction::Inspect => unreachable!("inspection returned before command handling"),
            KnowledgeAction::Discover => {
                let Some(index) = item_index(&state, knowledge_id.as_deref()) else {
                    response.reason = Some(
                        "That knowledge item is not discoverable in this settlement.".to_owned(),
                    );
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let item_id = state.phase4.knowledge[index].knowledge_id.clone();
                if known(&state, &key, &item_id) {
                    response.reason =
                        Some("Your field notes already contain that knowledge.".to_owned());
                } else {
                    state
                        .phase4
                        .known_by
                        .entry(key.clone())
                        .or_default()
                        .push(item_id.clone());
                    if !state.phase4.knowledge[index]
                        .discovered_by
                        .contains(&actor_id)
                    {
                        state.phase4.knowledge[index]
                            .discovered_by
                            .push(actor_id.clone());
                    }
                    response.accepted = true;
                    response.message = state.phase4.knowledge[index].effect.clone();
                    record(&mut state, "knowledge discovered", "A useful truth is found and recorded", "A player discovered a technique that can be applied or taught server-authoritatively.");
                }
            }
            KnowledgeAction::Teach => {
                let Some(index) = item_index(&state, knowledge_id.as_deref()) else {
                    response.reason = Some("Name the knowledge item to teach.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let Some(target_account) = target_account_id.as_deref() else {
                    response.reason = Some("Teaching needs a receiving account.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let Some(target_key) = super::key_for_account(&state, target_account) else {
                    response.reason = Some(
                        "The receiving player must have a recognised settlement account."
                            .to_owned(),
                    );
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let item_id = state.phase4.knowledge[index].knowledge_id.clone();
                if target_key == key {
                    response.reason =
                        Some("A lesson needs another player to receive the technique.".to_owned());
                } else if !known(&state, &key, &item_id) {
                    response.reason =
                        Some("Discover or receive the knowledge before teaching it.".to_owned());
                } else if !state.phase4.knowledge[index].teachable {
                    response.reason = Some(
                        "This knowledge is useful but cannot be taught as a shortcut.".to_owned(),
                    );
                } else {
                    let known_by_target = state.phase4.known_by.entry(target_key).or_default();
                    if !known_by_target.contains(&item_id) {
                        known_by_target.push(item_id);
                    }
                    response.accepted = true;
                    response.message =
                        "The receiving player can now apply the taught technique.".to_owned();
                    super::super::skills::record_practice(&mut state, &key, "teaching");
                    record(&mut state, "knowledge taught", "A useful truth crosses from one player to another", "The server recorded a teachable knowledge transfer in the settlement archive.");
                }
            }
            KnowledgeAction::Record => {
                let Some(index) = item_index(&state, knowledge_id.as_deref()) else {
                    response.reason =
                        Some("Name the knowledge item to write into the archive.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let item_id = state.phase4.knowledge[index].knowledge_id.clone();
                if !known(&state, &key, &item_id) {
                    response.reason = Some(
                        "Only known knowledge can be written to the guild archive.".to_owned(),
                    );
                } else if !state.phase4.knowledge[index].writable {
                    response.reason = Some(
                        "This clue is oral only and cannot be stored in the archive.".to_owned(),
                    );
                } else {
                    state.phase4.knowledge[index].stored_in =
                        "The Hearth guild archive (written)".to_owned();
                    response.accepted = true;
                    response.message =
                        "The guild archive will preserve this technique for new players."
                            .to_owned();
                    record(
                        &mut state,
                        "knowledge recorded",
                        "The guild archive keeps a lesson for later hands",
                        "A discovered technique was written into the settlement record.",
                    );
                }
            }
            KnowledgeAction::Apply => {
                let Some(index) = item_index(&state, knowledge_id.as_deref()) else {
                    response.reason = Some("Name the knowledge item to apply.".to_owned());
                    return finish(self, &mut state, cache, request.request_id, response);
                };
                let item_id = state.phase4.knowledge[index].knowledge_id.clone();
                if !known(&state, &key, &item_id) {
                    response.reason =
                        Some("Discover or receive this knowledge before applying it.".to_owned());
                } else {
                    response.accepted = true;
                    response.message = state.phase4.knowledge[index].effect.clone();
                    record(
                        &mut state,
                        "knowledge applied",
                        "A recorded technique changes local work",
                        &response.message,
                    );
                }
            }
        }
        response.knowledge = view(&state, &key);
        finish(self, &mut state, cache, request.request_id, response)
    }
}

fn finish(
    repository: &super::super::WorldRepository,
    state: &mut super::super::models::RepositoryState,
    cache: String,
    request_id: String,
    response: KnowledgeResponse,
) -> Result<ApiResponse<KnowledgeResponse>, super::super::RepositoryError> {
    state
        .phase4
        .request_results
        .insert(cache, super::Phase4Response::Knowledge(response.clone()));
    super::super::record_command_outcome(state, response.accepted);
    repository.persist(state)?;
    Ok(ApiResponse {
        meta: super::super::meta(state.tick, Some(request_id), Some(state.cursor)),
        data: response,
    })
}

fn view(state: &super::super::models::RepositoryState, key: &str) -> KnowledgeState {
    let known_items = state.phase4.known_by.get(key);
    KnowledgeState {
        items: state
            .phase4
            .knowledge
            .iter()
            .map(|item| {
                let known_by_player = known_items
                    .is_some_and(|known| known.iter().any(|id| id == &item.knowledge_id));
                if known_by_player || item.stored_in.contains("guild archive") {
                    item.clone()
                } else {
                    redacted_item(item)
                }
            })
            .collect(),
        known_by_player: known_items.cloned().unwrap_or_default(),
        cursor: state.cursor,
    }
}

fn redacted_item(item: &tarrowyn_protocol::KnowledgeItem) -> tarrowyn_protocol::KnowledgeItem {
    tarrowyn_protocol::KnowledgeItem {
        knowledge_id: item.knowledge_id.clone(),
        title: "Unrevealed field clue".to_owned(),
        kind: item.kind,
        description: "A discovery is waiting, but its method remains with the discoverer."
            .to_owned(),
        effect: "Discover this clue through play or receive it from another player.".to_owned(),
        teachable: false,
        writable: false,
        discovered_by: Vec::new(),
        stored_in: "Private field notes".to_owned(),
    }
}

fn item_index(
    state: &super::super::models::RepositoryState,
    knowledge_id: Option<&str>,
) -> Option<usize> {
    match knowledge_id {
        Some(id) => state
            .phase4
            .knowledge
            .iter()
            .position(|item| item.knowledge_id == id),
        None => (!state.phase4.knowledge.is_empty()).then_some(0),
    }
}

fn known(state: &super::super::models::RepositoryState, key: &str, item_id: &str) -> bool {
    state
        .phase4
        .known_by
        .get(key)
        .is_some_and(|items| items.iter().any(|item| item == item_id))
}
