use super::*;

#[test]
fn crafting_challenge_moves_across_a_wide_target() {
    let mut client = Phase4Client::new();
    client.begin_crafting("service-order-1");
    let before = client.crafting_view().unwrap();
    advance_crafting(&mut client.crafting, 1.0);
    let after = client.crafting_view().unwrap();
    assert!(after.0 > before.0);
    assert_eq!(after.1, 0.38);
    assert_eq!(after.2, 0.66);
}

#[test]
fn crafting_timing_pauses_during_authoritative_reload() {
    let mut client = Phase4Client::new();
    client.begin_crafting("service-order-reload");
    let before = client.crafting_view().unwrap().0;
    let mut api = HttpClient::new("https://example.test");
    let data = crate::data::GameData::load().expect("embedded game data should load");
    let mut projection = WorldProjection::new(&data.config);
    let mut notices = Vec::new();

    client.update_with_mode(
        2.0,
        &mut api,
        &mut projection,
        MutationContext {
            online: true,
            another_mutation_pending: false,
            session_only: true,
        },
        &mut notices,
    );

    assert_eq!(client.crafting_view().unwrap().0, before);
}
