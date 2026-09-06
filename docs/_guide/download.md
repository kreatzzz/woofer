---
title: Download
description: Get Woofer for macOS, Windows, or Linux, with install instructions for each.
---

# Download Woofer

Woofer **v0.4.0 is published** with checksum-verified builds for Linux, macOS,
and Windows. Download the platform build from the
[GitHub release](https://github.com/kreatzzz/woofer/releases/tag/v0.4.0), or
install the macOS build through Homebrew:

```sh
brew install --cask kreatzzz/tap/woofer
```

## Build from source

You need a stable [Rust](https://rustup.rs) toolchain (1.95 or newer):

```sh
git clone https://github.com/kreatzzz/woofer
cd woofer
cargo install --path .
```

On Linux, install the desktop and audio development libraries first. On Arch:

```sh
sudo pacman -S --needed alsa-lib libpulse libxkbcommon wayland
```

On Debian or Ubuntu:

```sh
sudo apt install libasound2-dev libpulse-dev libxkbcommon-dev libwayland-dev libgl1-mesa-dev
```

The repository's [Nix development shell](https://nixos.org) provides the
same libraries and the pinned toolchain.

## What the release includes

The v0.4.0 GitHub release carries:

- a universal macOS DMG for Apple Silicon and Intel;
- Windows installers and portable archives for x86_64 and ARM64;
- Linux archives for x86_64 and ARM64;
- `checksums.txt` with a SHA-256 entry for every file.

Use the accompanying `checksums.txt` to verify a manual download. The
[release plan](/dev/release-plan) records the package-manager rollout and the
unsigned macOS first-open note.

## Platform notes

### macOS

The v0.4.0 DMG is unsigned. It asks you to approve Woofer once in **System
Settings → Privacy & Security**; later launches work normally. A signed and
notarized DMG skips that first-open warning when the release workflow's full
Apple contract is configured.

### Windows

The installer needs no administrator rights. SmartScreen may warn about an
unknown publisher the first time; choose **More info → Run anyway** after
checking the checksum from the release.

### Linux

The archive includes the binary and desktop integration files. Runtime needs
are the ordinary desktop libraries: ALSA, PulseAudio or PipeWire, and Wayland
or X11. The AUR package is not live yet.

## Package managers

The Homebrew cask is live. The initial winget submission is under review, and
the AUR package is waiting for maintainer authentication. Until those two are
published, use the GitHub release builds on Windows and Linux.
