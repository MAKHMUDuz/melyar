<div align="center">

# Melyar

**A local-first desktop music player with a Liquid Glass-inspired interface.**

[English](README.md) · [Türkçe](README.tr.md)

[![CI](https://github.com/MAKHMUDuz/melyar/actions/workflows/ci.yml/badge.svg)](https://github.com/MAKHMUDuz/melyar/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/MAKHMUDuz/melyar.svg)](https://github.com/MAKHMUDuz/melyar/releases/latest)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-APACHE)
[![Contributors](https://img.shields.io/github/contributors/MAKHMUDuz/melyar.svg)](https://github.com/MAKHMUDuz/melyar/graphs/contributors)
[![Discussions](https://img.shields.io/github/discussions/MAKHMUDuz/melyar?label=discussions&logo=github)](https://github.com/MAKHMUDuz/melyar/discussions)

</div>

---

Melyar brings music discovery, playback, lyrics, queues, and a personal listening
library into one desktop application: a **Rust core**, a **Qt/C++ desktop
bridge**, and a **React + TypeScript interface**. Playback is powered by
`libmpv`, and stream/metadata resolution goes through the YouTube `InnerTube`
API. The client version is read from the live web client at runtime and several
client identities are tried in turn, so one of them being refused does not stop
playback. `yt-dlp` is used when it is installed, but it is not required.

> **Project status: Work in progress.** Some UI interactions and provider
> integrations are still being refined, so expect changes and occasional bugs.
> **[Looking to help? Read the contributing guide.](CONTRIBUTING.md)**

---

## Why Melyar

There are other YouTube Music clients. Melyar exists to be the best one **on
Linux**, not the most portable one.

| | Melyar | Spotube | BloomeeTunes |
|---|---|---|---|
| Stack | **Rust + Qt6/QML + React** | Flutter/Dart | Flutter/Dart |
| Platforms | Linux | Win, macOS, mobile, web | Win, macOS, mobile, web |
| Runtime footprint | Small - no Flutter engine, no Electron tree | Large | Large |
| MPRIS (GNOME, KDE, i3) | Full | Partial | Partial |
| Waybar / Polybar IPC | Built in | No | No |
| License | **MIT OR Apache-2.0** | BSD-4 | GPL-2.0 |

Three bets: **native beats emulated** (a Rust core with a Qt shell starts fast
and stays small), **desktop integration is the product** (MPRIS, a Waybar
socket, media keys, a real `.desktop` entry), and **permissive licensing**, so
distributors can embed it. Where the competitors are still stronger - platform
reach and provider breadth - is stated plainly in the [roadmap](ROADMAP.md).

---

## Highlights

- **A ten band graphic equaliser**, 31 Hz to 16 kHz, built on mpv's filter
  chain - the one feature the other clients still do not have.
- **A lighter application:** the graphics stack is off, because a music player
  draws no video. That alone is 59 MB less resident memory; the interface is
  ready in about 650 ms.
- **An interface that is checked.** Every push drives the application through
  every view and every overlay and fails when something is wrong.
- **Playback** via mpv with stream resolution through the YouTube Music API.
  Several client identities are tried in turn, so the client keeps working when
  YouTube changes something.
- **Synchronized LRC lyrics** with timestamped highlighting and plain-text
  fallback, and queue management with gapless prefetch.
- **Library import** - bring a Spotify account export, a JSON list or a CSV
  file over from another service; Melyar matches every track against YouTube
  Music and adds what it recognises.
- **Personal library** views for liked tracks, history, songs, albums and
  artists, kept in a local database.
- **Thirteen languages** - English, Turkish, Spanish, German, French, Italian,
  Portuguese, Russian, Ukrainian, Japanese, Chinese, Korean and Albanian - and
  settings that survive a restart.
- **Desktop integration:** Linux MPRIS (D-Bus) and a zero-latency Unix socket
  IPC for Waybar / Polybar / i3status.
- **Local-first:** no Melyar account and no Melyar-hosted cloud service to
  launch.

Availability can vary by provider, track, operating system, and external
service behavior. See the [roadmap](ROADMAP.md) for integrations that are not
yet complete.

---

## Screenshots

![Melyar home view](docs/screenshots/home-en.png)

![Melyar home view in Turkish](docs/screenshots/home-tr.png)

> The screenshots were taken against a seeded sample library. Use
> `MELYAR_SCREENSHOT=/path/to.png ./build/melyar` to capture the window
> yourself - see the [contributing guide](CONTRIBUTING.md#capturing-a-screenshot).

---

## Architecture

```
React + TypeScript UI ──QWebChannel──▶ Qt / C++ bridge ──▶ Rust core ──▶ mpv
                                                            │
                                        provider & metadata logic
```

The C++ ↔ Rust boundary is the [`cxx`](https://cxx.rs) bridge. The Rust side
runs on a `tokio` runtime; audio is driven synchronously by `libmpv`.

| Directory | Purpose |
|---|---|
| `core_rs/` | Rust core: playback coordination, track models, lyrics, provider logic, local application data. |
| `ui_cpp/` | Qt/C++ integration and the bridge between the web interface and the core. |
| `ui_web/` | React + TypeScript interface, views, components, Liquid Glass styling. |

---

## Requirements

| Component | Version |
|---|---|
| Rust toolchain and Cargo | ≥ 1.75 |
| CMake | ≥ 3.25 |
| C/C++ build toolchain | C++20 support |
| Qt 6 | Core, Gui, Quick, Qml, WebEngineQuick, WebChannel |
| Node.js / pnpm | ≥ 20 / ≥ 9 |
| mpv | ≥ 2.x (`libmpv` development files) |
| pkg-config | — |
| yt-dlp | optional, used when it is installed |

<details>
<summary>Distribution examples (Arch / Manjaro, Debian / Ubuntu)</summary>

**Arch / Manjaro**

```bash
sudo pacman -S base-devel cmake rust pkgconf \
    qt6-base qt6-declarative qt6-webengine qt6-webchannel mpv
```

**Debian / Ubuntu**

```bash
sudo apt install build-essential cmake pkg-config rustc cargo \
    qt6-base-dev qt6-declarative-dev qt6-webengine-dev qt6-webchannel-dev \
    libmpv-dev yt-dlp
```

</details>

`yt-dlp` is optional: streams resolve without it, but it handles some of the
harder parts of the protocol, so if a stream will not resolve, confirm it is
installed, executable, and available in the environment used to launch Melyar.

---

## Build from source

```bash
git clone https://github.com/MAKHMUDuz/melyar.git
cd melyar

# The compiled web bundle is committed under ui_web/dist/, so this step is
# only required when the interface sources change.
cd ui_web && pnpm install && pnpm build && cd ..

cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j"$(nproc)"
./build/melyar
```

---

## Installation

### Prebuilt packages

Packages for Arch and Debian/Ubuntu, plus a binary tarball, are attached to
the [releases](https://github.com/MAKHMUDuz/melyar/releases/latest). No build
toolchain is needed.

**Debian / Ubuntu** (Ubuntu 24.04 or newer):

```bash
sudo apt install ./melyar_0.0.4-1_amd64.deb
```

**Arch** (the package and its dependencies come from the official repos):

```bash
sudo pacman -U melyar-0.0.4-1-x86_64.pkg.tar.zst
```

**Any distro** - unpack the tarball and install the files by hand:

```bash
tar xf melyar-0.0.4-linux-x86_64.tar.gz
sudo cp -r bin/melyar /usr/local/bin/
sudo cp -r share/melyar /usr/local/share/
sudo cp share/applications/melyar.desktop /usr/local/share/applications/
sudo cp -r share/icons/hicolor /usr/local/share/icons/
melyar
```

The binary links against your distribution's Qt 6 and glibc, which is why the
`.deb` is built on Ubuntu and the `.pkg.tar.zst` on Arch. If a package refuses
to install because of the glibc version, build from source instead - it is a
five minute build with the dependencies above.

### From source

```bash
sudo cmake --install build
```

CMake's default install prefix is `/usr/local` (`bin/melyar`,
`share/melyar/web/`, `share/applications/melyar.desktop`,
`share/icons/.../melyar.svg`). To install into the system directory instead,
configure with `-DCMAKE_INSTALL_PREFIX=/usr` before installing.

The application resolves the web UI in this order: current working directory →
next to the binary → `/usr/share/melyar/web/` → `/usr/local/share/melyar/web/`.
To point it at a live Vite dev server, set `MELYAR_DEV_URL`.

A `PKGBUILD` (`makepkg -si`) and a `debian/` recipe
(`dpkg-buildpackage -us -uc -b`) are both included, and continuous integration
builds the Debian package on every push.

---

## Desktop integration

**MPRIS.** Melyar answers on the session bus as
`org.mpris.MediaPlayer2.melyar`, so the tray applet, media keys, lock-screen
widget and volume OSD all drive it, and the title, artist, cover, position and
playback state appear in KDE Plasma and GNOME with no configuration. It works
in both directions: play, pause, next, previous, seeking and the desktop's
volume slider move Melyar, and repeat and shuffle are reported. The service is
part of the application - no extra dependency, no setting to turn on.

```bash
busctl --user get-property org.mpris.MediaPlayer2.melyar \
    /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player Metadata
```

**Waybar / status bars.** For bars that do not speak D-Bus, Melyar runs a
zero-latency Unix socket IPC server at `$XDG_RUNTIME_DIR/melyar.sock`
(commands: `toggle`, `next`, `prev`, `stop`):

```jsonc
"custom/melyar": {
    "format": "{icon} {}",
    "return-type": "json",
    "exec": "socat -u UNIX-CONNECT:${XDG_RUNTIME_DIR:-/tmp}/melyar.sock -",
    "on-click": "echo toggle | nc -U ${XDG_RUNTIME_DIR:-/tmp}/melyar.sock",
    "interval": 1
}
```

Full setup, CSS and Hyprland/Sway keybinding examples:
[`docs/WAYBAR.md`](docs/WAYBAR.md).

---

## Local data

| Data | Location |
|---|---|
| Database (favorites, history, lyrics cache) | `~/.local/share/melyar/melyar.db` |
| IPC socket | `$XDG_RUNTIME_DIR/melyar.sock` |

---

## Roadmap

The full plan, including the honest list of where Melyar still lags the
alternatives, lives in **[ROADMAP.md](ROADMAP.md)**: a solid Linux citizen
(settings, accessibility, crash-safe writes), a library that works
(import/export, local folders, playlists), provider breadth where it is
permitted, and cross-platform only if it earns its place.

Third-party services may require their own accounts, subscriptions, or API
access. Melyar does not make paid features of another service free.

---

## Development

```bash
cargo fmt --all
cargo clippy --manifest-path core_rs/Cargo.toml --all-targets
cargo test --manifest-path core_rs/Cargo.toml
cd ui_web && pnpm build   # then rebuild the desktop app if Qt assets changed
```

Every push runs these checks plus a full Qt6 build, an interface audit at
three window sizes in both themes, and a memory budget check via
[GitHub Actions](.github/workflows/ci.yml). Before submitting a change, test
the affected feature in the actual desktop application - not only in a browser
preview.

---

## Contributing

Contributions, bug reports and focused pull requests are welcome - **new
contributors are actively wanted**, see the
[good first issues](ROADMAP.md#good-first-issues). The full guide is in
**[CONTRIBUTING.md](CONTRIBUTING.md)**, and the system design in
**[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)**.

Keep changes focused, preserve working playback and bridge behavior, and do
not commit build output, credentials or personal data. For UI changes include
screenshots; for playback or lyrics fixes, include reproducible steps and logs
with secrets removed.

---

## Bug reports

Use the **bug report template** - version, desktop environment, expected and
actual behavior, reproducible steps. For layout issues include the window size
and whether the queue or expanded player was open; for lyrics issues, the
track, playback position and whether the lyrics are timestamped.

**Security problems** are not public issues - see
[.github/SECURITY.md](.github/SECURITY.md). Participation is governed by the
[Code of Conduct](.github/CODE_OF_CONDUCT.md).

---

## License

Melyar is dual-licensed under **MIT** ([`LICENSE-MIT`](LICENSE-MIT)) and
**Apache License 2.0** ([`LICENSE-APACHE`](LICENSE-APACHE)).
`SPDX-License-Identifier: MIT OR Apache-2.0`

---

## Notes

- Melyar builds on open-source software and is inspired by modern desktop
  music-player design and Liquid Glass visual concepts. Third-party projects
  remain the property of their respective authors under their own licenses.
- Melyar is an unofficial client, not affiliated with, endorsed by or
  sponsored by YouTube, Google, Spotify or any other provider. Content
  licenses, account terms and the laws of your jurisdiction apply to you as
  the end user.

---

<div align="center">
<sub>Built with Rust, Qt 6, React and a lot of care for the details.</sub>
</div>
