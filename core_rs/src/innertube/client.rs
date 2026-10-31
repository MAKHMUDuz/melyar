use anyhow::{anyhow, Context, Result};
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use reqwest::{header, Client};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use super::cipher::CipherSolver;
use super::clients::{endpoint, parse_live_config, InnerTubeConfig, CLIENT_POOL};
use super::parser::parse_search_results;
use crate::ffi::CxxTrackMetadata;

/// The API key as a last resort, when the live configuration cannot be read.
const FALLBACK_API_KEY: &str = "AIzaSyC9XL3ZjWddXya6X74dJoCTL-WEYFDNX30";
const SEARCH_PATH: &str = "search";
const PLAYER_PATH: &str = "player";
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:128.0) Gecko/20100101 Firefox/128.0";
/// Where the live client configuration is read from.
const CONFIG_URL: &str = "https://music.youtube.com/";

/// Resolved video: metadata + direct audio stream url (single yt-dlp call).
#[derive(Clone, Debug, Default)]
pub struct ResolvedVideo {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration: f64,
    pub thumbnail: String,
    pub stream_url: String,
}

impl ResolvedVideo {
    pub fn to_meta(&self) -> CxxTrackMetadata {
        CxxTrackMetadata {
            id: self.id.clone(),
            title: self.title.clone(),
            artist: self.artist.clone(),
            album: self.album.clone(),
            duration_secs: self.duration,
            thumbnail_url: self.thumbnail.clone(),
            lyrics: String::new(),
        }
    }
}

static YTDLP_PATH: Lazy<Option<PathBuf>> = Lazy::new(|| {
    let names = ["yt-dlp", "yt-dlp_linux", "youtube-dl"];
    // 1. PATH
    for name in names {
        if let Ok(out) = Command::new("which").arg(name).output() {
            if out.status.success() {
                let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !p.is_empty() {
                    return Some(PathBuf::from(p));
                }
            }
        }
    }
    // 2. Common locations
    let home = std::env::var("HOME").unwrap_or_default();
    let mut candidates = vec![
        PathBuf::from("/usr/bin/yt-dlp"),
        PathBuf::from("/usr/local/bin/yt-dlp"),
        PathBuf::from("/opt/yt-dlp/yt-dlp"),
        PathBuf::from(home.clone()).join(".local/bin/yt-dlp"),
        PathBuf::from(home).join("bin/yt-dlp"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        for py in ["3.14", "3.13", "3.12", "3.11", "3.10"] {
            candidates.push(
                PathBuf::from(&home)
                    .join(format!(".local/lib/python{}/site-packages/bin/yt-dlp", py)),
            );
        }
    }
    candidates.into_iter().find(|p| p.exists() && p.is_file())
});

fn yt_dlp_available() -> bool {
    YTDLP_PATH.is_some()
}

fn yt_dlp_command() -> Command {
    let mut cmd = Command::new(YTDLP_PATH.as_ref().unwrap());
    cmd.arg("--no-warnings")
        .arg("--no-playlist")
        .arg("--no-progress")
        .arg("--ignore-config");
    cmd
}

/// Run yt-dlp -J (single JSON) and parse metadata + best audio stream url.
fn yt_dlp_json_blocking(video_id: &str) -> Result<ResolvedVideo> {
    if !yt_dlp_available() {
        return Err(anyhow!("yt-dlp binary not found"));
    }
    let watch_url = format!("https://www.youtube.com/watch?v={}", video_id);
    let output = yt_dlp_command()
        .arg("-J")
        .arg(&watch_url)
        .output()
        .context("yt-dlp -J failed to spawn")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "yt-dlp -J failed for {}: {}",
            video_id,
            err.lines().next().unwrap_or("")
        ));
    }

    let data: Value = serde_json::from_slice(&output.stdout).context("yt-dlp -J invalid JSON")?;
    parse_ytdlp_json(&data, video_id)
}

fn parse_ytdlp_json(data: &Value, video_id: &str) -> Result<ResolvedVideo> {
    let title = data
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Bilinmeyen Parça")
        .to_string();
    let artist = data
        .get("artist")
        .or_else(|| data.get("uploader"))
        .or_else(|| data.get("channel"))
        .and_then(|v| v.as_str())
        .unwrap_or("Bilinmeyen Sanatçı")
        .to_string();
    let album = data
        .get("album")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let duration = data.get("duration").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let thumbnail = data
        .get("thumbnail")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("https://i.ytimg.com/vi/{}/hqdefault.jpg", video_id));

    // Pick the best audio-only https stream (prefer mp4/aac itag 140, else highest abr)
    let mut best: Option<(i64, String)> = None;
    if let Some(formats) = data.get("formats").and_then(|v| v.as_array()) {
        for f in formats {
            let acodec = f.get("acodec").and_then(|v| v.as_str()).unwrap_or("none");
            let vcodec = f.get("vcodec").and_then(|v| v.as_str()).unwrap_or("none");
            if acodec == "none" || vcodec != "none" {
                continue;
            }
            let Some(url) = f.get("url").and_then(|v| v.as_str()) else {
                continue;
            };
            let protocol = f.get("protocol").and_then(|v| v.as_str()).unwrap_or("");
            if protocol.contains("m3u8") {
                // HLS works in mpv but prefer progressive https
                continue;
            }
            let abr = f.get("abr").and_then(|v| v.as_f64()).unwrap_or(0.0) as i64;
            let is_mp4a = acodec.starts_with("mp4a");
            let score = abr + if is_mp4a { 1000 } else { 0 };
            if best.as_ref().is_none_or(|(s, _)| score > *s) {
                best = Some((score, url.to_string()));
            }
        }
    }

    let stream_url = best.map(|(_, u)| u).or_else(|| {
        // Fallback: any format url (even manifests — mpv handles them)
        data.get("formats")
            .and_then(|v| v.as_array())
            .and_then(|formats| {
                formats
                    .iter()
                    .filter_map(|f| f.get("url").and_then(|u| u.as_str()))
                    .next()
                    .map(|s| s.to_string())
            })
    });

    match stream_url {
        Some(url) => Ok(ResolvedVideo {
            id: video_id.to_string(),
            title,
            artist,
            album,
            duration,
            thumbnail,
            stream_url: url,
        }),
        None => Err(anyhow!(
            "no usable audio stream in yt-dlp output for {}",
            video_id
        )),
    }
}

/// How long a resolved stream URL is reused.
///
/// InnerTube URLs are short-lived. Thirty seconds is long enough to cover the
/// gap between a gapless prefetch and the track actually starting, and short
/// enough that an expired URL is never handed to mpv.
const STREAM_URL_TTL: Duration = Duration::from_secs(30);

/// A URL remembered for a short while, together with the moment it was
/// resolved.
#[derive(Debug)]
struct CachedStreamUrl {
    url: String,
    inserted_at: Instant,
}

/// A cached URL is fresh while it is younger than the TTL.
fn cache_entry_is_fresh(inserted_at: Instant, now: Instant) -> bool {
    now.duration_since(inserted_at) < STREAM_URL_TTL
}

/// Short-lived cache of resolved stream URLs, keyed by video id.
///
/// The gapless prefetch resolves the next track early. Without this cache the
/// same track resolved again when it actually started, doubling the InnerTube
/// requests for every transition.
#[derive(Default)]
struct StreamUrlCache {
    entries: Mutex<HashMap<String, CachedStreamUrl>>,
}

impl StreamUrlCache {
    /// Return a still-fresh URL, dropping it when it has expired.
    fn get(&self, video_id: &str) -> Option<String> {
        let now = Instant::now();
        let mut entries = self.entries.lock();
        match entries.get(video_id) {
            Some(entry) if cache_entry_is_fresh(entry.inserted_at, now) => Some(entry.url.clone()),
            Some(_) => {
                entries.remove(video_id);
                None
            }
            None => None,
        }
    }

    /// Remember a resolved URL for the TTL.
    fn insert(&self, video_id: &str, url: &str) {
        let now = Instant::now();
        let mut entries = self.entries.lock();
        entries.insert(
            video_id.to_string(),
            CachedStreamUrl {
                url: url.to_string(),
                inserted_at: now,
            },
        );

        // The TTL is the real invalidation. This cap only keeps a long
        // listening session from retaining every expired entry.
        if entries.len() > 64 {
            entries.retain(|_, entry| cache_entry_is_fresh(entry.inserted_at, now));
        }
    }

    #[cfg(test)]
    fn insert_at(&self, video_id: &str, url: &str, inserted_at: Instant) {
        self.entries.lock().insert(
            video_id.to_string(),
            CachedStreamUrl {
                url: url.to_string(),
                inserted_at,
            },
        );
    }
}

pub struct InnerTubeClient {
    http: Client,
    cipher: CipherSolver,
    /// The live client configuration, read once and refreshed when it goes
    /// stale. Without it the client version would be written down here and
    /// would be wrong within a few months.
    config: Mutex<InnerTubeConfig>,
    /// Which identity in the pool worked last, so the working one is tried
    /// first.
    preferred: Mutex<usize>,
    /// Resolved stream URLs, shared by prefetch and playback.
    stream_cache: StreamUrlCache,
}

impl InnerTubeClient {
    pub fn new() -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            header::HeaderValue::from_static(USER_AGENT),
        );
        headers.insert(
            header::ORIGIN,
            header::HeaderValue::from_static("https://music.youtube.com"),
        );
        headers.insert(
            header::REFERER,
            header::HeaderValue::from_static("https://music.youtube.com/"),
        );
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/json"),
        );

        let http = Client::builder()
            .default_headers(headers)
            .http2_prior_knowledge()
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .timeout(Duration::from_secs(15))
            .gzip(true)
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            http,
            cipher: CipherSolver::new(),
            config: Mutex::new(InnerTubeConfig::default()),
            preferred: Mutex::new(0),
            stream_cache: StreamUrlCache::default(),
        })
    }

    /// Blocking metadata+url resolve for calls coming from Qt worker threads.
    pub fn resolve_video_blocking(&self, video_id: &str) -> Result<ResolvedVideo> {
        yt_dlp_json_blocking(video_id)
    }

    /// The HTTP client, for the tests that talk to the service directly.
    #[cfg(test)]
    pub fn http(&self) -> &Client {
        &self.http
    }

    /// The API key to use, from the live configuration when it is known.
    fn api_key(&self) -> String {
        let config = self.config.lock();
        if config.api_key.is_empty() {
            FALLBACK_API_KEY.to_string()
        } else {
            config.api_key.clone()
        }
    }

    /// The client version to send, from the live configuration when it is
    /// known.
    fn client_version(&self) -> String {
        self.config.lock().client_version.clone()
    }

    /// Read the live client configuration, so the version stays current.
    ///
    /// A failure here is not fatal. The fallback version is used until the
    /// configuration can be read.
    pub async fn refresh_config(&self) -> Result<InnerTubeConfig> {
        let resp = self.http.get(CONFIG_URL).send().await?;
        let html = resp.text().await?;
        let config = parse_live_config(&html)?;
        *self.config.lock() = config.clone();
        Ok(config)
    }

    /// Make sure the configuration is fresh before a request goes out.
    async fn ensure_config(&self) {
        let stale = self.config.lock().is_stale();
        if !stale {
            return;
        }
        if let Err(e) = self.refresh_config().await {
            eprintln!("Could not read the live InnerTube configuration: {e}");
        }
    }

    /// The identities to try, starting with the one that worked last.
    fn pool_order(&self) -> Vec<super::clients::ClientProfile> {
        let start = *self.preferred.lock() % CLIENT_POOL.len();
        let mut order = Vec::with_capacity(CLIENT_POOL.len());
        for offset in 0..CLIENT_POOL.len() {
            order.push(CLIENT_POOL[(start + offset) % CLIENT_POOL.len()]);
        }
        order
    }

    /// Remember that an identity worked, so it is tried first next time.
    fn remember(&self, name: &str) {
        if let Some(index) = CLIENT_POOL.iter().position(|p| p.name == name) {
            *self.preferred.lock() = index;
        }
    }

    /// The context an InnerTube request is sent with.
    fn client_context(&self, profile: &super::clients::ClientProfile) -> Value {
        let version = super::clients::version_for(profile, &self.client_version());
        let mut client = json!({
            "clientName": profile.name,
            "clientVersion": version,
            "hl": "en",
            "gl": "US"
        });
        // The phone clients are only accepted when they are told which
        // platform they are running on.
        if profile.name == "ANDROID" || profile.name == "ANDROID_VR" {
            client["androidSdkVersion"] = json!(30);
        }
        json!({ "context": { "client": client } })
    }

    /// Search tracks on YouTube Music.
    ///
    /// The identities are tried in turn, because the one that works today is
    /// not guaranteed to work tomorrow.
    pub async fn search_tracks(&self, query: &str) -> Result<Vec<CxxTrackMetadata>> {
        self.ensure_config().await;
        let key = self.api_key();

        let mut last_error: anyhow::Error = anyhow!("no InnerTube client could answer the search");

        for profile in self.pool_order() {
            let url = format!("{}/{SEARCH_PATH}?key={key}", endpoint(&profile));
            let mut payload = self.client_context(&profile);
            payload["query"] = json!(query);
            payload["params"] = json!("Eg-KAQwIARAAGAAgACgAMABqChAKEAMQBBAKEAU%3D");

            let request = self
                .http
                .post(&url)
                .json(&payload)
                .header(header::USER_AGENT, profile.user_agent.unwrap_or(USER_AGENT));

            match request.send().await {
                Ok(resp) if resp.status().is_success() => match resp.json::<Value>().await {
                    Ok(data) => {
                        if let Ok(tracks) = parse_search_results(&data) {
                            if !tracks.is_empty() {
                                self.remember(profile.name);
                                return Ok(tracks);
                            }
                        }
                        last_error = anyhow!("the search returned nothing");
                    }
                    Err(e) => last_error = e.into(),
                },
                Ok(resp) => {
                    last_error = anyhow!("the search failed with status {}", resp.status());
                }
                Err(e) => last_error = e.into(),
            }
        }

        Err(last_error)
    }

    /// Resolve direct audio stream URL for a given track/video id.
    /// Primary: yt-dlp (handles PO tokens + deciphering).
    /// Fallback: innertube clients + public APIs.
    pub async fn get_stream_url(&self, video_id: &str) -> Result<String> {
        // A prefetch may have resolved this track a moment ago.
        if let Some(url) = self.stream_cache.get(video_id) {
            return Ok(url);
        }

        // 1. yt-dlp (metadata json gives us both url and meta in one shot)
        if yt_dlp_available() {
            let vid = video_id.to_string();
            let resolved = tokio::task::spawn_blocking(move || yt_dlp_json_blocking(&vid)).await;
            if let Ok(Ok(res)) = resolved {
                if !res.stream_url.is_empty() {
                    self.stream_cache.insert(video_id, &res.stream_url);
                    return Ok(res.stream_url);
                }
            }
        }

        // 2. The InnerTube player endpoint, trying the identities in turn.
        let url = self.resolve_stream_via_pool(video_id).await?;
        self.stream_cache.insert(video_id, &url);
        Ok(url)
    }

    /// Resolve a stream using only the InnerTube identities, with no help
    /// from yt-dlp.
    ///
    /// This is the path a user without yt-dlp takes, so it is worth keeping
    /// separate and worth testing on its own.
    pub async fn resolve_stream_via_pool(&self, video_id: &str) -> Result<String> {
        self.ensure_config().await;
        let key = self.api_key();

        for profile in self.pool_order() {
            let url = format!("{}/{PLAYER_PATH}?key={key}", endpoint(&profile));
            let mut payload = self.client_context(&profile);
            payload["videoId"] = json!(video_id);

            let mut request = self.http.post(&url).json(&payload);
            if let Some(agent) = profile.user_agent {
                request = request.header(header::USER_AGENT, agent);
            }

            if let Ok(resp) = request.send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<Value>().await {
                        if let Some(stream_url) = self.extract_best_audio_stream(&data).await {
                            self.remember(profile.name);
                            return Ok(stream_url);
                        }
                    }
                }
            }
        }

        // 3. Fallback: Piped / Invidious audio stream resolver
        let fallback_apis = [
            format!("https://pipedapi.kavin.rocks/streams/{video_id}"),
            format!("https://api.piped.privacydev.net/streams/{video_id}"),
            format!("https://invidious.asir.dev/api/v1/videos/{video_id}"),
        ];

        for fallback in fallback_apis {
            if let Ok(resp) = self
                .http
                .get(&fallback)
                .timeout(Duration::from_secs(5))
                .send()
                .await
            {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<Value>().await {
                        if let Some(audio_streams) =
                            data.get("audioStreams").and_then(|v| v.as_array())
                        {
                            for s in audio_streams {
                                if let Some(stream_url) = s.get("url").and_then(|u| u.as_str()) {
                                    return Ok(stream_url.to_string());
                                }
                            }
                        }
                        if let Some(adaptive) =
                            data.get("adaptiveFormats").and_then(|v| v.as_array())
                        {
                            for f in adaptive {
                                if f.get("type")
                                    .and_then(|t| t.as_str())
                                    .is_some_and(|t| t.starts_with("audio"))
                                {
                                    if let Some(stream_url) = f.get("url").and_then(|u| u.as_str())
                                    {
                                        return Ok(stream_url.to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Err(anyhow!(
            "Failed to extract audio stream for video {video_id}"
        ))
    }

    async fn extract_best_audio_stream(&self, player_json: &Value) -> Option<String> {
        let streaming_data = player_json.get("streamingData")?;

        // 1. Check adaptive formats first (Opus itag 251 / AAC audio streams)
        if let Some(formats) = streaming_data
            .get("adaptiveFormats")
            .and_then(|v| v.as_array())
        {
            let mut audio_formats: Vec<&Value> = formats
                .iter()
                .filter(|f| {
                    f.get("mimeType")
                        .and_then(|m| m.as_str())
                        .is_some_and(|m| m.starts_with("audio/"))
                })
                .collect();

            audio_formats.sort_by(|a, b| {
                let itag_a = a.get("itag").and_then(|v| v.as_i64()).unwrap_or(0);
                let itag_b = b.get("itag").and_then(|v| v.as_i64()).unwrap_or(0);

                if itag_a == 251 {
                    return std::cmp::Ordering::Less;
                }
                if itag_b == 251 {
                    return std::cmp::Ordering::Greater;
                }

                let br_a = a.get("bitrate").and_then(|v| v.as_i64()).unwrap_or(0);
                let br_b = b.get("bitrate").and_then(|v| v.as_i64()).unwrap_or(0);
                br_b.cmp(&br_a)
            });

            for fmt in audio_formats {
                if let Some(direct_url) = fmt.get("url").and_then(|v| v.as_str()) {
                    return Some(direct_url.to_string());
                }

                let cipher_field = fmt
                    .get("signatureCipher")
                    .or_else(|| fmt.get("cipher"))
                    .and_then(|v| v.as_str());

                if let Some(cipher_str) = cipher_field {
                    if let Ok(solved) = self.cipher.solve(cipher_str, None) {
                        return Some(solved);
                    }
                }
            }
        }

        // 2. Check progressive formats (contains audio+video, libmpv plays audio-only via vo=null)
        if let Some(formats) = streaming_data.get("formats").and_then(|v| v.as_array()) {
            for fmt in formats {
                if let Some(direct_url) = fmt.get("url").and_then(|v| v.as_str()) {
                    return Some(direct_url.to_string());
                }

                let cipher_field = fmt
                    .get("signatureCipher")
                    .or_else(|| fmt.get("cipher"))
                    .and_then(|v| v.as_str());

                if let Some(cipher_str) = cipher_field {
                    if let Ok(solved) = self.cipher.solve(cipher_str, None) {
                        return Some(solved);
                    }
                }
            }
        }

        None
    }
}

/// Serialize yt-dlp json calls behind a mutex to avoid hammering with parallel lookups.
static YTDLP_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub fn with_ytdlp_lock<T>(f: impl FnOnce() -> T) -> T {
    let _guard = YTDLP_LOCK.lock();
    f()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_stream_url_is_returned() {
        let cache = StreamUrlCache::default();
        cache.insert("video-1", "https://example.test/audio-1");

        assert_eq!(
            cache.get("video-1").as_deref(),
            Some("https://example.test/audio-1")
        );
        assert_eq!(cache.get("another-video"), None);
    }

    #[test]
    fn expired_stream_url_is_dropped() {
        let cache = StreamUrlCache::default();
        let expired = Instant::now()
            .checked_sub(STREAM_URL_TTL + Duration::from_secs(1))
            .expect("the test clock is far enough from the epoch");
        cache.insert_at("video-2", "https://example.test/audio-2", expired);

        assert_eq!(cache.get("video-2"), None);
        assert!(!cache.entries.lock().contains_key("video-2"));
    }

    #[test]
    fn ttl_boundary_is_treated_as_expired() {
        let now = Instant::now();
        let inserted = now
            .checked_sub(STREAM_URL_TTL)
            .expect("the test clock is far enough from the epoch");

        assert!(!cache_entry_is_fresh(inserted, now));
        assert!(cache_entry_is_fresh(
            inserted + Duration::from_millis(1),
            now
        ));
    }
}
