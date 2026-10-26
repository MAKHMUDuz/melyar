<div align="center">

# Melyar

**Liquid Glass ilhamlı, yerel öncelikli bir masaüstü müzik oynatıcısı.**

English · [Türkçe](README.tr.md)

[![CI](https://github.com/MAKHMUDuz/melyar/actions/workflows/ci.yml/badge.svg)](https://github.com/MAKHMUDuz/melyar/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/MAKHMUDuz/melyar.svg)](https://github.com/MAKHMUDuz/melyar/releases/latest)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-APACHE)
[![Contributors](https://img.shields.io/github/contributors/MAKHMUDuz/melyar.svg)](https://github.com/MAKHMUDuz/melyar/graphs/contributors)
[![Discussions](https://img.shields.io/github/discussions/MAKHMUDuz/melyar?label=discussions&logo=github)](https://github.com/MAKHMUDuz/melyar/discussions)

</div>

---

Melyar; müzik keşfi, çalma, şarkı sözleri, kuyruk ve kişisel dinleme
kütüphanesini tek bir masaüstü uygulamasında toplar: bir **Rust çekirdeği**,
bir **Qt/C++ köprüsü** ve bir **React + TypeScript arayüzü**. Çalma `libmpv`
ile, akış/üzeri veri çözümleme ise YouTube `InnerTube` API'si üzerinden
yürür. İstemci sürümü çalışma anında canlı web istemciden okunur ve birden
fazla istemci kimliği sırayla denenir; biri reddedilse bile çalma durmaz.
`yt-dlp` kuruluysa kullanılır, ama zorunlu değildir.

> **Proje durumu: geliştirme aşamasında.** Bazı arayüz etkileşimleri ve
> sağlayıcı entegrasyonları hâlâ olgunlaşıyor; değişiklikler ve ara sıra
> hatalar beklenin.
> **[Katkıda bulunmak isterseniz: katkı rehberi](CONTRIBUTING.md)**

---

## Neden Melyar

Başka YouTube Music istemcileri var. Melyar'ın amacı en taşınabilir istemci
olmak değil, **Linux'ta en iyi** istemci olmak.

| | Melyar | Spotube | BloomeeTunes |
|---|---|---|---|
| Yığın | **Rust + Qt6/QML + React** | Flutter/Dart | Flutter/Dart |
| Platformlar | Linux | Win, macOS, mobil, web | Win, macOS, mobil, web |
| Çalışma zamanı | Küçük - Flutter motoru/Electron ağacı yok | Büyük | Büyük |
| MPRIS (GNOME, KDE, i3) | Tam | Kısmi | Kısmi |
| Waybar / Polybar IPC | Yerleşik | Yok | Yok |
| Lisans | **MIT OR Apache-2.0** | BSD-4 | GPL-2.0 |

Üç iddia: **yerel olan kazanır** (Rust çekirdek + Qt kabuk hızlı başlar, az
bellek kullanır), **masaüstü entegrasyonu ürünün kendisidir** (MPRIS, Waybar
soketi, medya kısayolları, gerçek bir `.desktop` girdisi) ve **esnek lisans**
(distribütörler gömüp dağıtabilir). Rakiplerin hâlâ güçlü olduğu alanlar -
çok platformluluk ve sağlayıcı çeşitliliği - [roadmap](ROADMAP.md) içinde
açıkça yazılı.

---

## Öne çıkanlar

- **On bantlı grafik ekolayzer**, 31 Hz - 16 kHz, mpv filtre zinciri üzerine -
  diğer istemcilerin hâlâ sahip olmadığı tek özellik.
- **Daha hafif bir uygulama:** grafik yığını kapalı, çünkü müzik oynatıcı
  video çizmez. Tek başına bu 59 MB daha az bellek; arayüz yaklaşık 650 ms'de
  hazır.
- **Denetlenen bir arayüz.** Her push, uygulamayı her görünüm ve her
  katman üzerinden yürütür; bir şey tersse derlemeyi düşürür.
- **Çalma:** mpv üzerinden, akış çözümlemesi YouTube Music API'si ile. Birden
  fazla istemci kimliği sırayla denenir; YouTube bir şey değiştirse bile
  çalışmaya devam eder.
- **Senkronize LRC şarkı sözleri** (zaman damgalı vurgu + düz metin yedeği) ve
  gapless ön yükleme ile kuyruk yönetimi.
- **Kütüphane içe aktarma** - Spotify hesap dışa aktarımı, JSON listesi veya
  CSV dosyası getirin; Melyar her şarkıyı YouTube Music ile eşleştirip
  tanıyabildiklerini ekler.
- **Kişisel kütüphane** görünümleri: beğenilenler, geçmiş, şarkılar, albümler,
  sanatçılar - yerel veritabanında.
- **On üç dil** - İngilizce, Türkçe, İspanyolca, Almanca, Fransızca, İtalyanca,
  Portekizce, Rusça, Ukraynaca, Japonca, Çince, Korece ve Arnavutça - ve
  yeniden başlatmada korunan ayarlar.
- **Masaüstü entegrasyonu:** Linux MPRIS (D-Bus) ve Waybar / Polybar /
  i3status için sıfır gecikmeli Unix soketi IPC.
- **Yerel öncelikli:** başlatmak için Melyar hesabı veya Melyar bulut servisi
  gerekmez.

Kullanılabilirlik sağlayıcıya, parçaya, işletim sistemine ve dış servis
davranışına göre değişebilir. Henüz tamamlanmamış entegrasyonlar için
[roadmap](ROADMAP.md).

---

## Ekran görüntüleri

![Melyar ana sayfa](docs/screenshots/home-en.png)

![Melyar ana sayfa (Türkçe)](docs/screenshots/home-tr.png)

> Ekran görüntüleri örnek bir kütüphaneye karşı alındı. Kendi görüntünüzü
> almak için: `MELYAR_SCREENSHOT=/path/to.png ./build/melyar` -
> bkz. [katkı rehberi](CONTRIBUTING.md#capturing-a-screenshot).

---

## Mimari

```
React + TypeScript UI ──QWebChannel──▶ Qt / C++ köprü ──▶ Rust çekirdek ──▶ mpv
                                                              │
                                          sağlayıcı ve üzeri veri mantığı
```

C++ ↔ Rust sınırı [`cxx`](https://cxx.rs) köprüsüdür. Rust tarafı `tokio`
runtime'ı üzerinde çalışır; ses `libmpv` ile senkron sürülür.

| Dizin | Amaç |
|---|---|
| `core_rs/` | Rust çekirdek: çalma koordinasyonu, parça modelleri, şarkı sözleri, sağlayıcı mantığı, yerel uygulama verisi. |
| `ui_cpp/` | Qt/C++ entegrasyonu ve web arayüzü ile çekirdek arasındaki köprü. |
| `ui_web/` | React + TypeScript arayüzü, görünümler, bileşenler, Liquid Glass stilleri. |

---

## Gereksinimler

| Bileşen | Sürüm |
|---|---|
| Rust toolchain ve Cargo | ≥ 1.75 |
| CMake | ≥ 3.25 |
| C/C++ derleme aracı | C++20 desteği |
| Qt 6 | Core, Gui, Quick, Qml, WebEngineQuick, WebChannel |
| Node.js / pnpm | ≥ 20 / ≥ 9 |
| mpv | ≥ 2.x (`libmpv` geliştirme dosyaları) |
| pkg-config | — |
| yt-dlp | isteğe bağlı, kuruluysa kullanılır |

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

`yt-dlp` isteğe bağlıdır: akışlar onun olmadan da çözülür, ama protokolün
zorlu kısımlarının bir kısmını o üstlenir. Bir akış çözülmüyorsa `yt-dlp`'nin
kurulu, çalıştırılabilir ve Melyar'ın başlatıldığı ortamda erişilebilir
olduğunu doğrulayın.

---

## Kaynaktan derleme

```bash
git clone https://github.com/MAKHMUDuz/melyar.git
cd melyar

# Derlenmiş web paketi ui_web/dist/ altında commit'li gelir; bu adım yalnızca
# arayüz kaynakları değiştiğinde gerekir.
cd ui_web && pnpm install && pnpm build && cd ..

cmake -S . -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build -j"$(nproc)"
./build/melyar
```

Canlı geliştirmek için Vite dev sunucusunu ayrı bir terminalde başlatıp
uygulamayı `MELYAR_DEV_URL=http://localhost:5173 ./build/melyar` ile
çalıştırın.

---

## Kurulum

### Hazır paketler

Arch ve Debian/Ubuntu paketleri ile binary tarball,
[releases](https://github.com/MAKHMUDuz/melyar/releases/latest) sayfasında.
Derleme aracı gerekmez.

**Debian / Ubuntu** (Ubuntu 24.04 veya yenisi):

```bash
sudo apt install ./melyar_0.0.4-1_amd64.deb
```

**Arch** (bağımlılıklar resmi depolardan gelir):

```bash
sudo pacman -U melyar-0.0.4-1-x86_64.pkg.tar.zst
```

**Herhangi bir distro** - tarball'ı açıp dosyaları elle kurun:

```bash
tar xf melyar-0.0.4-linux-x86_64.tar.gz
sudo cp bin/melyar /usr/local/bin/
sudo cp -r share/melyar /usr/local/share/
sudo cp share/applications/melyar.desktop /usr/local/share/applications/
sudo cp -r share/icons/hicolor /usr/local/share/icons/
melyar
```

Binary, dağıtımınızın Qt 6 ve glibc sürümüne bağlıdır; bu yüzden `.deb`
Ubuntu'da, `.pkg.tar.zst` Arch'ta üretilir. Paket glibc sürümü yüzünden
kurulmuyorsa kaynaktan derleyin - yukarıdaki bağımlılıklarla beş dakikalık
bir derlemedir.

### Kaynaktan

```bash
sudo cmake --install build
```

CMake'in varsayılan prefix'i `/usr/local`'dır (`bin/melyar`,
`share/melyar/web/`, `share/applications/melyar.desktop`,
`share/icons/.../melyar.svg`). Sistem dizinine kurmak için derleme öncesi
`-DCMAKE_INSTALL_PREFIX=/usr` verin.

Uygulama web arayüzünü şu sırayla arar: çalışma dizini → binary'nin yanı →
`/usr/share/melyar/web/` → `/usr/local/share/melyar/web/`. Canlı Vite sunucusu
için `MELYAR_DEV_URL` kullanın.

Depoda bir `PKGBUILD` (`makepkg -si`) ve bir `debian/` tarifi
(`dpkg-buildpackage -us -uc -b`) var; sürekli entegrasyon Debian paketini her
push'da derliyor.

---

## Masaüstü entegrasyonu

**MPRIS.** Melyar, oturum veri yolunda `org.mpris.MediaPlayer2.melyar`
adıyla yanıt verir; tepsi uygulaması, medya kısayolları, kilit ekranı bileşeni
ve ses OSD'si onu sürer, başlık, sanatçı, kapak, konum ve çalma durumu KDE
Plasma ile GNOME'da yapılandırma olmadan görünür. İki yönlü çalışır: oynat,
duraklat, sonraki, önceki, seek ve masaüstünün ses kaydırıcısı Melyar'ı
hareket ettirir; tekrarlama ve karıştırma `LoopStatus`/`Shuffle` olarak
bildirilir. Servis uygulamanın parçasıdır - ek bağımlılık yok, açacak ayar yok.

```bash
busctl --user get-property org.mpris.MediaPlayer2.melyar \
    /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player Metadata
```

**Waybar / durum çubukları.** D-Bus konuşmayan çubuklar için Melyar
`$XDG_RUNTIME_DIR/melyar.sock` adresinde sıfır gecikmeli Unix soketi IPC
sunucusu çalıştırır (komutlar: `toggle`, `next`, `prev`, `stop`):

```jsonc
"custom/melyar": {
    "format": "{icon} {}",
    "return-type": "json",
    "exec": "socat -u UNIX-CONNECT:${XDG_RUNTIME_DIR:-/tmp}/melyar.sock -",
    "on-click": "echo toggle | nc -U ${XDG_RUNTIME_DIR:-/tmp}/melyar.sock",
    "interval": 1
}
```

Detaylı kurulum, CSS ve Hyprland/Sway kısayol örnekleri:
[`docs/WAYBAR.md`](docs/WAYBAR.md).

---

## Veri konumu

| Veri | Yol |
|---|---|
| Veritabanı (favoriler, geçmiş, söz önbelleği) | `~/.local/share/melyar/melyar.db` |
| IPC soketi | `$XDG_RUNTIME_DIR/melyar.sock` |

---

## Yol haritası

Tam plan - Melyar'ın rakiplerin gerisinde kaldığı yerler dahil -**
[ROADMAP.md](ROADMAP.md)** içinde: sağlam bir Linux vatandaşı (ayarlar,
erişilebilirlik, çökme güvenli yazımlar), çalışan bir kütüphane (içe/dışa
aktarma, yerel klasörler, çalma listeleri), izin verildiği ölçüde sağlayıcı
çeşitliliği ve ancak yerini hak ederse çok platformluluk.

Üçüncü taraf servisler kendi hesaplarını, aboneliklerini veya API erişimini
gerektirebilir. Melyar, başka bir servisin ücretli özelliklerini ücretsiz
yapmaz.

---

## Geliştirme

```bash
cargo fmt --all
cargo clippy --manifest-path core_rs/Cargo.toml --all-targets
cargo test --manifest-path core_rs/Cargo.toml
cd ui_web && pnpm build   # Qt varlıkları değiştiyse masaüstü uygulamasını yeniden derleyin
```

Her push bu denetimlerin yanında tam bir Qt6 derlemesini, üç pencere
boyutunda iki temada arayüz denetimini ve bellek bütçesi kontrolünü
[GitHub Actions](.github/workflows/ci.yml) üzerinden çalıştırır. Değişiklik
göndermeden önce etkilenen özelliği gerçek masaüstü uygulamasında deneyin -
yalnızca tarayıcı önizlemesinde değil.

---

## Katkı

Katkılara, hata raporlarına ve odaklı pull request'lere açığız - **yeni
katkıcılar arıyoruz**, başlangıç için
[good first issues](ROADMAP.md#good-first-issues). Tam rehber
**[CONTRIBUTING.md](CONTRIBUTING.md)**, sistem tasarımı ise
**[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** içinde.

Değişiklikleri odaklı tutun, çalışan çalma ve köprü davranışını koruyun,
derleme çıktısı, kimlik bilgisi veya kişisel veri commit etmeyin. Arayüz
değişikliklerine ekran görüntüsü, çalma/söz düzeltmelerine gizli veriler
çıkarılmış yeniden üretilebilir adımlar ve günlük ekleyin.

---

## Hata raporları

**Hata raporu şablonunu** kullanın - sürüm, masaüstü ortamı, beklenen ve
gerçekleşen davranış, yeniden üretilebilir adımlar. Düzen sorunlarında pencere
boyutunu ve kuyruk/genişletilmiş oynatıcının açık olup olmadığını; şarkı sözü
sorunlarında parçayı, çalma konumunu ve sözlerin zaman damgalı olup olmadığını
ekleyin.

**Güvenlik sorunları** açık issue değildir -
[.github/SECURITY.md](.github/SECURITY.md). Katılım
[Davranış Kuralları](.github/CODE_OF_CONDUCT.md) ile düzenlenir.

---

## Lisans

Melyar çift lisanslıdır; aşağıdakilerden birini seçebilirsiniz:
**MIT** ([`LICENSE-MIT`](LICENSE-MIT)) veya
**Apache License 2.0** ([`LICENSE-APACHE`](LICENSE-APACHE)).
`SPDX-License-Identifier: MIT OR Apache-2.0`

---

## Notlar

- Melyar açık kaynak yazılımlar üzerine kurulur ve modern masaüstü müzik
  oynatıcı tasarımı ile Liquid Glass görsel kavramlarından ilham alır. Üçüncü
  taraf projeler sahiplerine aittir ve kendi lisanslarına tabidir.
- Melyar resmi olmayan bir istemcidir; YouTube, Google, Spotify veya başka
  bir sağlayıcı ile bağlantılı, onaylanmış veya sponsor değildir. İçerik
  lisansları, hesap şartları ve bulunduğunuz ülkenin yasaları size kullanıcı
  olarak uygulanır.

---

<div align="center">
<sub>Rust, Qt 6, React ve detaylara gösterilen özenle üretildi.</sub>
</div>
