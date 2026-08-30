//! Apply server-owned crops to the client world projection.

use super::WorldProjection;
use macroquad_toolkit::grid::TilePos;

impl WorldProjection {
    pub(crate) fn response_is_current(&self, server_tick: u64, cursor: u64) -> bool {
        server_tick >= self.server_tick && cursor >= self.cursor
    }

    pub(crate) fn response_is_newer(&self, server_tick: u64, cursor: u64) -> bool {
        server_tick >= self.server_tick && cursor > self.cursor
    }

    pub(crate) fn accept_response_version(
        &mut self,
        server_tick: u64,
        cursor: Option<u64>,
    ) -> bool {
        let version_cursor = cursor.unwrap_or(self.cursor);
        let current = self.response_is_current(server_tick, version_cursor);
        self.record_response_version(server_tick, cursor);
        current
    }

    pub(crate) fn record_response_version(&mut self, server_tick: u64, cursor: Option<u64>) {
        self.server_tick = self.server_tick.max(server_tick);
        if let Some(cursor) = cursor {
            self.cursor = self.cursor.max(cursor);
        }
    }

    pub(crate) fn authoritative_player_position(&self) -> Option<TilePos> {
        self.player_position_authoritative
            .then_some(self.player_position)
    }

    pub(crate) fn set_authoritative_player_position(&mut self, position: TilePos) {
        self.player_position = position;
        if let Some(player) = self.player.as_mut() {
            player.position = tarrowyn_protocol::Position {
                x: position.x,
                y: position.y,
            };
        }
        self.player_position_authoritative = true;
    }

    pub(crate) fn forget_authoritative_player_position(&mut self) {
        self.player_position_authoritative = false;
    }

    pub(super) fn apply_plots(&mut self, plots: &[tarrowyn_protocol::FarmPlot]) {
        for plot in plots {
            self.apply_plot(*plot);
        }
    }

    pub(super) fn apply_plot(&mut self, plot: tarrowyn_protocol::FarmPlot) {
        let crop = plot.crop.map(|crop| crate::state::CropState {
            kind: match crop.kind {
                tarrowyn_protocol::CropKind::Wheat => crate::state::CropKind::Wheat,
                tarrowyn_protocol::CropKind::Turnip => crate::state::CropKind::Turnip,
                tarrowyn_protocol::CropKind::Moonberry => crate::state::CropKind::Moonberry,
            },
            stage: crop.stage,
        });
        self.world
            .crops
            .set(TilePos::new(plot.position.x, plot.position.y), crop);
    }
}
