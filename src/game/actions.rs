use super::*;

pub(super) fn parse_foundation_cache_command(
    value: &str,
) -> Option<(FoundationCacheAction, Option<FoundationResourceKind>)> {
    let mut parts = value.strip_prefix("foundation-cache:")?.split(':');
    let action = match parts.next()? {
        "inspect" => FoundationCacheAction::Inspect,
        "deposit" => FoundationCacheAction::Deposit,
        "withdraw" => FoundationCacheAction::Withdraw,
        _ => return None,
    };
    let resource = match parts.next()? {
        "timber" => Some(FoundationResourceKind::Timber),
        "stone" => Some(FoundationResourceKind::Stone),
        "iron-ore" => Some(FoundationResourceKind::IronOre),
        "none" if action == FoundationCacheAction::Inspect => None,
        _ => return None,
    };
    let valid_pair = matches!(
        (action, resource),
        (FoundationCacheAction::Inspect, None)
            | (
                FoundationCacheAction::Deposit | FoundationCacheAction::Withdraw,
                Some(_)
            )
    );
    (parts.next().is_none() && valid_pair).then_some((action, resource))
}

pub(super) fn parse_foundation_forge_command(value: &str) -> Option<FoundationForgeAction> {
    match value.strip_prefix("foundation-forge:")? {
        "inspect" => Some(FoundationForgeAction::Inspect),
        "burn-charcoal" => Some(FoundationForgeAction::BurnCharcoal),
        "shape-handle" => Some(FoundationForgeAction::ShapeHandle),
        "forge-field-tool" => Some(FoundationForgeAction::ForgeFieldTool),
        _ => None,
    }
}

pub(super) fn parse_foundation_property_command(
    value: &str,
) -> Option<tarrowyn_protocol::FoundationPropertyRequest> {
    use tarrowyn_protocol::{
        FoundationPropertyAccess, FoundationPropertyAction, FoundationPropertyRequest,
        FoundationResourceKind, Position,
    };
    let mut parts = value.strip_prefix("foundation-property:")?.split(':');
    let verb = parts.next()?;
    let mut request = FoundationPropertyRequest {
        request_id: String::new(),
        action: FoundationPropertyAction::Inspect,
        property_id: None,
        anchor: None,
        entrance: None,
        access: None,
        resource: None,
        amount: 0,
    };
    match verb {
        "preview" | "place" => {
            request.action = if verb == "preview" {
                FoundationPropertyAction::PreviewPlacement
            } else {
                FoundationPropertyAction::PlaceTent
            };
            request.anchor = Some(Position {
                x: parts.next()?.parse().ok()?,
                y: parts.next()?.parse().ok()?,
            });
            request.entrance = Some(parse_property_direction(parts.next()?)?);
        }
        "inspect" | "upgrade" | "builder" | "maintain" => {
            request.action = match verb {
                "inspect" => FoundationPropertyAction::Inspect,
                "upgrade" => FoundationPropertyAction::UpgradeWithMaterials,
                "builder" => FoundationPropertyAction::HireBuilder,
                _ => FoundationPropertyAction::Maintain,
            };
            request.property_id = Some(parts.next()?.to_owned());
        }
        "access" => {
            request.action = FoundationPropertyAction::SetAccess;
            request.property_id = Some(parts.next()?.to_owned());
            request.access = Some(match parts.next()? {
                "owner" => FoundationPropertyAccess::OwnerOnly,
                "guests" => FoundationPropertyAccess::GuestsAllowed,
                _ => return None,
            });
        }
        "store" | "collect" => {
            request.action = if verb == "store" {
                FoundationPropertyAction::Store
            } else {
                FoundationPropertyAction::Collect
            };
            request.property_id = Some(parts.next()?.to_owned());
            request.resource = Some(match parts.next()? {
                "timber" => FoundationResourceKind::Timber,
                "stone" => FoundationResourceKind::Stone,
                "iron-ore" => FoundationResourceKind::IronOre,
                _ => return None,
            });
            request.amount = parts.next()?.parse().ok()?;
            if request.amount == 0 {
                return None;
            }
        }
        _ => return None,
    }
    parts.next().is_none().then_some(request)
}

fn parse_property_direction(value: &str) -> Option<tarrowyn_protocol::FoundationPropertyDirection> {
    use tarrowyn_protocol::FoundationPropertyDirection::*;
    match value {
        "north" => Some(North),
        "east" => Some(East),
        "south" => Some(South),
        "west" => Some(West),
        _ => None,
    }
}

pub(super) fn parse_foundation_storehouse_command(
    value: &str,
) -> Option<(
    String,
    Option<tarrowyn_protocol::FoundationStorehouseContributionInput>,
)> {
    let mut parts = value.strip_prefix("foundation-storehouse:")?.split(':');
    let landmark_id = parts.next()?;
    if !matches!(
        landmark_id,
        "builder-mara" | "first-beacon-noticeboard" | "storehouse-site"
    ) {
        return None;
    }
    let source = parts.next()?;
    let contribution = match source {
        "inspect" if parts.next().is_none() => None,
        "material" | "gold" => {
            let kind = match parts.next()? {
                "timber" => FoundationResourceKind::Timber,
                "stone" => FoundationResourceKind::Stone,
                _ => return None,
            };
            let amount = parts.next()?.parse::<u32>().ok()?;
            if amount == 0 || parts.next().is_some() {
                return None;
            }
            Some(if source == "material" {
                tarrowyn_protocol::FoundationStorehouseContributionInput::Material { kind, amount }
            } else {
                tarrowyn_protocol::FoundationStorehouseContributionInput::Gold {
                    toward: kind,
                    amount,
                }
            })
        }
        _ => return None,
    };
    Some((landmark_id.to_owned(), contribution))
}

pub(super) fn parse_cooperation_trade_command(value: &str) -> Option<TradeRequest> {
    let (action, selector) = if let Some(account_id) = value.strip_prefix("cooperation-offer-ore:")
    {
        (TradeAction::Create, account_id)
    } else if let Some(trade_id) = value.strip_prefix("cooperation-accept-ore:") {
        (TradeAction::Accept, trade_id)
    } else {
        let trade_id = value.strip_prefix("cooperation-review-ore:")?;
        (TradeAction::Review, trade_id)
    };
    if selector.is_empty() || selector.len() > 160 || selector.chars().any(char::is_control) {
        return None;
    }
    Some(TradeRequest {
        request_id: String::new(),
        action,
        trade_id: (action != TradeAction::Create).then(|| selector.to_owned()),
        recipient_account_id: (action == TradeAction::Create).then(|| selector.to_owned()),
        offer: (action == TradeAction::Create).then_some(TradeBundle {
            iron_ore: 2,
            ..TradeBundle::default()
        }),
        request: (action == TradeAction::Create).then_some(TradeBundle::default()),
    })
}

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
        if let Some((action, resource)) = parse_foundation_cache_command(id) {
            client.queue_foundation_cache(action, resource);
            return;
        }
        if let Some(action) = parse_foundation_forge_command(id) {
            client.queue_foundation_forge(action);
            return;
        }
        if let Some(request) = parse_foundation_property_command(id) {
            client.queue_foundation_property(request);
            return;
        }
        if let Some((landmark_id, contribution)) = parse_foundation_storehouse_command(id) {
            client.queue_foundation_storehouse(&landmark_id, contribution);
            return;
        }
        if let Some(request) = parse_cooperation_trade_command(id) {
            client.queue_trade(request);
            return;
        }
        if let Some(interaction_id) = id.strip_prefix("foundation:") {
            client.queue_foundation_interaction(interaction_id);
            return;
        }
        if let Some(resource) = id.strip_prefix("foundation-resource:") {
            let mut parts = resource.split(':');
            let node_id = parts.next().unwrap_or_default();
            let action = match parts.next() {
                Some("log") => Some(tarrowyn_protocol::FoundationResourceAction::Log),
                Some("mine") => Some(tarrowyn_protocol::FoundationResourceAction::Mine),
                _ => None,
            };
            if let Some(action) = action {
                client.queue_foundation_resource(node_id, action);
            }
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
