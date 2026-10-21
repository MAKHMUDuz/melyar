# Contributing to Melyar

Melyar is a native YouTube Music client for Linux, built on a Rust core with a
Qt6/QML shell and a React + TypeScript interface. Contributions of every size
are welcome: bug reports, translations, UI polish, provider work, or a full
subsystem.

This document is the contract for how changes land. It is deliberately short.

---

## Table of contents

- [Before you start](#before-you-start)
- [Development setup](#development-setup)
- [Running the app](#running-the-app)
- [Settings](#settings)
- [Translations](#translations)
- [Capturing a screenshot](#capturing-a-screenshot)
- [Checks](#checks)
- [Commit and branch conventions](#commit-and-branch-conventions)
- [Pull request process](#pull-request-process)
- [Code of conduct](#code-of-conduct)
- [Security reports](#security-reports)
- [Where things live](#where-things-live)

---

## Before you start

- **Search existing issues.** A fix that already exists as an open PR costs
  more than it gives.
- **For anything larger than a bug fix, open an issue first.** Architecture
  changes are expensive to review after the code is written. A short issue
  describing the problem and the intended behaviour saves everyone time.
- **Keep the scope focused.** Unrelated rewrites bundled into a feature PR make
  review impossible. Send them as separate PRs.
- **Do not add dependencies without a clear technical reason.** A dependency is
  a permanent maintenance commitment for every future contributor.

---

## Development setup

### Prerequisites

| Tool | Version |
|---|---|
| Rust | ≥ 1.75 |
| CMake | ≥ 3.25 |
| C++ compiler | C++20 support |
| Qt 6 | Core, Gui, Quick, Qml, WebEngineQuick, WebChannel |
| mpv | ≥ 2.x with `libmpv` headers |
| pkg-config | — |
| Node.js | ≥ 20 |
| pnpm | ≥ 9 (10+ required for the `allowBuilds` setting) |
| yt-dlp | on `PATH` |

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

### Build

```bash
git clone https://github.com/MAKHMUDuz/melyar.git
cd melyar

# Web interface
cd ui_web && pnpm install && pnpm build && cd ..

# Desktop application
cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j"$(nproc)"
```

> **Linker note:** CMake emits `-lmpv` before `libmelyar_core.a`. Distributions
> that link with `--as-needed` by default (Ubuntu, Debian) would otherwise drop
> `libmpv` and fail with `undefined reference to mpv_create`. The target
> therefore passes `--no-as-needed`. Keep this in mind when touching
> `CMakeLists.txt`.

---

## Running the app

```bash
./build/melyar
```

For web interface work, run Vite alongside the app:

```bash
cd ui_web && pnpm dev              # terminal 1
cd ../ && MELYAR_DEV_URL=http://localhost:5173 ./build/melyar   # terminal 2
```

The Rust core logs to stderr. Set `RUST_LOG=debug` for verbose output.

### Environment variables

| Variable | Effect |
|---|---|
| `MELYAR_DEV_URL` | Load the interface from a live dev server instead of `ui_web/dist/`. |
| `MELYAR_LANG` | Force the interface language (`en`, `tr` or `sq`), overriding browser detection. |
| `MELYAR_SCREENSHOT` | Write a PNG of the window to this path, then exit. |
| `MELYAR_SCREENSHOT_DELAY` | How long to wait before the capture, in milliseconds. Default 6000. |
| `MELYAR_EVAL_JS` | Run a script inside the interface once it has booted. A debugging aid. |
| `RUST_LOG` | Rust log level, for example `debug`. |

### Deep links

The desktop entry registers the `melyar://` handler, and the application acts
on these links:

| Link | Effect |
|---|---|
| `melyar://settings` | Opens the settings panel. |
| `melyar://search/<query>` | Runs a search. |
| `melyar://track/<id>` | Plays a track by its YouTube video id. |

```bash
melyar://settings
```

---

## Settings

The settings live in the core's SQLite database, in a `settings` table, so
they survive a restart.

- **Rust:** `core_rs/src/db/storage.rs` holds the table and the accessors.
- **Bridge:** `loadSettings`, `saveSetting` and `removeSetting` in
  `ui_cpp/bridge/bridge.cpp` move them between the core and the interface.
- **Interface:** `ui_web/src/lib/settings.ts` holds the store and applies the
  settings to the document.

Adding a setting means touching all three, plus a default in
`DEFAULT_SETTINGS` and a type rule in `coerce`.

> QWebChannel calls are asynchronous. A method that returns a value gives you
> a promise, not the value itself - `await` it. This is the single most
> common mistake when adding a bridge method.

---

## Translations

The interface ships in twelve languages: English, Turkish, Spanish, German,
French, Italian, Portuguese, Russian, Ukrainian, Japanese, Chinese and
Korean. Adding one is a translation job, not a programming job.

1. Copy `ui_web/src/locales/en.ts` to `ui_web/src/locales/<code>.ts`, where
   `<code>` is the two letter code (`es`, `de`, `fr`, `it`, `pt`, `ru`, `uk`,
   `ja`, `zh`, `ko`).
2. Translate the **values**. Keep the **keys** exactly as they are.
3. Register the language in `ui_web/src/lib/i18n.ts`:
   - import the table
   - add it to `MESSAGES`
   - add it to `LOCALES` with the language name in its own language
   - widen the `Locale` type
4. Run the checks:
   ```bash
   cd ui_web
   pnpm exec tsc --noEmit
   pnpm build
   ```
5. Try it:
   ```bash
   MELYAR_LANG=<code> ./build/melyar
   ```

**Rules**

- **Do not translate the keys.** They are identifiers, not text.
- **Keep the `{placeholders}`** exactly as they are. `"{n} tracks · {time}"`
  becomes `"{n} pistas · {time}"` in Spanish, not `"{n} canciones"` - the count
  and the duration are inserted at runtime.
- Keep it short. These are interface labels, not sentences.
- Translate the `aria-label` values too. Screen readers read them.
- Every locale file must have the same keys as `en.ts`. A missing key shows
  the key itself in the interface, so it is easy to spot.

**Right-to-left languages** need a little more: the layout still uses a few
physical margins, so they have to be converted to logical properties first.
Open an issue before starting one, so the work can be agreed on.

---

## Capturing a screenshot

The issue template asks for a visual when a problem is visual. You do not need
a screenshot tool for this:

```bash
MELYAR_SCREENSHOT=/tmp/melyar.png MELYAR_SCREENSHOT_DELAY=8000 ./build/melyar
```

The application renders, scrolls the interface back to the top, writes the PNG
and exits. Combine it with `MELYAR_LANG` to capture a specific language:

```bash
MELYAR_LANG=tr MELYAR_SCREENSHOT=/tmp/melyar-tr.png ./build/melyar
```

---

## Checks

Run these before opening a pull request. CI runs the same commands, but a
green local run means a faster review.

```bash
# Rust core
cargo fmt --all
cargo clippy --manifest-path core_rs/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path core_rs/Cargo.toml

# Web interface
cd ui_web
pnpm exec tsc --noEmit
pnpm build
cd ..

# Full application
cmake --build build -j"$(nproc)"
```

`ui_web/dist/` is committed so that a plain `cmake` build produces a runnable
binary. **If you change anything under `ui_web/src/`, rebuild the bundle and
commit the regenerated `dist/`** with your changes.

---

## Commit and branch conventions

**Branch names** are prefixed by kind:

```
feat/short-description
fix/short-description
refactor/short-description
chore/short-description
docs/short-description
ci/short-description
```

**Commit messages** follow the Conventional Commits style:

```
type(scope): concise description
```

- `type` is one of `feat`, `fix`, `docs`, `refactor`, `chore`, `ci`, `build`,
  `revert`, `perf`, `test`
- `scope` is the component: `core`, `ui`, `bridge`, `innertube`, `audio`,
  `db`, `mpris`, `waybar`, `ci`
- The first line stays under 72 characters and is written in the imperative mood

Examples:

```
feat(innertube): add playlist continuation support
fix(audio): stop prefetching when the queue is empty
docs(ui): document the glass panel spacing scale
```

Explain **why** in the body, not what - the diff already shows what changed.

---

## Pull request process

### Working from a fork

External contributors usually work from a fork rather than a branch in this
repository.

```bash
# 1. Fork the repository on GitHub, then clone your fork
git clone https://github.com/<your-username>/melyar.git
cd melyar

# 2. Add the upstream repository as a second remote
git remote add upstream https://github.com/MAKHMUDuz/melyar.git

# 3. Create a branch
git checkout -b feat/short-description

# 4. ... make your changes ...

# 5. Rebase onto the latest upstream before pushing
git fetch upstream
git rebase upstream/master

# 6. Push to your fork
git push -u origin feat/short-description
```

Then open the pull request against `MAKHMUDuz/melyar` from your fork. GitHub
will offer a "Compare & pull request" link on the branch page.

If you have push access to this repository, work on a branch here directly
instead - the rebase step is the same.

### The process

1. **Branch** off `master` with a prefixed name.
2. **Rebase** onto the latest base branch before opening the PR:
   ```bash
   git fetch origin && git rebase origin/master
   ```
3. **Open the PR** using the template. Describe the problem, the intended
   behaviour, and how you verified it.
4. **Attach screenshots or a short recording** for any UI change. For playback
   or lyrics fixes, include reproducible steps and logs with secrets removed.
5. **Keep the checks green.** A failing CI run blocks review.

### Review expectations

- **Preserve working playback and bridge behaviour.** The C++/Rust bridge and
  the mpv engine are load-bearing; changes there need extra care and a manual
  playback test.
- **Explain UI changes visually.** Screenshots beat descriptions.
- **Do not include build output, credentials, personal data, or machine-specific
  paths** in a commit.

---

## Code of conduct

Participation is governed by the [Code of Conduct](.github/CODE_OF_CONDUCT.md).
By taking part you agree to uphold it.

---

## Security reports

Do **not** open a public issue for a security problem. Follow
[`.github/SECURITY.md`](.github/SECURITY.md) instead.

---

## Where things live

| Path | Contents |
|---|---|
| `core_rs/src/innertube/` | InnerTube client, stream cipher, lyrics fetching |
| `core_rs/src/audio/` | mpv engine wrapper and the playback queue |
| `core_rs/src/db/` | SQLite storage (favorites, history, lyrics cache) |
| `core_rs/src/linux/` | MPRIS (D-Bus) and the Waybar socket IPC server |
| `core_rs/src/lib.rs` | The cxx bridge and the public core API |
| `ui_cpp/main.cpp` | Qt application bootstrap |
| `ui_cpp/bridge/` | C++ side of the bridge exposed to QML |
| `ui_cpp/qml/` | QML components, including the Liquid Glass set |
| `ui_web/src/` | React + TypeScript interface |
| `ui_web/src/lib/` | Shared types, the QWebChannel client, motion tokens |
| `docs/ARCHITECTURE.md` | How the pieces fit together |

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for a map of the system and
the boundaries between the three languages.
