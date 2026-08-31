use super::*;

pub(super) fn apply_chronicle_key(query: &mut String, key: &str) {
    match key {
        "space" if query.chars().count() < 80 => query.push(' '),
        "delete" => {
            query.pop();
        }
        "clear" => query.clear(),
        key if key.chars().count() == 1 && query.chars().count() < 80 => {
            query.push(key.chars().next().expect("single-character chronicle key"));
        }
        _ => {}
    }
}

impl Game {
    pub(super) fn interact(&mut self, id: &str) {
        if id == "chronicle-close" {
            self.chronicle_open = false;
            return;
        }
        if id == "account-close" {
            self.account_open = false;
            return;
        }
        if id == "account-details" {
            if matches!(&self.mode, ClientMode::Online(_)) {
                self.regional_inspection_open = false;
                self.skill_selection_open = false;
                self.school_selection_open = false;
                self.chronicle_open = false;
                self.account_open = !self.account_open;
            }
            return;
        }
        if id == "chronicle-search" {
            let query = self.chronicle_query.trim().to_owned();
            self.chronicle_query = query.clone();
            if let ClientMode::Online(client) = &mut self.mode {
                client.search_chronicle(&query);
            }
            return;
        }
        if id == "chronicle-search-next" {
            if let ClientMode::Online(client) = &mut self.mode {
                if let Some(cursor) = client.projection.chronicle_search_next_cursor {
                    let query = client
                        .projection
                        .chronicle_search_query
                        .clone()
                        .unwrap_or_default();
                    client.search_chronicle_page(&query, cursor);
                }
            }
            return;
        }
        if let Some(key) = id.strip_prefix("chronicle-key-") {
            apply_chronicle_key(&mut self.chronicle_query, key);
            return;
        }
        if id == "region-details" {
            if matches!(&self.mode, ClientMode::Online(_)) {
                self.skill_selection_open = false;
                self.school_selection_open = false;
                self.chronicle_open = false;
                self.regional_inspection_open = !self.regional_inspection_open;
            }
            return;
        }
        if id == "skill-close" {
            self.skill_selection_open = false;
            return;
        }
        if id == "school-close" {
            self.school_selection_open = false;
            return;
        }
        if id == "practice" {
            if matches!(&self.mode, ClientMode::Online(_)) {
                self.regional_inspection_open = false;
                self.skill_selection_open = true;
                self.school_selection_open = false;
                self.chronicle_open = false;
            }
            return;
        }
        if id == "chronicle" {
            if matches!(&self.mode, ClientMode::Online(_)) {
                self.regional_inspection_open = false;
                self.skill_selection_open = false;
                self.school_selection_open = false;
                self.chronicle_open = !self.chronicle_open;
                if self.chronicle_open {
                    if let ClientMode::Online(client) = &mut self.mode {
                        client.refresh_tavern();
                    }
                }
            }
            return;
        }
        if matches!(id, "logout" | "delete-account") {
            self.regional_inspection_open = false;
            self.skill_selection_open = false;
            self.school_selection_open = false;
            self.chronicle_open = false;
            self.account_open = false;
        }
        if let ClientMode::Online(client) = &mut self.mode {
            match id {
                "plant" => client.queue_farming(FarmingAction::Plant),
                "tend" => client.queue_farming(FarmingAction::Tend),
                "harvest" => client.queue_farming(FarmingAction::Harvest),
                "animal" => client.queue_farming(FarmingAction::TendAnimal),
                "listen" => client.refresh_tavern(),
                "trade" => {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let pending_trade_id = own.and_then(|account_id| {
                        client
                            .pending_trade_for(account_id)
                            .map(|trade| trade.trade_id.clone())
                    });
                    if let Some(trade_id) = pending_trade_id {
                        client.queue_trade(TradeRequest {
                            request_id: String::new(),
                            action: TradeAction::Review,
                            trade_id: Some(trade_id),
                            recipient_account_id: None,
                            offer: None,
                            request: None,
                        });
                    } else {
                        let target = client.projection.players.iter().find(|player| {
                            Some(player.account_id.as_str()) != own
                                && !player.stale(client.projection.server_tick)
                        });
                        if let Some(target) = target {
                            client.queue_trade(TradeRequest {
                                request_id: String::new(),
                                action: TradeAction::Create,
                                trade_id: None,
                                recipient_account_id: Some(target.account_id.clone()),
                                offer: Some(TradeBundle {
                                    seeds: 1,
                                    ..TradeBundle::default()
                                }),
                                request: Some(TradeBundle {
                                    gold: 2,
                                    ..TradeBundle::default()
                                }),
                            });
                        } else {
                            self.notifications
                                .warning("Another player must be present before offering a seed.");
                        }
                    }
                }
                "accept-trade" => {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let incoming_trade_id = own.and_then(|account_id| {
                        client
                            .incoming_trade_for(account_id)
                            .map(|trade| trade.trade_id.clone())
                    });
                    if let Some(trade_id) = incoming_trade_id {
                        client.queue_trade(TradeRequest {
                            request_id: String::new(),
                            action: TradeAction::Accept,
                            trade_id: Some(trade_id),
                            recipient_account_id: None,
                            offer: None,
                            request: None,
                        });
                    } else {
                        self.notifications
                            .warning("No pending trade is waiting for this character.");
                    }
                }
                "cancel-trade" => {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let pending_trade_id = own.and_then(|account_id| {
                        client
                            .pending_trade_for(account_id)
                            .map(|trade| trade.trade_id.clone())
                    });
                    if let Some(trade_id) = pending_trade_id {
                        client.queue_trade(TradeRequest {
                            request_id: String::new(),
                            action: TradeAction::Cancel,
                            trade_id: Some(trade_id),
                            recipient_account_id: None,
                            offer: None,
                            request: None,
                        });
                    } else {
                        self.notifications
                            .warning("No pending trade is waiting to be cancelled.");
                    }
                }
                "contract" => client.queue_contract_cycle(),
                "strike" => client.queue_combat(
                    tarrowyn_protocol::CombatAction::Strike,
                    tarrowyn_protocol::WeaponKind::IronSword,
                ),
                "frontier-retreat" => client.queue_combat(
                    tarrowyn_protocol::CombatAction::Retreat,
                    tarrowyn_protocol::WeaponKind::IronSword,
                ),
                "recover-self" => {
                    client.queue_recovery(tarrowyn_protocol::RecoveryChoice::SelfRecover)
                }
                "recover" => client.queue_recovery(tarrowyn_protocol::RecoveryChoice::AskRescuer),
                "recover-healer" => {
                    client.queue_recovery(tarrowyn_protocol::RecoveryChoice::PayHealer)
                }
                "claim" => client.queue_claim_cycle(),
                "abandon-claim" => client
                    .queue_claim_action(tarrowyn_protocol::ClaimLifecycleAction::Abandon, None),
                "transfer-claim" => {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let target = client
                        .projection
                        .players
                        .iter()
                        .find(|player| {
                            Some(player.account_id.as_str()) != own
                                && !player.stale(client.projection.server_tick)
                                && player.position == client.projection.player_position
                        })
                        .map(|player| player.account_id.clone());
                    if let Some(target) = target {
                        client.queue_claim_action(
                            tarrowyn_protocol::ClaimLifecycleAction::Transfer,
                            Some(target),
                        );
                    } else {
                        self.notifications.warning(
                            "Stand beside another recognised player before transferring a lease.",
                        );
                    }
                }
                "expedition" => client.queue_expedition_cycle(),
                "report" => client.queue_report(),
                "knowledge" => {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let target = client
                        .projection
                        .players
                        .iter()
                        .find(|player| {
                            Some(player.account_id.as_str()) != own
                                && !player.stale(client.projection.server_tick)
                                && player.position == client.projection.player_position
                        })
                        .map(|player| player.account_id.clone());
                    client.queue_knowledge_cycle(target);
                }
                "town-hall" | "tax-rate" | "registry" | "order" | "households" | "local-fight"
                | "retreat" | "technique" | "guard" | "item" | "reposition" | "spell" => {
                    client.queue_phase4(id)
                }
                "crafting-timing" => client.queue_crafting_timing(),
                "school" => {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let target = client
                        .projection
                        .players
                        .iter()
                        .find(|player| {
                            Some(player.account_id.as_str()) != own
                                && !player.stale(client.projection.server_tick)
                                && player.position == client.projection.player_position
                        })
                        .map(|player| player.account_id.clone());
                    match target {
                        Some(target) if client.has_open_skill_lesson(&target) => {
                            if !client.queue_skill_teach(&target) {
                                self.notifications.warning(
                                    "The open lesson is no longer ready; refresh the school ledger.",
                                );
                            }
                        }
                        Some(_) => {
                            self.regional_inspection_open = false;
                            self.skill_selection_open = false;
                            self.school_selection_open = true;
                            self.chronicle_open = false;
                        }
                        None => self
                            .notifications
                            .warning("Another nearby player must be present for a school lesson."),
                    }
                }
                "travel" | "recover-travel" | "route-repair" | "route-escort" | "route-improve"
                | "market-region" | "region-event" | "cancel-market" | "account" | "logout"
                | "delete-account" => client.queue_phase5(id),
                _ => self.notifications.warning(format!("Unknown action: {id}")),
            }
            return;
        }
        let ClientMode::Offline(session) = &mut self.mode else {
            return;
        };
        let Some(action) = self.data.actions.get(id) else {
            self.notifications.warning(format!("Unknown action: {id}"));
            return;
        };
        let result = session.apply_action(action);
        if result.success {
            self.notifications.success(result.message);
        } else {
            self.notifications.warning(result.message);
        }
    }
}

impl Game {
    pub(super) fn apply_action(&mut self, action: UiAction) {
        match action {
            UiAction::UseOffline => {
                self.mode = ClientMode::Offline(GameSession::new(&self.data.config));
                self.regional_inspection_open = false;
                self.skill_selection_open = false;
                self.school_selection_open = false;
                self.chronicle_open = false;
                self.chronicle_query.clear();
                self.account_open = false;
                self.chat_draft.clear();
                self.sync_camera(TilePos::new(8, 6));
                self.notifications
                    .info("Offline first evening enabled; it does not connect to the shared road.");
            }
            UiAction::UseOnline => {
                self.mode = ClientMode::Online(Box::new(OnlineClient::new(
                    &self.server_url,
                    &self.data.config,
                )));
                self.regional_inspection_open = false;
                self.skill_selection_open = false;
                self.school_selection_open = false;
                self.chronicle_open = false;
                self.chronicle_query.clear();
                self.account_open = false;
                self.chat_draft.clear();
                self.sync_camera(TilePos::new(8, 6));
                self.notifications.info("Connecting to the shared road…");
            }
            UiAction::Reconnect => match &mut self.mode {
                ClientMode::Online(client) => {
                    self.regional_inspection_open = false;
                    self.skill_selection_open = false;
                    self.school_selection_open = false;
                    self.chronicle_open = false;
                    self.chronicle_query.clear();
                    self.account_open = false;
                    if !client.reconnect() {
                        self.notifications
                            .warning("Wait for the reconnect cooldown to finish.");
                    }
                }
                ClientMode::Offline(_) => {
                    self.mode = ClientMode::Online(Box::new(OnlineClient::new(
                        &self.server_url,
                        &self.data.config,
                    )));
                    self.regional_inspection_open = false;
                    self.skill_selection_open = false;
                    self.school_selection_open = false;
                    self.chronicle_open = false;
                    self.chronicle_query.clear();
                    self.account_open = false;
                    self.sync_camera(TilePos::new(8, 6));
                    self.notifications.info("Connecting to the shared road…");
                }
            },
            UiAction::NewEvening => match &mut self.mode {
                ClientMode::Offline(session) => {
                    *session = GameSession::new(&self.data.config);
                    self.sync_camera(TilePos::new(8, 6));
                    self.notifications
                        .info("A fresh offline first evening begins at the Hearth.");
                }
                ClientMode::Online(_) => self
                    .notifications
                    .warning("The shared road owns the online world; use Reconnect to recover it."),
            },
            UiAction::Save => self.save_game(),
            UiAction::Load => self.load_game(),
            UiAction::DeleteSave => self.delete_save(),
            UiAction::Move(dx, dy) => self.queue_movement(dx, dy),
            UiAction::MoveTo(tile) => self.move_toward(tile),
            UiAction::Interact(id) => self.interact(&id),
            UiAction::Practice(skill_id) => {
                self.skill_selection_open = false;
                self.school_selection_open = false;
                self.chronicle_open = false;
                if let ClientMode::Online(client) = &mut self.mode {
                    client.queue_skill_practice(skill_id);
                }
            }
            UiAction::Teach(skill_id) => {
                self.school_selection_open = false;
                self.chronicle_open = false;
                if let ClientMode::Online(client) = &mut self.mode {
                    let own = client
                        .account
                        .as_ref()
                        .map(|account| account.account_id.as_str());
                    let target = own.and_then(|own| {
                        client
                            .projection
                            .players
                            .iter()
                            .find(|player| {
                                player.account_id != own
                                    && !player.stale(client.projection.server_tick)
                                    && player.position == client.projection.player_position
                            })
                            .map(|player| player.account_id.clone())
                    });
                    if let Some(target) = target {
                        if !client.queue_skill_teach_for(&target, &skill_id) {
                            self.notifications.warning(
                                "That discipline is no longer ready to teach; refresh the school ledger.",
                            );
                        }
                    } else {
                        self.notifications
                            .warning("Another nearby player must be present for a school lesson.");
                    }
                }
            }
            UiAction::RegionalEvent(intervention) => {
                if let ClientMode::Online(client) = &mut self.mode {
                    client.queue_region_intervention(intervention);
                }
            }
            UiAction::SendChat => {
                let text = self.chat_draft.trim().to_owned();
                if let ClientMode::Online(client) = &mut self.mode {
                    if client.queue_chat(&text) {
                        self.chat_draft.clear();
                    }
                }
            }
            UiAction::QuickChat(text) => {
                if let ClientMode::Online(client) = &mut self.mode {
                    client.queue_chat(&text);
                }
            }
            UiAction::Zoom(delta) => {
                self.camera.zoom = (self.camera.zoom + delta).clamp(0.9, 1.15);
            }
        }
    }
}
