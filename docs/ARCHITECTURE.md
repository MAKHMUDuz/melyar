# Melyar Architecture

This document maps the system for people new to the codebase. It describes
where things live, how data flows between the three languages, and which
boundaries are load-bearing.

---

## The three languages

Melyar is one application written in three languages. Each has a distinct job.

```
┌──────────────────────────────────────────────────────────────────────┐
│  ui_web/           React 19 + TypeScript + Tailwind 4                │
│                    Rendering, state, the Liquid Glass interface       │
│                     ↕  QWebChannel (JSON over a local socket)         │
├──────────────────────────────────────────────────────────────────────┤
│  ui_cpp/           Qt 6 / QML shell                                   │
│                    Window, native menus, global shortcuts, the        │
│                    QWebEngineView that hosts the React app            │
│                     ↕  cxx bridge (C++ header ↔ Rust)                 │
├──────────────────────────────────────────────────────────────────────┤
│  core_rs/          Rust core                                          │
│                    InnerTube client, mpv audio, SQLite, MPRIS,        │
│                    Waybar IPC                                          │
└──────────────────────────────────────────────────────────────────────┘
```

**Why three languages?** Rust owns everything that must be correct and
concurrent (network, audio timing, disk, IPC). QML owns the native window
chrome and desktop integration. TypeScript owns the interface, where
iteration speed matters most and where the Liquid Glass design system lives.

---

## Layer 1: `core_rs` (Rust)

The core is the only layer with no UI dependency. It can be tested, run, and
profiled on its own.

| Module | Responsibility |
|---|---|
| `innertube/client.rs` | InnerTube HTTP client: search, library queries, stream URL resolution. Also resolves metadata through `yt-dlp` when InnerTube does not return enough. |
| `innertube/cipher.rs` | Solves the InnerTube stream cipher (the `n`-parameter transform) by extracting and interpreting the base.js actions. |
| `innertube/parser.rs` | Converts raw InnerTube JSON into the track, album, artist and playlist models. |
| `innertube/lyrics.rs` | Fetches lyrics from `lrclib`, returning synchronized LRC when available and plain text otherwise. |
| `import.rs` | Reads another service's library export and matches every entry against InnerTube. |
| `audio/engine.rs` | Wraps `libmpv`: playback, pause, seek, volume, property observation, gapless prefetch. |
| `audio/queue.rs` | The playback queue, with shuffle and repeat handling. |
| `db/storage.rs` | SQLite persistence: favorites, listening history, search history, lyrics cache. |
| `linux/mpris.rs` | The `org.mpris.MediaPlayer2.melyar` D-Bus service, so GNOME, KDE, i3 and sway can control playback. |
| `linux/waybar.rs` | A Unix domain socket server at `$XDG_RUNTIME_DIR/melyar.sock` for status bars. |

### The cxx bridge

`core_rs/src/lib.rs` defines the bridge. The pattern is always the same:

- **Rust exposes `extern "Rust"` functions** - `search`, `play_track`,
  `toggle_play`, `seek`, `set_volume`, `next_track`, `fetch_lyrics_live`,
  `toggle_favorite`, `get_library_albums`, and so on.
- **Rust calls back into C++** through `extern "C++"`:
  `on_progress_update`, `on_track_changed`, `on_state_changed`,
  `on_lyrics_loaded`, `on_search_results`.

Data crosses the boundary as plain structs (`CxxTrackMetadata`,
`CxxLibraryAlbum`, `CxxLibraryArtist`, `CxxLibraryPlaylist`). Keep it that
way. Passing complex types across the bridge makes the contract hard to see
and the generated headers harder to maintain.

`core_rs/build.rs` compiles the bridge and copies the generated headers into
`core_rs/include/melyar_core/`. Those generated files are **not** committed -
they are rebuilt on every compile.

### Concurrency model

A single `tokio` runtime lives in `MelyarCore`. Blocking work (notably
`yt-dlp` calls) is pushed to `spawn_blocking` so the runtime is never stalled.
Playback events arrive on the mpv event thread and are forwarded to the UI
through the bridge.

### Audio engine

`libmpv` is driven in-process:

- `mpv_observe_property` watches `time-pos`, `duration`, `pause` and
  `eof-reached`.
- When a track ends, the core resolves the next stream URL and starts it -
  gapless prefetch is triggered slightly before the end.
- The bridge receives `on_track_changed`, which is the signal the UI uses to
  refresh the player.

---

## Layer 2: `ui_cpp` (C++ / QML)

| File | Responsibility |
|---|---|
| `main.cpp` | Application bootstrap. Sets up the icon, the QML engine, the `MelyarBridge` instance, and resolves the web UI location. |
| `bridge/bridge.cpp`, `bridge.hpp` | The C++ half of the cxx bridge. Exposes the Rust API to QML as invokable methods and forwards Rust callbacks to the QML side. |
| `qml/Main.qml` | The root window. Hosts the `WebEngineView` and owns the `WebChannel` handshake. |
| `qml/theme/Style.qml` | The design tokens, exposed as a QML singleton. |
| `qml/components/` | The Liquid Glass component set - `GlassCard`, `GlassSidebar`, `PlayerBar`, `LiquidLyricsView`, and the rest. |

### The WebChannel handshake

The order matters, and it is deliberate:

1. QML creates the `WebChannel` and the `WebEngineView` **without** a URL.
2. `main.cpp` registers the bridge object onto the channel.
3. Only then is the view URL set.

If the URL were set first, the page could initialise its QWebChannel client
before the bridge object existed, and the first calls would silently fail.

### Global shortcuts


`ui_cpp/qml/Main.qml` dosyasında `StandardKey` ile window-scoped (pencere kapsamlı) Shortcut tanımlanmıştır. X11/Wayland global grab yoktur; sadece pencere odaklı kısayollardir.

for the application shortcuts. The bindings are configurable from the settings
view, and `ShortcutEditor.qml` / `ShortcutRow.qml` render them.

---

## Layer 3: `ui_web` (React + TypeScript)

| Path | Responsibility |
|---|---|
| `src/main.tsx` | Entry point. |
| `src/App.tsx` | The application shell and routing between views. |
| `src/store.ts` | The Zustand store: playback state, queue, library, lyrics. |
| `src/lib/client.ts` | The QWebChannel client. Every call into Rust goes through here. |
| `src/lib/i18n.ts` | The translation layer: the message tables, `t()`, the locale detection and the `useLocale()` hook. |
| `src/lib/settings.ts` | The settings store, the defaults, and the mapping from stored values to the document. |
| `src/locales/` | One file per language, each with the same keys as `en.ts`. |
| `src/lib/types.ts` | The TypeScript mirrors of the bridge structs. |
| `src/lib/registry.ts` | The action/command registry used by the context menu and keyboard handling. |
| `src/lib/motion.ts` | Central motion tokens. One place defines durations and easings, including the reduced-motion path. |
| `src/lib/useLyricsScroll.ts` | The synchronized lyrics scroll behaviour. |
| `src/components/` | The interface: `Sidebar`, `Toolbar`, `PlayerBar`, `QueuePanel`, `ExpandedPlayer`, `LyricsOverlay`, the glass primitives, and the views. |

### State flow

```
Rust event  ->  bridge callback  ->  QML  ->  QWebChannel  ->  store  ->  React
Rust command <-  client.ts        <-  QML  <-  QWebChannel  <-  store  <-  React
```

React never talks to Rust directly. It goes through `client.ts`, which is the
single place where the bridge surface is mirrored. When the bridge API changes,
that file is the only one in the web layer that needs to change with it.

### Building

`pnpm build` produces `ui_web/dist/`, which is **committed** so that a plain
`cmake` build yields a runnable binary without requiring Node.js. If you touch
anything in `src/`, rebuild and commit `dist/` alongside your change.

---

## Data flow: playing a track

1. The user clicks a track in React.
2. React calls `play_track(id)` through `client.ts`.
3. The bridge reaches `core_rs/src/lib.rs::play_track`.
4. The core resolves the metadata (queue lookup, falling back to `yt-dlp`),
   then asks InnerTube for the stream URL.
5. The audio engine hands the URL to `libmpv`.
6. `libmpv` starts playing; the engine observes `time-pos`.
7. The core calls `on_track_changed` and `on_progress_update`.
8. The bridge forwards to QML, which pushes to the web layer over WebChannel.
9. The store updates; the player bar, the expanded player and the lyrics view
   re-render.

---

## Build system

`CMakeLists.txt` drives everything.

- **Corrosion** bridges Rust into CMake. It is fetched automatically if it is
  not already installed.
- `corrosion_import_crate` imports `core_rs` as the static library
  `melyar_core`.
- Qt resources are compiled in via `qt6_add_resources`, covering both the QML
  tree and `resources/`.
- The web bundle is installed to `share/melyar/web`.

### The `--as-needed` caveat

CMake emits `-lmpv` **before** `libmelyar_core.a`, because the Rust core
declares libmpv as an interface dependency. Distributions that link with
`--as-needed` by default (Ubuntu, Debian) then drop `libmpv` from the link
line, because no `mpv_*` symbol is undefined at that point in the command.
The link then fails with `undefined reference to mpv_create`.

The target therefore passes `--no-as-needed`. If you restructure the link
libraries, keep that flag or verify the build on Ubuntu.

---

## Design system

The Liquid Glass look is implemented in three places that must stay in sync:

- `ui_web/src/glass.css` and the components in `ui_web/src/components/GlassComponents.tsx`
- `ui_cpp/qml/components/GlassCard.qml` and `GlassSidebar.qml`
- `ui_web/src/lib/motion.ts` and `ui_cpp/qml/theme/Style.qml`

Motion durations and easings live in `motion.ts` for the web layer and
`Style.qml` for the QML layer. Both honour a reduced-motion preference. When
you add an animation, add it to the token system rather than hard-coding a
duration, so that the reduced-motion path keeps working.

---

## Testing

- **Rust:** `cargo test --manifest-path core_rs/Cargo.toml`. The existing suite
  covers storage, the cipher solver, the lyrics fetcher and the Waybar IPC
  server.
- **Web:** `ui_web/src/lib/lyrics.test.ts` exists; the runner is not yet wired
  into CI. Adding `vitest` and running it in the `web-ui` job is a welcome
  contribution.
- **Manual:** playback, seeking, lyrics sync, queue behaviour, panel
  open/close, and window resizing are the areas that most need a human check.
