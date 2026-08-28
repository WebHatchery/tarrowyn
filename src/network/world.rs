//! Apply server-owned crops to the client world projection.

use super::WorldProjection;
use macroquad_toolkit::grid::TilePos;

impl WorldProjection {
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
