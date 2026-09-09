//! offline romanization for lyric lines, backed by AnyAscii's Unicode
//! tables. The module asks the host for no network access.

use any_ascii::any_ascii;
use serde::Deserialize;
use serde_json::json;

use woofer_plugin_sdk::register_plugin;

const MANIFEST: &str = r#"{
    "id": "romanize-offline",
    "name": "Offline Romanizer",
    "publisher": "kreatzzz",
    "version": "1.0.0",
    "api": 1,
    "capabilities": ["provider:romanize"],
    "domains": [],
    "homepage": "https://github.com/kreatzzz/woofer/tree/main/plugins/romanize-offline"
}"#;

register_plugin! {
    manifest = MANIFEST,
    plan = plan,
    fulfil = fulfil,
}

#[derive(Deserialize)]
struct Input {
    kind: String,
    lines: Vec<String>,
}

fn parse(input: &str) -> Result<Input, String> {
    let input: Input =
        serde_json::from_str(input).map_err(|error| format!("malformed input: {error}"))?;
    if input.kind != "romanize" {
        return Err(format!(
            "offline romanizer serves \"romanize\", not \"{}\"",
            input.kind
        ));
    }
    Ok(input)
}

fn plan(input: &str) -> Result<String, String> {
    parse(input)?;
    Ok(json!({ "requests": [] }).to_string())
}

fn fulfil(input: &str) -> Result<String, String> {
    let input = parse(input)?;
    let romanized = input
        .lines
        .iter()
        .map(|line| {
            if line.is_ascii() {
                return None;
            }
            let text = any_ascii(line);
            (!text.trim().is_empty() && text != *line).then_some(text)
        })
        .collect::<Vec<_>>();
    let translated = vec![serde_json::Value::Null; input.lines.len()];
    Ok(json!({
        "romanized": romanized,
        "translated": translated,
    })
    .to_string())
}
