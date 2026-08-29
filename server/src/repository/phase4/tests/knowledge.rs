use super::super::super::{ServerConfig, WorldRepository};
use super::guest;
use tarrowyn_protocol::{KnowledgeAction, KnowledgeRequest};

fn knowledge_request(request_id: &str, action: KnowledgeAction) -> KnowledgeRequest {
    KnowledgeRequest {
        request_id: request_id.to_owned(),
        action,
        knowledge_id: Some("moonberry-tending".to_owned()),
        target_account_id: None,
    }
}

#[test]
fn undiscovered_knowledge_stays_private_until_taught_or_recorded() {
    let repository = WorldRepository::new(ServerConfig {
        backup_path: None,
        ..ServerConfig::default()
    });
    let discoverer = guest(&repository, "knowledge-private-discoverer");
    let learner = guest(&repository, "knowledge-private-learner");
    let observer = guest(&repository, "knowledge-private-observer");

    let initial = repository
        .knowledge(
            &observer.account_token,
            KnowledgeRequest {
                request_id: "private-inspect".to_owned(),
                action: KnowledgeAction::Inspect,
                knowledge_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    assert_eq!(initial.knowledge.items[0].title, "Unrevealed field clue");
    assert!(!initial.knowledge.items[0]
        .description
        .contains("moonberries"));

    let discovered = repository
        .knowledge(
            &discoverer.account_token,
            knowledge_request("discover-private", KnowledgeAction::Discover),
        )
        .unwrap()
        .data;
    assert!(discovered.accepted);
    assert_eq!(
        discovered.knowledge.items[0].title,
        "Moonberry trellis method"
    );

    let still_private = repository
        .knowledge(
            &observer.account_token,
            KnowledgeRequest {
                request_id: "private-inspect-after-discovery".to_owned(),
                action: KnowledgeAction::Inspect,
                knowledge_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    assert_eq!(
        still_private.knowledge.items[0].title,
        "Unrevealed field clue"
    );

    let taught = repository
        .knowledge(
            &discoverer.account_token,
            KnowledgeRequest {
                request_id: "teach-private".to_owned(),
                action: KnowledgeAction::Teach,
                knowledge_id: Some("moonberry-tending".to_owned()),
                target_account_id: Some(learner.account_id.clone()),
            },
        )
        .unwrap()
        .data;
    assert!(taught.accepted);
    let learner_view = repository
        .knowledge(
            &learner.account_token,
            KnowledgeRequest {
                request_id: "learner-inspect".to_owned(),
                action: KnowledgeAction::Inspect,
                knowledge_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    assert_eq!(
        learner_view.knowledge.items[0].title,
        "Moonberry trellis method"
    );

    let recorded = repository
        .knowledge(
            &discoverer.account_token,
            knowledge_request("record-private", KnowledgeAction::Record),
        )
        .unwrap()
        .data;
    assert!(recorded.accepted);
    let public_view = repository
        .knowledge(
            &observer.account_token,
            KnowledgeRequest {
                request_id: "public-inspect".to_owned(),
                action: KnowledgeAction::Inspect,
                knowledge_id: None,
                target_account_id: None,
            },
        )
        .unwrap()
        .data;
    assert_eq!(
        public_view.knowledge.items[0].title,
        "Moonberry trellis method"
    );
}
