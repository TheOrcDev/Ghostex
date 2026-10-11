//! The desktop keeps its own copy of the default chords (`GPUI_DEFAULT_GHOSTEX_HOTKEYS` in
//! apps/desktop/src/app/hotkeys.rs) because it overlays defaults at read time and never persists
//! them. Repo policy keeps tests out of apps/desktop/, so this reads that table from source and
//! checks it against the catalog: adding, removing, renaming or rebinding a default on one side
//! without the other fails here. Moved from packages/shared/gpui-hotkey-defaults-parity.test.ts
//! when the hotkey catalog became Rust.

use std::collections::BTreeMap;

fn desktop_defaults() -> BTreeMap<String, String> {
    let source = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../apps/desktop/src/app/hotkeys.rs"
    ))
    .expect("apps/desktop/src/app/hotkeys.rs");
    let start = source
        .find("const GPUI_DEFAULT_GHOSTEX_HOTKEYS")
        .expect("GPUI_DEFAULT_GHOSTEX_HOTKEYS");
    let end = start + source[start..].find("];").expect("end of the table");
    let mut defaults = BTreeMap::new();
    for line in source[start..end].lines() {
        let quoted: Vec<&str> = line.split('"').skip(1).step_by(2).collect();
        if let [action_id, default_key] = quoted[..] {
            let previous = defaults.insert(action_id.to_string(), default_key.to_string());
            assert!(previous.is_none(), "duplicate Rust default for {action_id}");
        }
    }
    defaults
}

#[test]
fn desktop_default_hotkeys_match_the_catalog() {
    let desktop = desktop_defaults();
    // Guard the extraction itself: an extraction that finds nothing must fail loudly.
    assert!(
        desktop.len() >= 40,
        "found only {} desktop defaults",
        desktop.len()
    );
    let catalog: BTreeMap<String, String> = ghostex_settings_catalog::default_hotkeys()
        .into_iter()
        .map(|(id, key)| (id.to_string(), key.to_string()))
        .collect();
    assert_eq!(desktop, catalog);
}
