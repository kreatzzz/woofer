# Offline Romanizer

An opt-in `provider:romanize` plugin for Woofer. It converts non-ASCII lyric
lines with AnyAscii's Unicode tables and plans no HTTP requests. The manifest
therefore declares an empty domain allowlist.

```bash
cargo test
cargo build --release --target wasm32-unknown-unknown
```

The resulting module is
`target/wasm32-unknown-unknown/release/woofer_plugin_romanize_offline.wasm`.
See `THIRD-PARTY-LICENSES.md` for AnyAscii's ISC license.
