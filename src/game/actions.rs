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
        if id == "art-catalog" {
            self.menu_open = false;
            self.art_catalog_open = true;
            self.art_catalog_page = 0;
            self.regional_inspection_open = false;
            self.skill_selection_open = false;
            self.school_selection_open = false;
            self.chronicle_open = false;
            self.account_open = false;
            return;
        }
        if id == "art-catalog-close" {
            self.art_catalog_open = false;
            return;
        }
        if id == "art-page-prev" {
            self.art_catalog_page = self.art_catalog_page.saturating_sub(1);
            return;
        }
        if id == "art-page-next" {
            self.art_catalog_page = (self.art_catalog_page + 1) % 3;
            return;
        }
        if id == "menu-toggle" {
            self.menu_open = !self.menu_open;
            return;
        }
        if id == "menu-close" {
            self.menu_open = false;
            return;
        }
        self.menu_open = false;
        if id == "chronicle-close" {
            self.chronicle_open = false;
            return;
        }
        if id == "account-close" {
            self.account_open = false;
            return;
        }
        if id == "account-details" {
            self.regional_inspection_open = false;
            self.skill_selection_open = false;
            self.school_selection_open = false;
            self.chronicle_open = false;
            self.account_open = !self.account_open;
            return;
        }
        if id == "chronicle-search" {
            let query = self.chronicle_query.trim().to_owned();
            self.chronicle_query = query.clone();
            self.mode.search_chronicle(&query);
            return;
        }
        if id == "chronicle-search-next" {
            if let Some(cursor) = self.mode.projection.chronicle_search_next_cursor {
                let query = self
                    .mode
                    .projection
                    .chronicle_search_query
                    .clone()
                    .unwrap_or_default();
                self.mode.search_chronicle_page(&query, cursor);
            }
            return;
        }
        if let Some(key) = id.strip_prefix("chronicle-key-") {
            apply_chronicle_key(&mut self.chronicle_query, key);
            return;
        }
        if id == "region-details" {
            self.skill_selection_open = false;
            self.school_selection_open = false;
            self.chronicle_open = false;
            self.regional_inspection_open = !self.regional_inspection_open;
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
            self.regional_inspection_open = false;
            self.skill_selection_open = true;
            self.school_selection_open = false;
            self.chronicle_open = false;
            return;
        }
        if id == "chronicle" {
            self.regional_inspection_open = false;
            self.skill_selection_open = false;
            self.school_selection_open = false;
            self.chronicle_open = !self.chronicle_open;
            if self.chronicle_open {
                self.mode.refresh_tavern();
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
        let client = &mut self.mode;
        if let Some(interaction_id) = id.strip_prefix("foundation:") {
            client.queue_foundation_interaction(interaction_id);
            return;
        }
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
            "recover-self" => client.queue_recovery(tarrowyn_protocol::RecoveryChoice::SelfRecover),
            "recover" => client.queue_recovery(tarrowyn_protocol::RecoveryChoice::AskRescuer),
            "recover-healer" => client.queue_recovery(tarrowyn_protocol::RecoveryChoice::PayHealer),
            "claim" => client.queue_claim_cycle(),
            "abandon-claim" => {
                client.queue_claim_action(tarrowyn_protocol::ClaimLifecycleAction::Abandon, None)
            }
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
    }
}

impl Game {
    pub(super) fn apply_action(&mut self, action: UiAction) {
        match action {
            UiAction::Reconnect => {
                let client = &mut self.mode;
                self.menu_open = false;
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
            UiAction::MoveTo(tile) => self.move_toward(tile),
            UiAction::Interact(id) => self.interact(&id),
            UiAction::Practice(skill_id) => {
                self.menu_open = false;
                self.skill_selection_open = false;
                self.school_selection_open = false;
                self.chronicle_open = false;
                self.mode.queue_skill_practice(skill_id);
            }
            UiAction::Teach(skill_id) => {
                self.menu_open = false;
                self.school_selection_open = false;
                self.chronicle_open = false;
                {
                    let client = &mut self.mode;
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
                self.menu_open = false;
                self.mode.queue_region_intervention(intervention);
            }
            UiAction::SendChat => {
                let text = self.chat_draft.trim().to_owned();
                if self.mode.queue_chat(&text) {
                    self.chat_draft.clear();
                }
            }
            UiAction::QuickChat(text) => {
                self.mode.queue_chat(&text);
            }
            UiAction::Zoom(delta) => {
                self.camera.zoom = (self.camera.zoom + delta).clamp(0.9, 1.15);
            }
        }
    }
}
