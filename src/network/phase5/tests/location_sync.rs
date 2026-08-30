use super::*;
use macroquad_toolkit::grid::TilePos;
use tarrowyn_protocol::{LocationKind, LocationRecord, Position, RegionSnapshot};

fn location(id: &str, position: Position) -> LocationRecord {
    LocationRecord {
        location_id: id.to_owned(),
        name: id.to_owned(),
        kind: LocationKind::Settlement,
        position,
        role: "settlement".to_owned(),
        resources: vec!["seeds".to_owned()],
        services: vec!["market".to_owned()],
        condition: 70,
        access_note: "The road remains open.".to_owned(),
    }
}

#[test]
fn cached_region_follows_the_authoritative_player_location() {
    let mut client = Phase5Client::new();
    client.region = Some(RegionSnapshot {
        region_id: "hearthlands".to_owned(),
        season: "thaw".to_owned(),
        calendar_day: 1,
        locations: vec![
            location("hearth", Position { x: 8, y: 6 }),
            location("whisperwood-outpost", Position { x: 12, y: 4 }),
        ],
        routes: Vec::new(),
        visible_settlements: Vec::new(),
        player_location_id: "hearth".to_owned(),
        travel: None,
        interest_radius: 12,
        cursor: 1,
    });

    client.sync_player_location(TilePos::new(12, 4));
    assert_eq!(
        client.region.as_ref().unwrap().player_location_id,
        "whisperwood-outpost"
    );

    client.sync_player_location(TilePos::new(8, 6));
    assert_eq!(client.region.as_ref().unwrap().player_location_id, "hearth");
}
