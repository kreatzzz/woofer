//! a plain-lyrics fallback backed by lyrics.ovh. Spotify and LRCLIB remain
//! ahead of this provider in Woofer's lyrics flow.

use serde::Deserialize;
use serde_json::json;

use woofer_plugin_sdk::register_plugin;

const MANIFEST: &str = r#"{
    "id": "lyrics-ovh",
    "name": "Lyrics.ovh",
    "publisher": "kreatzzz",
    "version": "1.0.0",
    "api": 1,
    "capabilities": ["provider:lyrics"],
    "domains": ["api.lyrics.ovh"],
    "homepage": "https://github.com/kreatzzz/woofer/tree/main/plugins/lyrics-ovh"
}"#;

const API: &str = "https://api.lyrics.ovh/v1";

register_plugin! {
    manifest = MANIFEST,
    plan = plan,
    fulfil = fulfil,
}

#[derive(Deserialize)]
struct Input {
    kind: String,
    lines: Vec<String>,
    #[serde(default)]
    responses: Vec<Answer>,
}

#[derive(Deserialize)]
struct Answer {
    #[serde(default)]
    status: u16,
    #[serde(default)]
    body: String,
}

#[derive(Deserialize)]
struct LyricsAnswer {
    lyrics: String,
}

fn parse(input: &str) -> Result<Input, String> {
    let input: Input =
        serde_json::from_str(input).map_err(|error| format!("malformed input: {error}"))?;
    if input.kind != "lyrics" {
        return Err(format!(
            "lyrics.ovh serves \"lyrics\", not \"{}\"",
            input.kind
        ));
    }
    if input.lines.len() != 4 {
        return Err(format!(
            "lyrics input needs artist, title, album, and duration; got {} fields",
            input.lines.len()
        ));
    }
    Ok(input)
}

fn query(input: &Input) -> Option<String> {
    let artist = input.lines[0].trim();
    let title = input.lines[1].trim();
    if artist.is_empty() || title.is_empty() {
        return None;
    }
    Some(format!(
        "{API}/{}/{}",
        urlencoding::encode(artist),
        urlencoding::encode(title)
    ))
}

fn plan(input: &str) -> Result<String, String> {
    let input = parse(input)?;
    let requests = query(&input)
        .map(|url| vec![json!({ "url": url })])
        .unwrap_or_default();
    Ok(json!({ "requests": requests }).to_string())
}

fn fulfil(input: &str) -> Result<String, String> {
    let input = parse(input)?;
    if query(&input).is_none() {
        return Ok(json!({ "miss": true }).to_string());
    }
    let Some(answer) = input.responses.first() else {
        return Err("expected one lyrics.ovh answer, got none".into());
    };
    if input.responses.len() != 1 {
        return Err(format!(
            "expected one lyrics.ovh answer, got {}",
            input.responses.len()
        ));
    }
    if answer.status == 404 {
        return Ok(json!({ "miss": true }).to_string());
    }
    if !(200..300).contains(&answer.status) {
        return Err(format!("lyrics.ovh answered {}", answer.status));
    }
    let answer: LyricsAnswer = serde_json::from_str(&answer.body)
        .map_err(|error| format!("unexpected answer from lyrics.ovh: {error}"))?;
    let lines = answer
        .lyrics
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|text| json!({ "at_ms": null, "text": text }))
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Ok(json!({ "miss": true }).to_string());
    }
    Ok(json!({
        "lyrics": {
            "synced": false,
            "lines": lines,
        }
    })
    .to_string())
}
