use super::*;
use tarrowyn_protocol::{
    FoundationResourceAction, FoundationResourceAmount, FoundationResourceKind,
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
