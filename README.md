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

Packages for Arch and Debian/Ubuntu, plus a binary tarball, are attached to
the [latest release](https://github.com/MAKHMUDuz/melyar/releases/latest).
No build toolchain is needed:

**Debian / Ubuntu** (Ubuntu 24.04 or newer):

```bash
sudo apt install ./melyar_0.0.4-1_amd64.deb
```

**Arch** (dependencies come from the official repos):

```bash
sudo pacman -U melyar-0.0.4-1-x86_64.pkg.tar.zst
```

**Any other distro** — unpack the tarball and copy the files into place:

```bash
tar xf melyar-0.0.4-linux-x86_64.tar.gz
sudo cp bin/melyar /usr/local/bin/
sudo cp -r share/melyar /usr/local/share/
sudo cp share/applications/melyar.desktop /usr/local/share/applications/
sudo cp -r share/icons/hicolor /usr/local/share/icons/
melyar
```

Each binary links against its own distribution's Qt 6 and glibc — the `.deb`
is built on Ubuntu and the `.pkg.tar.zst` on Arch, which is why a package
built for another release may refuse to install. If that happens, build from
source below; it takes about five minutes with the dependencies listed.

You can also install from source once it is built:

```bash
sudo cmake --install build
```

CMake installs to `/usr/local` by default (`bin/melyar`,
`share/melyar/web/`, `share/applications/melyar.desktop`,
`share/icons/.../melyar.svg`). Configure with `-DCMAKE_INSTALL_PREFIX=/usr`
to install into the system directory instead. A `PKGBUILD` (`makepkg -si`)
and a `debian/` recipe (`dpkg-buildpackage -us -uc -b`) are included in the
repository, and CI builds the Debian package on every push.

## Build from source

### Requirements

- Rust ≥ 1.75
- CMake ≥ 3.25
- Qt 6 (`Core`, `Gui`, `Quick`, `Qml`, `WebEngineQuick`, `WebChannel`)
- Node.js ≥ 20
- pnpm ≥ 9
- mpv ≥ 2.x with development files
- `pkg-config`
- `yt-dlp` (optional, used when installed)

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
