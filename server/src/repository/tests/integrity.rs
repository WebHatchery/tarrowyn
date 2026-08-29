use super::super::{ServerConfig, WorldRepository};
use tarrowyn_protocol::{
    AccountDeletionRequest, AuthLinkRequest, ClaimLifecycleAction, ClaimLifecycleRequest,
    CommodityKind, CropKind, CropState, GovernanceRequest, MarketOrder, MarketOrderStatus,
    RegionalEvent, RegionalEventStage, RegionalHousehold, TradeBundle, TradeOffer, TradeStatus,
    TravelState, TravelStatus,
};

#[test]
fn invalid_regional_route_bounds_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.routes[0].risk_percent = 101;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_regional_settlement_bounds_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.settlements[0].governance = 101;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn missing_regional_collections_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.routes.clear();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_regional_topology_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.routes[0].origin_location_id = "missing-location".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn duplicate_settlement_locations_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        let location_id = state.phase5.settlements[0].location_id.clone();
        state.phase5.settlements[1].location_id = location_id;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_market_order_reference_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.market_orders.push(MarketOrder {
            order_id: "integrity-market-order".to_owned(),
            owner_account_id: "former-resident".to_owned(),
            owner_name: "Former resident".to_owned(),
            origin_location_id: "hearth".to_owned(),
            destination_location_id: "saltmere".to_owned(),
            commodity: CommodityKind::Stone,
            quantity: 1,
            unit_price: 3,
            total_price: 3,
            status: MarketOrderStatus::Open,
            created_tick: 0,
            settled_tick: None,
            route_id: "missing-route".to_owned(),
            fallback_used: false,
        });
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_travel_reference_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = super::guest(&repository, "integrity-travel");
    let identity_key = {
        let state = repository.state.lock().expect("repository lock");
        state
            .identities
            .iter()
            .find(|(_, identity)| identity.character_id == session.character_id)
            .map(|(key, _)| key.clone())
            .expect("guest identity")
    };
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.travel.insert(
            identity_key,
            TravelState {
                travel_id: "integrity-travel".to_owned(),
                route_id: "missing-route".to_owned(),
                origin_location_id: "hearth".to_owned(),
                destination_location_id: "saltmere".to_owned(),
                departure_tick: 0,
                eta_tick: 7,
                progress: 0,
                risk_percent: 12,
                status: TravelStatus::Travelling,
                interruption: None,
                recovery_note: None,
            },
        );
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_event_location_reference_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.events.push(RegionalEvent {
            event_id: "integrity-event".to_owned(),
            title: "A malformed signal".to_owned(),
            kind: "weather".to_owned(),
            stage: RegionalEventStage::Signal,
            affected_location_ids: vec!["missing-location".to_owned()],
            effects: vec!["The signal cannot be placed".to_owned()],
            cause: "integrity test".to_owned(),
            intervention_options: vec!["watch".to_owned()],
            chosen_intervention: None,
            outcome: None,
            started_tick: 0,
            updated_tick: 0,
            cursor: 0,
        });
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_household_location_reference_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase5.households.push(RegionalHousehold {
            household_id: "integrity-household".to_owned(),
            household_name: "A malformed household".to_owned(),
            origin_location_id: "missing-location".to_owned(),
            destination_location_id: Some("hearth".to_owned()),
            status: "considering".to_owned(),
            reason: "integrity test".to_owned(),
            service: "test service".to_owned(),
            departure_tick: None,
            arrival_tick: None,
            history: vec!["integrity test".to_owned()],
        });
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_market_stock_reference_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase5
            .stock
            .insert("missing-location:stone".to_owned(), 1);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn duplicate_identity_account_ids_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let first = super::guest(&repository, "integrity-account-one");
    let second = super::guest(&repository, "integrity-account-two");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let first_key = state
            .identities
            .iter()
            .find(|(_, identity)| identity.character_id == first.character_id)
            .map(|(key, _)| key.clone())
            .expect("first guest identity");
        let second_key = state
            .identities
            .iter()
            .find(|(_, identity)| identity.character_id == second.character_id)
            .map(|(key, _)| key.clone())
            .expect("second guest identity");
        let account_id = state
            .identities
            .get(&first_key)
            .expect("first identity")
            .account_id
            .clone();
        state
            .identities
            .get_mut(&second_key)
            .expect("second identity")
            .account_id = account_id;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_phase4_governance_reference_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.governance.settlement_id = "missing-settlement".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn duplicate_phase4_claim_ids_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = super::guest(&repository, "integrity-phase4-claim");
    repository
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "integrity-phase4-claim-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let duplicate = state.phase4.claims[0].clone();
        state.phase4.claims.push(duplicate);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_phase4_account_reference_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = super::guest(&repository, "integrity-phase4-account");
    let mut request = GovernanceRequest {
        request_id: "integrity-phase4-proposal".to_owned(),
        action: tarrowyn_protocol::GovernanceAction::Propose,
        office_id: None,
        proposal_id: None,
        public_action: None,
        target: None,
        cost: None,
        tax_rate_percent: None,
    };
    request.public_action = Some(tarrowyn_protocol::PublicAction::RepairRoad);
    repository
        .governance(&session.account_token, request)
        .expect("proposal request");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.governance.proposals[0].proposer_account_id = "missing-account".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_phase4_keyed_state_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase4
            .profiles
            .insert("missing-identity".to_owned(), Vec::new());
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_phase4_bounds_degrade_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.households[0].food = 101;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_phase4_claim_position_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = super::guest(&repository, "integrity-phase4-claim-position");
    repository
        .claim_lifecycle(
            &session.account_token,
            ClaimLifecycleRequest {
                request_id: "integrity-phase4-claim-position-request".to_owned(),
                action: ClaimLifecycleAction::Request,
                claim_id: None,
                target_account_id: None,
            },
        )
        .expect("claim request");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.claims[0].position.x = -1;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_phase4_available_plot_position_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase4.available_plots[0].y = -1;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn missing_phase4_foundation_collections_degrade_readiness() {
    for missing in [
        "offices",
        "infrastructure",
        "households",
        "knowledge",
        "animals",
    ] {
        let repository = WorldRepository::new(ServerConfig::default());
        {
            let mut state = repository.state.lock().expect("repository lock");
            match missing {
                "offices" => state.phase4.governance.offices.clear(),
                "infrastructure" => state.phase4.infrastructure.clear(),
                "households" => state.phase4.households.clear(),
                "knowledge" => state.phase4.knowledge.clear(),
                "animals" => state.phase4.animals.clear(),
                _ => unreachable!("test collection is fixed"),
            }
        }

        let health = repository.ops_health().data;
        assert!(!health.ready, "missing {missing} should degrade readiness");
        assert!(
            !health.integrity_ok,
            "missing {missing} should fail the integrity check"
        );
    }
}

#[test]
fn invalid_player_position_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let session = super::guest(&repository, "integrity-player-position");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .identities
            .get_mut(&session.client_key)
            .expect("guest identity")
            .position = tarrowyn_protocol::Position { x: -1, y: 6 };
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_saved_crop_state_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.plots[0].crop = Some(CropState {
            kind: CropKind::Wheat,
            stage: 4,
            quality: 80,
            planted_tick: 0,
            last_tended_tick: None,
        });
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_frontier_threat_state_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase3.zone.monster_health = 99;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_trade_record_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let creator = super::guest(&repository, "integrity-trade-creator");
    let recipient = super::guest(&repository, "integrity-trade-recipient");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.trades.insert(
            "wrong-trade-key".to_owned(),
            TradeOffer {
                trade_id: "integrity-trade".to_owned(),
                creator_account_id: creator.account_id,
                creator_name: "Creator".to_owned(),
                recipient_account_id: recipient.account_id,
                recipient_name: "Recipient".to_owned(),
                offer: TradeBundle::default(),
                request: TradeBundle::default(),
                status: TradeStatus::Pending,
                created_tick: 1,
                expires_tick: 2,
            },
        );
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_event_cursor_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .events
            .front_mut()
            .expect("startup notice event")
            .cursor = 0;
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_production_account_link_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = super::guest(&repository, "integrity-production-account");
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "integrity-production-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "integrity-production-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("production account link");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase6
            .accounts
            .get_mut(&linked.data.account_id)
            .expect("production account")
            .identity_key = "missing-identity".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn missing_production_session_mirror_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = super::guest(&repository, "integrity-production-session");
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "integrity-production-session-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "integrity-production-session-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("production account link")
        .data;
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.sessions.remove(&linked.session.account_token);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn invalid_audit_outcome_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = super::guest(&repository, "integrity-production-audit");
    repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "integrity-production-audit-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "integrity-production-audit-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("production account link");
    {
        let mut state = repository.state.lock().expect("repository lock");
        state.phase6.audits.back_mut().expect("link audit").outcome = "unknown".to_owned();
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn orphan_moderation_timestamp_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    {
        let mut state = repository.state.lock().expect("repository lock");
        state
            .phase6
            .report_created_at
            .insert("orphan-report".to_owned(), 1);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}

#[test]
fn malformed_account_deletion_queue_key_degrades_readiness() {
    let repository = WorldRepository::new(ServerConfig::default());
    let guest = super::guest(&repository, "integrity-production-deletion");
    let linked = repository
        .auth_link(
            &guest.account_token,
            AuthLinkRequest {
                request_id: "integrity-production-deletion-link".to_owned(),
                provider: "webhatchery-identity-oidc".to_owned(),
                subject: "integrity-production-deletion-subject".to_owned(),
                display_name: None,
            },
        )
        .expect("production account link")
        .data;
    repository
        .account_delete(
            &linked.session.account_token,
            AccountDeletionRequest {
                request_id: "integrity-production-deletion-request".to_owned(),
                account_id: linked.account_id,
            },
        )
        .expect("account deletion request");
    {
        let mut state = repository.state.lock().expect("repository lock");
        let original_key = state
            .phase6
            .deletion_requests
            .keys()
            .next()
            .cloned()
            .expect("pending deletion");
        let pending = state
            .phase6
            .deletion_requests
            .remove(&original_key)
            .expect("pending deletion");
        state
            .phase6
            .deletion_requests
            .insert("malformed-deletion-key".to_owned(), pending);
    }

    let health = repository.ops_health().data;
    assert!(!health.ready);
    assert!(!health.integrity_ok);
}
