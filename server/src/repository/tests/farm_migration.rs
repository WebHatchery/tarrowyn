use super::*;

#[test]
fn fresh_world_farm_plots_follow_the_validated_region_manifest() {
    let repository = repo();
    let session = guest(&repository, "manifest-farm-plots");
    let world = repository.world(&session.account_token).unwrap().data;
    let expected = crate::content::farm_plot_positions();

    assert_eq!(
        world
            .plots
            .iter()
            .map(|plot| plot.position)
            .collect::<Vec<_>>(),
        expected
    );
    for position in expected {
        assert_eq!(
            world
                .tiles
                .iter()
                .find(|tile| tile.position == position)
                .map(|tile| tile.kind),
            Some(TileKind::Field)
        );
    }
}

#[test]
fn empty_legacy_farm_layout_upgrades_without_moving_crop_state() {
    let legacy = [(3, 4), (3, 5), (4, 4), (4, 5), (5, 4), (5, 5)]
        .into_iter()
        .map(|(x, y)| tarrowyn_protocol::FarmPlot {
            position: Position { x, y },
            crop: None,
        })
        .collect();

    let restored = super::super::world::restore_plots(legacy, 0, true);

    assert_eq!(
        restored
            .iter()
            .map(|plot| plot.position)
            .collect::<Vec<_>>(),
        crate::content::farm_plot_positions()
    );
    assert!(restored.iter().all(|plot| plot.crop.is_none()));
}

#[test]
fn populated_legacy_farm_layout_remains_unchanged() {
    let stored = [(3, 4), (3, 5), (4, 4), (4, 5), (5, 4), (5, 5)]
        .into_iter()
        .enumerate()
        .map(|(index, (x, y))| tarrowyn_protocol::FarmPlot {
            position: Position { x, y },
            crop: (index == 0).then_some(CropState {
                kind: CropKind::Wheat,
                stage: 1,
                quality: 2,
                planted_tick: 4,
                growth_ticks: 0,
                last_tended_tick: None,
            }),
        })
        .collect::<Vec<_>>();

    assert_eq!(
        super::super::world::restore_plots(stored.clone(), 5, false),
        stored
    );
}
