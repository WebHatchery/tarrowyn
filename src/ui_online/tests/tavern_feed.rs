use super::super::tavern_feed_line;

#[test]
fn startup_board_notice_yields_to_the_current_rumour() {
    let board_notice = tarrowyn_protocol::TavernNotice {
        notice_id: 1,
        kind: "settlement".to_owned(),
        text: "The Hearth notice board is open; bring useful things to one another.".to_owned(),
        created_tick: 0,
        cursor: 1,
    };

    assert_eq!(
        tavern_feed_line(
            &[board_notice],
            &["The north road is costly while the Brambleback prowls.".to_owned()],
        ),
        Some("Tavern rumour: The north road is costly while the Brambleback prowls.".to_owned())
    );
}
