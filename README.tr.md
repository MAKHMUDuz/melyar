<div align="center">

# Melyar

**YouTube Music için ultra hafif yerel masaüstü istemcisi**

[![CI](https://github.com/MAKHMUDuz/melyar/actions/workflows/ci.yml/badge.svg)](https://github.com/MAKHMUDuz/melyar/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/MAKHMUDuz/melyar.svg)](https://github.com/MAKHMUDuz/melyar/releases/latest)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-APACHE)
[![Discussions](https://img.shields.io/github/discussions/MAKHMUDuz/melyar?label=discussions&logo=github)](https://github.com/MAKHMUDuz/melyar/discussions)

</div>

Melyar, YouTube Music için Linux öncelikli, yerel öncelikli bir masaüstü müzik
istemcisidir. Hızlı bir Rust çekirdeği, yerel Qt/C++ kabuğu ve şık bir React +
TypeScript arayüzünü bir araya getirir; ağır masaüstü çatılarının yükü olmadan
temiz, duyarlı bir dinleme deneyimi sunar.

![Melyar ana sayfa](docs/screenshots/home-en.png)

## Neden Melyar

Melyar, Linux'ta yerel hissettiren bir müzik oynatıcı isteyenler için:

- Küçük çalışma zamanı ayak izi
- Hızlı başlangıç ve duyarlı kontroller
- MPRIS ve Waybar ile derin masaüstü entegrasyonu
- Uygulamayı başlatmak için hesap gerekmez
- Kişisel kütüphane verisi diskte tutulur, yerel öncelikli davranış

## Öne çıkanlar

- YouTube Music içeriğinde arama, gezinme ve çalma
- `libmpv` ile çalma ve gapless akış davranışı
- Kuyruk yönetimi, karıştırma, tekrarlama, ses ve seek kontrolleri
- Senkronize LRC şarkı sözleri ve düz metin yedeği
- Beğeniler, geçmiş, albümler, sanatçılar ve kütüphane görünümleri
- Spotify dışa aktarımı, JSON veya CSV'den kütüphane içe aktarma
- On bantlı grafik ekolayzer (31 Hz - 16 kHz)
- Liquid Glass ilhamlı arayüz, açık/koyu tema
- Anlık değiştirilebilen on üç arayüz dili
- MPRIS ve Unix soketi IPC ile yerel Linux medya entegrasyonu

## Ekran görüntüleri

![Melyar ana sayfa (Türkçe)](docs/screenshots/home-tr.png)

## Mimari

```text
React + TypeScript UI
        │
        │ QWebChannel
        ▼
Qt / C++ köprü
        │
        ▼
Rust çekirdek ── çalma, kütüphane, sağlayıcı mantığı, IPC
        │
        ▼
mpv ses oynatma
```

C++ ↔ Rust sınırı `cxx` köprüsüyle, arayüz ile çekirdek arasındaki iletişim
Qt/WebChannel üzerinden kurulur.

## Kurulum

Arch ve Debian/Ubuntu paketleri ile binary tarball,
[sürüm sayfasına](https://github.com/MAKHMUDuz/melyar/releases/latest)
eklidir. Derleme aracı gerekmez:

**Debian / Ubuntu** (Ubuntu 24.04 veya yenisi):

```bash
sudo apt install ./melyar_0.0.4-1_amd64.deb
```

**Arch** (bağımlılıklar resmi depolardan gelir):

```bash
sudo pacman -U melyar-0.0.4-1-x86_64.pkg.tar.zst
```

**Diğer distrolar** — tarball'ı açıp dosyaları elle yerleştirin:

```bash
tar xf melyar-0.0.4-linux-x86_64.tar.gz
sudo cp bin/melyar /usr/local/bin/
sudo cp -r share/melyar /usr/local/share/
sudo cp share/applications/melyar.desktop /usr/local/share/applications/
sudo cp -r share/icons/hicolor /usr/local/share/icons/
melyar
```

Her binary kendi dağıtımının Qt 6 ve glibc sürümüne bağlıdır — `.deb`
Ubuntu'da, `.pkg.tar.zst` Arch'ta derlenir; başka bir sürüm için üretilmiş
paket kurulmayı reddedebilir. Öyle olursa aşağıdaki gibi kaynaktan derleyin;
bağımlılıklarla yaklaşık beş dakika sürer.

Kaynaktan derledikten sonra şöyle de kurabilirsiniz:

```bash
sudo cmake --install build
```

CMake varsayılan olarak `/usr/local`'a kurar (`bin/melyar`,
`share/melyar/web/`, `share/applications/melyar.desktop`,
`share/icons/.../melyar.svg`). Sistem dizinine kurmak için derlemeden önce
`-DCMAKE_INSTALL_PREFIX=/usr` verin. Depoda bir `PKGBUILD`
(`makepkg -si`) ve bir `debian/` tarifi (`dpkg-buildpackage -us -uc -b`)
vardır; CI Debian paketini her push'da derler.

## Kaynaktan derleme

### Gereksinimler

- Rust ≥ 1.75
- CMake ≥ 3.25
- Qt 6 (`Core`, `Gui`, `Quick`, `Qml`, `WebEngineQuick`, `WebChannel`)
- Node.js ≥ 20
- pnpm ≥ 9
- mpv ≥ 2.x (development dosyalarıyla)
- `pkg-config`
- `yt-dlp` (isteğe bağlı, kuruluysa kullanılır)

<details>
<summary>Dağıtım örnekleri (Arch / Manjaro, Debian / Ubuntu)</summary>

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

### Derle ve çalıştır

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

## Masaüstü entegrasyonu

### MPRIS

Melyar, oturum veri yolunda `org.mpris.MediaPlayer2.melyar` olarak kendini
kaydeder; Linux masaüstleri çalma durumunu elle yapılandırma olmadan keşfeder.

### Waybar / durum çubuğu

Melyar, Waybar, Polybar ve i3status gibi araçlar için
`$XDG_RUNTIME_DIR/melyar.sock` adresinde sıfır gecikmeli bir Unix soketi
sunar.

Detaylı kurulum, CSS ve Hyprland/Sway kısayol örnekleri:
[`docs/WAYBAR.md`](docs/WAYBAR.md).

## Veri konumu

| Veri | Yol |
|---|---|
| Veritabanı | `~/.local/share/melyar/melyar.db` |
| IPC soketi | `$XDG_RUNTIME_DIR/melyar.sock` |

## Yol haritası

Melyar aktif olarak geliştiriliyor. Proje Linux masaüstü entegrasyonu,
kütüphane akışları, sağlayıcı çeşitliliği ve güvenilirliği zamanla
iyileştirmeyi hedefliyor.

Ayrıntılı plan için [ROADMAP.md](ROADMAP.md).

## Katkı

Katkılara, hata raporlarına ve odaklı pull request'lere açığız.

- [CONTRIBUTING.md](CONTRIBUTING.md)
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)
- [ROADMAP.md](ROADMAP.md)

## Lisans

Melyar çift lisanslıdır:

- MIT — [LICENSE-MIT](LICENSE-MIT)
- Apache 2.0 — [LICENSE-APACHE](LICENSE-APACHE)

`SPDX-License-Identifier: MIT OR Apache-2.0`

---

<div align="center">
<sub>Rust, Qt 6, React ve detaylara gösterilen özenle üretildi.</sub>
</div>
