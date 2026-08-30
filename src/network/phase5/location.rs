use super::Phase5Client;
use macroquad_toolkit::grid::TilePos;

impl Phase5Client {
    pub(crate) fn sync_player_location(&mut self, position: TilePos) {
        let Some(region) = self.region.as_mut() else {
            return;
        };
        let location_id = region
            .locations
            .iter()
            .min_by_key(|location| {
                TilePos::new(location.position.x, location.position.y).manhattan_distance(&position)
            })
            .map(|location| location.location_id.clone());
        if let Some(location_id) = location_id {
            region.player_location_id = location_id;
        }
    }
}
