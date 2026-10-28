<div align="center">

# Melyar

**Ultra-lightweight native desktop client for YouTube Music**

[![CI](https://github.com/MAKHMUDuz/melyar/actions/workflows/ci.yml/badge.svg)](https://github.com/MAKHMUDuz/melyar/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/MAKHMUDuz/melyar.svg)](https://github.com/MAKHMUDuz/melyar/releases/latest)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-APACHE)
[![Discussions](https://img.shields.io/github/discussions/MAKHMUDuz/melyar?label=discussions&logo=github)](https://github.com/MAKHMUDuz/melyar/discussions)

</div>

Melyar is a Linux-first, local-first desktop music client for YouTube Music. It combines a fast Rust core, a native Qt/C++ shell, and a polished React + TypeScript interface to deliver a clean, responsive listening experience without the overhead of heavier desktop frameworks.

![Melyar home view](docs/screenshots/home-en.png)

## Why Melyar

Melyar is built for people who want a music player that feels native on Linux:

- Small runtime footprint
- Fast startup and responsive controls
- Deep desktop integration with MPRIS and Waybar
- No account requirement to launch the app
- Local-first behavior with personal library data stored on disk

## Highlights

- Search, browse, and play YouTube Music content
- Playback with `libmpv` and gapless streaming behavior
- Queue management, shuffle, repeat, volume, and seek controls
- Synced LRC lyrics with plain-text fallback
- Favorites, history, albums, artists, and library views
- Library import from a Spotify export, JSON or CSV
- Ten band graphic equaliser (31 Hz - 16 kHz)
- Liquid Glass-inspired interface with light/dark theming
- Thirteen interface languages, switched at runtime
- Native Linux media integration via MPRIS and Unix socket IPC

## Screenshots

![Melyar home view in Turkish](docs/screenshots/home-tr.png)

## Architecture

```text
React + TypeScript UI
        │
        │ QWebChannel
        ▼
Qt / C++ bridge
        │
        ▼
Rust core ── playback, library, provider logic, IPC
        │
        ▼
mpv audio playback
```

The C++ ↔ Rust boundary is connected via the `cxx` bridge, while the UI communicates with the core through Qt/WebChannel.

## Install

Arch and Debian/Ubuntu packages are attached to the
[latest release](https://github.com/MAKHMUDuz/melyar/releases/latest). No
build toolchain is needed:

**Debian / Ubuntu** (Ubuntu 24.04 or newer):

```bash
sudo apt install ./melyar_*_amd64.deb
```

**Arch** (dependencies come from the official repos):

```bash
sudo pacman -U melyar-*.pkg.tar.zst
```

The packages use their distribution's Qt 6 and glibc. For other distributions,
build from source below so the binary links against the Qt and system libraries
available on that machine.

You can also install from source once it is built:

```bash
sudo cmake --install build
```

CMake installs to `/usr/local` by default (`bin/melyar`,
`share/melyar/web/`, `share/applications/melyar.desktop`,
`share/icons/.../melyar.svg`). Configure with `-DCMAKE_INSTALL_PREFIX=/usr`
to install into the system directory instead. A `PKGBUILD` (`makepkg -si`)
and a `debian/` recipe (`dpkg-buildpackage -us -uc -b`) are included in the
repository. CI builds and validates both packages on every push; version tags
publish the validated packages to GitHub Releases.

## Build from source

### Requirements

- Rust ≥ 1.88 (install with rustup; Ubuntu 24.04's packaged Rust is too old)
- CMake ≥ 3.25
- Qt 6 (`Core`, `Gui`, `Quick`, `Qml`, `WebEngineQuick`, `WebChannel`)
- Node.js ≥ 20
- pnpm ≥ 10
- mpv ≥ 2.x with development files
- `pkg-config`
- `yt-dlp` (optional, used when installed)

<details>
<summary>Distribution examples (Arch / Manjaro, Debian / Ubuntu)</summary>

**Arch / Manjaro**

```bash
sudo pacman -S base-devel cmake rust pkgconf \
    qt6-base qt6-declarative qt6-webengine qt6-webchannel qt6-svg mpv
```

**Debian / Ubuntu**

```bash
sudo apt install build-essential cmake pkg-config \
    qt6-base-dev qt6-declarative-dev qt6-webengine-dev qt6-webchannel-dev \
    qml6-module-qtquick qml6-module-qtquick-controls \
    qml6-module-qtquick-dialogs qml6-module-qtquick-layouts \
    qml6-module-qtquick-templates qml6-module-qtquick-window \
    qml6-module-qtqml-workerscript qml6-module-qtwebchannel \
    qml6-module-qtwebengine libqt6svg6 libmpv-dev yt-dlp
```

</details>

Install Rust 1.88 or newer with [rustup](https://rustup.rs/) before building.

### Build and run

```bash
git clone https://github.com/MAKHMUDuz/melyar.git
cd melyar

cd ui_web
pnpm install
pnpm build
cd ..

cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j"$(nproc)"
./build/melyar
```

## Desktop integration

### MPRIS

Melyar registers itself on the session bus as `org.mpris.MediaPlayer2.melyar`, so Linux desktops can discover playback state without any manual configuration.

### Waybar / status bar

Melyar also exposes a zero-latency Unix socket at `$XDG_RUNTIME_DIR/melyar.sock` for tools such as Waybar, Polybar, and i3status.

Full setup, CSS and Hyprland/Sway keybinding examples: [`docs/WAYBAR.md`](docs/WAYBAR.md).

## Local data

| Data | Location |
|---|---|
| Database | `~/.local/share/melyar/melyar.db` |
| IPC socket | `$XDG_RUNTIME_DIR/melyar.sock` |

## Roadmap

Melyar is actively under development. The project aims to improve Linux desktop integration, library workflows, provider breadth, and reliability over time.

See [ROADMAP.md](ROADMAP.md) for the detailed project plan and upcoming work.

## Contributing

Contributions, bug reports, and focused pull requests are welcome.

- [CONTRIBUTING.md](CONTRIBUTING.md)
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [ROADMAP.md](ROADMAP.md)

## License

Melyar is dual-licensed under:

- MIT — [LICENSE-MIT](LICENSE-MIT)
- Apache 2.0 — [LICENSE-APACHE](LICENSE-APACHE)

`SPDX-License-Identifier: MIT OR Apache-2.0`

---

<div align="center">
<sub>Built with Rust, Qt 6, React, and a lot of care for the details.</sub>
</div>
