# Maintainer: mahmud <mahmudjonmurodov@mail.ru>
pkgname=melyar
pkgrel=1
pkgdesc="Ultra-lightweight native YouTube Music desktop client in C++20 and Rust"
arch=('x86_64')
url="https://github.com/MAKHMUDuz/melyar"
license=('MIT' 'Apache')
# Initial value: pkgver() below replaces it once the checkout exists, but the
# PKGBUILD lint runs before any VCS function and rejects an unset variable.
pkgver=0.0.4
depends=('qt6-base' 'qt6-declarative' 'qt6-webengine' 'qt6-webchannel' 'mpv')
makedepends=('cmake' 'rust' 'cargo' 'pkgconf' 'nodejs' 'pnpm')
optdepends=('yt-dlp: resolves metadata for tracks missing from InnerTube results')
source=("git+https://github.com/MAKHMUDuz/melyar.git")
sha256sums=('SKIP')

pkgver() {
    # `makepkg --printsrcinfo` calls this before the source tree exists, so a
    # failed checkout falls back to the current directory; only a real build
    # runs it from $srcdir/$pkgname.
    cd "$srcdir/$pkgname" 2>/dev/null || true
    # The tags are pushed both with and without the v prefix, and git
    # describe returns whichever it finds, so the prefix is stripped here
    # or makepkg ends up with a version string that sorts before the tag
    # it came from.
    local tag
    tag=$(git describe --tags --abbrev=0 2>/dev/null | sed 's/^v//')
    # The pipeline above always exits 0 (sed does), so an empty result - a
    # directory that is not a checkout - is caught here instead.
    [ -n "$tag" ] || tag=0.0.0
    local commits
    commits=$(git rev-list --count "${tag}"..HEAD 2>/dev/null || echo 0)
    if [ "$commits" -gt 0 ]; then
        printf '%s.r%s.g%s' "$tag" "$commits" "$(git rev-parse --short HEAD)"
    else
        printf '%s' "$tag"
    fi
}

build() {
    cd "$srcdir/$pkgname"

    # Clean previous build artifacts completely
    rm -rf build-release
    rm -rf core_rs/target
    rm -rf ui_web/node_modules

    # Build the web UI (React + TypeScript)
    if command -v pnpm >/dev/null 2>&1; then
        pushd ui_web
        pnpm install --frozen-lockfile
        pnpm build
        popd
    fi

    export PATH="$HOME/.local/bin:$PATH"
    cmake -B build-release -S . \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_INSTALL_PREFIX=/usr
    cmake --build build-release -j$(nproc)
}

package() {
    cd "$srcdir/$pkgname"
    DESTDIR="$pkgdir" cmake --install build-release

    # Install the web UI bundle next to the binary
    install -Dm644 ui_web/dist/index.html "$pkgdir/usr/share/melyar/web/index.html"
    if [ -d ui_web/dist/assets ]; then
        cp -r ui_web/dist/assets "$pkgdir/usr/share/melyar/web/assets"
    fi
    for f in ui_web/dist/favicon.svg ui_web/dist/qwebchannel.js; do
        [ -f "$f" ] && install -Dm644 "$f" "$pkgdir/usr/share/melyar/web/$(basename "$f")"
    done
}