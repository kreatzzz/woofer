//! the lyrics provider is tested through Wasm with canned HTTP answers.

use serde_json::{Value, json};
use woofer_plugin_sdk::harness::{Plugin, Response};

fn plugin() -> Plugin {
    Plugin::from_local_artifact("woofer-plugin-lyrics-ovh").expect("built plugin module")
}

fn input(artist: &str, title: &str) -> Value {
    json!({
        "kind": "lyrics",
        "target": "",
        "lines": [artist, title, "Album", "201000"]
    })
}

#[test]
fn the_manifest_declares_one_lyrics_domain() {
    let mut plugin = plugin();
    assert_eq!(plugin.abi_version().unwrap(), 1);
    let manifest: Value = serde_json::from_str(&plugin.manifest().unwrap()).unwrap();
    assert_eq!(manifest["id"], "lyrics-ovh");
    assert_eq!(manifest["capabilities"], json!(["provider:lyrics"]));
    assert_eq!(manifest["domains"], json!(["api.lyrics.ovh"]));
}

#[test]
fn the_plan_encodes_artist_and_title_as_path_segments() {
    let mut plugin = plugin();
    let plan: Value = serde_json::from_str(
        &plugin
            .plan(&input("Björk & Trio", "It's Oh So Quiet").to_string())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(plan["requests"].as_array().unwrap().len(), 1);
    let url = plan["requests"][0]["url"].as_str().unwrap();
    assert!(url.starts_with("https://api.lyrics.ovh/v1/"));
    assert!(url.contains("Bj%C3%B6rk%20%26%20Trio"));
    assert!(url.ends_with("It%27s%20Oh%20So%20Quiet"));
}

#[test]
fn a_hit_becomes_plain_nonempty_lines() {
    let mut plugin = plugin();
    let output: Value = serde_json::from_str(
        &plugin
            .fulfil(
                &input("Artist", "Song").to_string(),
                &[Response {
                    status: 200,
                    body: json!({ "lyrics": " first \r\n\r\nsecond\n" }).to_string(),
                }],
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(output["lyrics"]["synced"], false);
    assert_eq!(
        output["lyrics"]["lines"],
        json!([
            { "at_ms": Value::Null, "text": "first" },
            { "at_ms": Value::Null, "text": "second" }
        ])
    );
}

#[test]
fn missing_track_or_missing_identity_is_a_quiet_miss() {
    let mut plugin = plugin();
    let missing = plugin
        .fulfil(
            &input("Artist", "Song").to_string(),
            &[Response {
                status: 404,
                body: String::new(),
            }],
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&missing).unwrap()["miss"],
        true
    );

    let no_artist = input("", "Song");
    let plan: Value = serde_json::from_str(&plugin.plan(&no_artist.to_string()).unwrap()).unwrap();
    assert_eq!(plan["requests"], json!([]));
    let output: Value =
        serde_json::from_str(&plugin.fulfil(&no_artist.to_string(), &[]).unwrap()).unwrap();
    assert_eq!(output["miss"], true);
}

#[test]
fn upstream_breakage_is_an_error_for_the_health_counter() {
    let mut plugin = plugin();
    let output: Value = serde_json::from_str(
        &plugin
            .fulfil(
                &input("Artist", "Song").to_string(),
                &[Response {
                    status: 503,
                    body: String::new(),
                }],
            )
            .unwrap(),
    )
    .unwrap();
    assert!(output["error"].as_str().unwrap().contains("503"));
}
