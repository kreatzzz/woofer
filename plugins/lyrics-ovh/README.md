# Lyrics.ovh

An opt-in `provider:lyrics` plugin for Woofer. It asks
`api.lyrics.ovh/v1/{artist}/{title}` for plain lyrics after Woofer's Spotify and
LRCLIB sources miss. HTTP 404 is a provider miss; malformed or unavailable
upstream responses count as failures and follow the normal provider health
rules.

```bash
cargo test
cargo build --release --target wasm32-unknown-unknown
```

The resulting module is
`target/wasm32-unknown-unknown/release/woofer_plugin_lyrics_ovh.wasm`.
