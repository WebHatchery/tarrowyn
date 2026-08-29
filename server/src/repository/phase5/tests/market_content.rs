use super::super::super::WorldRepository;
use crate::ServerConfig;

#[test]
fn fresh_regional_stock_follows_settlement_content() {
    let repository = WorldRepository::new(ServerConfig::default());
    let state = repository.state.lock().expect("repository lock");

    assert_eq!(state.phase5.locations.len(), 3);
    assert_eq!(state.phase5.routes.len(), 3);
    assert_eq!(state.phase5.settlements.len(), 3);
    assert_eq!(state.phase5.stock.get("hearth:timber"), Some(&4));
    assert_eq!(state.phase5.stock.get("hearth:stone"), Some(&6));
    assert_eq!(
        state.phase5.stock.get("whisperwood-outpost:timber"),
        Some(&18)
    );
    assert_eq!(
        state.phase5.stock.get("whisperwood-outpost:stone"),
        Some(&2)
    );
    assert_eq!(state.phase5.stock.get("saltmere:stone"), Some(&20));
    assert_eq!(state.phase5.stock.get("saltmere:bandages"), Some(&12));
}
