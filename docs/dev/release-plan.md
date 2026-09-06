---
title: Release plan
description: The reproducible runbook for publishing Woofer builds and package-manager manifests.
---

# Release and packaging runbook

State: **v0.4.0 was published with verified assets on Sep 5, 2026**.
The Homebrew cask is live, the winget submission is open as
`microsoft/winget-pkgs#429939`, and AUR is waiting for an authenticated
maintainer SSH key. The repository version remains `0.4.0` in `Cargo.toml`.

## 0. Cut, verify, and publish a release

The release workflow builds and checks the complete matrix before publishing.
It also accepts a manual dispatch: a branch run is an isolated smoke build
named `<version>-dev.<run-number>`, while a dispatch from a release tag can
publish only when the `publish` input is explicitly set to `true`.

For a verified tag that has not yet been published, start the publish pass
from that tag:

```bash
gh workflow run Release --repo kreatzzz/woofer --ref vX.Y.Z -f publish=true
```

The assemble job validates every name, archive path, and checksum before the
publish job uploads anything. Each successful build contains:

- Linux x86_64 and arm64 portable `.tar.gz` archives plus AppImages, built
  with pinned, SHA-256-verified AppImage tools;
- Windows x86_64 and arm64 portable `.zip` archives plus per-user Inno Setup
  installers;
- one universal macOS DMG; and
- a sorted `checksums.txt` covering every asset.

Windows Authenticode signing is optional, enabled only when both documented
certificate secrets exist. macOS signing and notarization are optional too,
but require the complete six-secret Apple contract; otherwise the DMG is
ad-hoc/unsigned and needs the usual first-open approval. Tag-push smoke runs
never receive signing secrets.

## 1. Homebrew tap (live for v0.4.0)

The cask lives at `kreatzzz/homebrew-tap/Casks/woofer.rb`; users install it
with `brew install --cask kreatzzz/tap/woofer`. The setup steps below are kept
as the template for another tap or maintainer.

```bash
gh repo create kreatzzz/homebrew-tap --public --clone=false
git clone https://github.com/kreatzzz/homebrew-tap /tmp/tap && cd /tmp/tap
mkdir -p Casks
```

`Casks/woofer.rb`:

```ruby
cask "woofer" do
  version "0.4.0"
  sha256 "SHA256_OF_THE_DMG"   # from the release's checksums.txt

  url "https://github.com/kreatzzz/woofer/releases/download/v#{version}/woofer-v#{version}-macos-universal.dmg"
  name "Woofer"
  desc "Fast, lightweight, native Spotify client"
  homepage "https://github.com/kreatzzz/woofer"

  livecheck do
    url :url
    strategy :github_latest
  end

  app "Woofer.app"
end
```

Once the release assets are public, the repository's handoff helper renders
this cask with the real DMG hash (and renders the AUR and winget files at the
same time):

```bash
bash packaging/release/prepare-packages.sh \
  --dist /path/to/woofer-release-assets \
  --version 0.4.0 \
  --output-dir /tmp/woofer-packages
```

It verifies every selected asset against `checksums.txt` before writing any
manifest, so package files are not prepared with placeholder hashes.

Commit, push. Users: `brew install kreatzzz/tap/woofer`. Every future
release: bump `version` + `sha256` in one tap commit. If the first DMG is
unsigned, put the first-open note in the README: right-click → Open, or
`xattr -cr /Applications/Woofer.app`. Add the direct release URL only after
the explicit publish pass has created a public asset.

## 2. AUR (waiting for the maintainer's aur.archlinux.org SSH key)

Two packages, each pushed to `aur@aur.archlinux.org:<pkg>.git`:

**`woofer`** (release tarball, x86_64):

```sh
pkgname=woofer
pkgver=0.4.0
pkgrel=1
pkgdesc="Fast, lightweight, native Spotify client built with Rust and egui"
arch=('x86_64')
url="https://github.com/kreatzzz/woofer"
license=('MIT')
depends=('alsa-lib' 'libpulse' 'wayland' 'libxkbcommon')
conflicts=('woofer-git')
source=("$pkgname-$pkgver.tar.gz::$url/releases/download/v$pkgver/woofer-v$pkgver-x86_64-unknown-linux-gnu.tar.gz")
sha256sums=('FILL_FROM_checksums.txt')
package() {
  cd "$srcdir/$pkgname-v$pkgver-x86_64-unknown-linux-gnu"
  install -Dm755 woofer "$pkgdir/usr/bin/woofer"
  install -Dm644 packaging/applications/woofer.desktop \
    "$pkgdir/usr/share/applications/woofer.desktop"
  install -Dm644 packaging/icons/woofer.svg \
    "$pkgdir/usr/share/icons/hicolor/scalable/apps/woofer.svg"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/woofer/LICENSE"
}
```

Validate with `makepkg -f` (or `namcap`), then
`git init && git add . && git commit && git push aur:woofer.git`. Test with
`yay -S woofer` in a clean chroot if possible.

The helper writes the release package to
`/tmp/woofer-packages/aur/woofer/PKGBUILD` and the rolling package to
`/tmp/woofer-packages/aur/woofer-git/PKGBUILD`. The release package receives
the verified x86_64 archive hash; the `-git` package deliberately uses
`sha256sums=('SKIP')` because its source is a moving Git checkout.

**`woofer-git`**: same shape, `makedepends=('cargo' 'git')`,
`source=("$pkgname::git+$url.git")`, `pkgver()` from `git describe`,
build with `cargo build --release --locked`, same `package()` installs.

## 3. winget (v0.4.0 submission open)

The initial submission is
[`microsoft/winget-pkgs#429939`](https://github.com/microsoft/winget-pkgs/pull/429939).
For future releases:

1. Fork `microsoft/winget-pkgs` under `kreatzzz`, shallow-clone it.
2. Add `manifests/k/kreatzzz/Woofer/0.4.0/` with three YAML files:
   - `kreatzzz.Woofer.yaml` — the version manifest with
     `DefaultLocale: en-US`.
   - `kreatzzz.Woofer.installer.yaml` — `InstallerType: inno`, both
     architectures pointing at the GitHub `…-setup.exe` URLs, each with
     its sha256 from `checksums.txt`, and
     `InstallerSwitches: { Silent: /VERYSILENT /SUPPRESSMSGBOXES /NORESTART,
     SilentWithProgress: /SILENT }`, `AppsAndFeaturesEntries` with
     `ProductCode` if the .iss declares one.
   - `kreatzzz.Woofer.locale.en-US.yaml` — the default English locale with
     publisher, description, tags, and homepage.
   The helper writes these three files below
   `/tmp/woofer-packages/winget/manifests/k/kreatzzz/Woofer/0.4.0/` and fills
   both installer hashes from the published `checksums.txt`.
3. Validate locally with `winget validate` (on Windows) or the
   `winget-pkgs` CI, then open the PR with their template. The bot
   verifies the URLs and hashes; a human reviews (1-3 days). First-time
   submitters need a seasoned GitHub account — having the tap and AUR
   package public first helps the review.

Future versions: a new version folder per release.

## After each release

- Bump the tap (version + sha256) — one commit.
- Keep `docs/_guide/download.md` aligned with the package managers that are
  actually live.
- The update checker (`src/updates.rs`) watches
  `kreatzzz/woofer/releases/latest`.
