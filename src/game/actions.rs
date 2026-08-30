use super::*;

impl Game {
    pub(super) fn interact(&mut self, id: &str) {
        if id == "chronicle-close" {
            self.chronicle_open = false;
            return;
        }
        if id == "chronicle-search" {
            if let ClientMode::Online(client) = &mut self.mode {
                client.search_chronicle(&self.chronicle_query);
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
        if id == "region-details" {
            if matches!(&self.mode, ClientMode::Online(_)) {
                self.skill_selection_open = false;
                self.chronicle_open = false;
                self.regional_inspection_open = !self.regional_inspection_open;
            }
            return;
        }
        if id == "skill-close" {
            self.skill_selection_open = false;
            return;
        }
        if id == "practice" {
            if matches!(&self.mode, ClientMode::Online(_)) {
                self.regional_inspection_open = false;
                self.skill_selection_open = true;
                self.chronicle_open = false;
            }
            return;
        }
        if id == "chronicle" {
            if matches!(&self.mode, ClientMode::Online(_)) {
                self.regional_inspection_open = false;
                self.skill_selection_open = false;
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
            self.chronicle_open = false;
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
                        Some(target) if client.queue_skill_teach(&target) => {}
                        Some(_) => self.notifications.warning(
                            "No mastered discipline is ready, or the school ledger is busy.",
                        ),
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
