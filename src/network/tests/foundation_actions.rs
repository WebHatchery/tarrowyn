use super::*;
use tarrowyn_protocol::{
    FoundationCacheAction, FoundationResourceAction, FoundationResourceAmount,
    FoundationResourceKind,
};

#[test]
fn nearby_resource_queue_uses_one_non_blocking_authoritative_command() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.state_reload_pending = false;

    assert!(
        client.queue_foundation_resource("whisperwood-edge-node", FoundationResourceAction::Log)
    );
    let pending = client.pending_foundation_resource.as_ref().unwrap();
    assert_eq!(pending.request.node_id, "whisperwood-edge-node");
    assert_eq!(pending.request.action, FoundationResourceAction::Log);
    assert!(!client
        .queue_foundation_resource("shallow-stone-seam-node", FoundationResourceAction::Mine));
}

#[test]
fn shared_cache_queue_keeps_one_typed_request_for_safe_retries() {
    let mut client = OnlineClient::new("http://127.0.0.1:8787", &config());
    client.state = ConnectionState::Online;
    client.state_reload_pending = false;

    assert!(client.queue_foundation_cache(
        FoundationCacheAction::Deposit,
        Some(FoundationResourceKind::Stone)
    ));
    let pending = client.pending_foundation_cache.as_ref().unwrap();
    assert_eq!(pending.request.action, FoundationCacheAction::Deposit);
    assert_eq!(
        pending.request.resource,
        Some(FoundationResourceKind::Stone)
    );
    assert_eq!(pending.request.amount, 1);
    assert!(pending.request.request_id.starts_with("foundation-cache-"));
    assert!(!client.queue_foundation_cache(FoundationCacheAction::Inspect, None));
}

#[test]
fn gathering_notice_names_every_authoritative_yield() {
    let notice = super::super::foundation::foundation_resource_success_notice(&[
        FoundationResourceAmount {
            kind: FoundationResourceKind::Stone,
            amount: 2,
        },
        FoundationResourceAmount {
            kind: FoundationResourceKind::IronOre,
            amount: 1,
        },
    ]);

    assert_eq!(
        notice,
        "Gathered 2 stone and 1 iron ore with the shared crude tools."
    );
}

#[test]
fn cache_feedback_names_the_authoritative_transfer() {
    assert_eq!(
        super::super::foundation::foundation_cache_success_notice(
            FoundationCacheAction::Withdraw,
            Some(FoundationResourceKind::IronOre)
        ),
        "Collected 1 iron ore from the shared cache."
    );
}
