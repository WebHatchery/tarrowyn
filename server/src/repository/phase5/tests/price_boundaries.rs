#[test]
fn indexed_price_saturates_malformed_manifest_values() {
    assert_eq!(
        super::super::logic::indexed_price(u32::MAX, u16::MAX),
        u32::MAX / 100
    );
}
