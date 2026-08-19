use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

#[test]
fn asset_registry_contains_external_texture_manifest_paths() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let registry_json = fs::read_to_string(root.join("asset_registry.json"))
        .expect("asset_registry.json must be readable");
    let registry: Value =
        serde_json::from_str(&registry_json).expect("asset registry must be valid JSON");
    assert_eq!(registry["version"], 1);

    let registered: BTreeSet<&str> = registry["assets"]
        .as_array()
        .expect("asset registry needs an assets array")
        .iter()
        .map(|entry| entry.as_str().expect("asset paths must be strings"))
        .collect();

    let manifest_json = fs::read_to_string(root.join("assets/data/texture_manifest.json"))
        .expect("texture manifest must be readable");
    let manifest: Vec<Value> =
        serde_json::from_str(&manifest_json).expect("texture manifest must be valid JSON");
    let expected: BTreeSet<&str> = manifest
        .iter()
        .map(|entry| {
            entry["path"]
                .as_str()
                .expect("each texture manifest entry needs a path")
        })
        .collect();

    let missing: Vec<&str> = expected.difference(&registered).copied().collect();
    assert!(
        missing.is_empty(),
        "texture manifest paths missing from asset registry: {missing:?}"
    );
    for relative in registered {
        assert!(
            root.join(relative).is_file(),
            "registered runtime asset is missing: {relative}"
        );
    }
}
