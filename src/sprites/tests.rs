use super::*;

#[test]
fn foundation_content_kinds_select_authored_sprite_art() {
    let expected = [
        ("beacon", FoundationSprite::Beacon),
        ("tent_settlement", FoundationSprite::TentSettlement),
        ("gathering_place", FoundationSprite::GatheringFire),
        ("noticeboard", FoundationSprite::Noticeboard),
        ("shared_storage", FoundationSprite::SharedCache),
        ("crude_tools", FoundationSprite::ToolRack),
        ("rough_forge", FoundationSprite::RoughForge),
        ("construction_space", FoundationSprite::ConstructionSite),
    ];

    for (kind, sprite) in expected {
        assert_eq!(FoundationSprite::from_kind(kind), Some(sprite));
    }
    assert_eq!(FoundationSprite::from_kind("npc"), None);
    assert_eq!(FoundationSprite::from_kind("woodland"), None);
}
