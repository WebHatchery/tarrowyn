use super::*;

#[test]
fn online_footer_keeps_wallet_inventory_and_presence_visible() {
    let stats = "Gold 12  Skill 4  Reputation 3\nRank Wayfarer • 1 credentials\nField tool 2/3 • Clear • pests 0/2 • Goat 3/3\nWheat 5  Turnips 2  Moonberries 1  Seeds 4 • Bandages 2";

    assert_eq!(
        online_footer_detail(stats, 3),
        "Gold 12  Skill 4  Reputation 3 • 3 players\nWheat 5  Turnips 2  Moonberries 1  Seeds 4 • Bandages 2"
    );
}
