//! the offline romanizer is exercised through the same Wasm ABI as the host.

use serde_json::{Value, json};
use woofer_plugin_sdk::harness::Plugin;

fn plugin() -> Plugin {
    Plugin::from_local_artifact("woofer-plugin-romanize-offline").expect("built plugin module")
}

#[test]
fn the_manifest_needs_no_network() {
    let mut plugin = plugin();
    assert_eq!(plugin.abi_version().unwrap(), 1);
    let manifest: Value = serde_json::from_str(&plugin.manifest().unwrap()).unwrap();
    assert_eq!(manifest["id"], "romanize-offline");
    assert_eq!(manifest["capabilities"], json!(["provider:romanize"]));
    assert_eq!(manifest["domains"], json!([]));
}

#[test]
fn planning_never_asks_for_a_fetch() {
    let mut plugin = plugin();
    let input = json!({
        "kind": "romanize",
        "target": "en",
        "lines": ["άνθρωποι", "深圳"]
    });
    let plan: Value = serde_json::from_str(&plugin.plan(&input.to_string()).unwrap()).unwrap();
    assert_eq!(plan["requests"], json!([]));
}

#[test]
fn non_latin_lines_are_romanized_without_replacing_ascii_lines() {
    let mut plugin = plugin();
    let input = json!({
        "kind": "romanize",
        "target": "en",
        "lines": ["άνθρωποι", "hello", "", "深圳"]
    });
    let output: Value =
        serde_json::from_str(&plugin.fulfil(&input.to_string(), &[]).unwrap()).unwrap();
    assert_eq!(output["romanized"][0], "anthropoi");
    assert_eq!(output["romanized"][1], Value::Null);
    assert_eq!(output["romanized"][2], Value::Null);
    assert!(
        output["romanized"][3]
            .as_str()
            .is_some_and(|line| !line.is_empty())
    );
    assert_eq!(
        output["translated"],
        json!([Value::Null, Value::Null, Value::Null, Value::Null])
    );
}

#[test]
fn another_provider_kind_is_refused() {
    let mut plugin = plugin();
    let input = json!({ "kind": "translate", "target": "en", "lines": ["γεια"] });
    let output: Value = serde_json::from_str(&plugin.plan(&input.to_string()).unwrap()).unwrap();
    assert!(output["error"].as_str().unwrap().contains("translate"));
}
