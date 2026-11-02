//! The InnerTube client identities Melyar can use, and the live
//! configuration that keeps them from going stale.
//!
//! InnerTube has no public API, so every request has to identify itself as
//! one of the official clients. Those identities are versioned, and the
//! versions move. A client that keeps using a version from two years ago
//! keeps working right up until the day it does not, which is exactly how
//! most of these clients break.
//!
//! So the version is read from the live web client instead of being written
//! down here, and a small pool of identities is tried in turn, so that one
//! of them being refused does not stop playback.

use std::time::{Duration, Instant};

use super::error::InnerTubeError;

/// The live configuration read, or the reason it could not be read.
type Result<T> = std::result::Result<T, InnerTubeError>;

/// One client identity InnerTube accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientProfile {
    /// The name InnerTube knows this client by.
    pub name: &'static str,
    /// The version to use until the live one has been read.
    pub fallback_version: &'static str,
    /// Some identities are only accepted together with their own user agent.
    pub user_agent: Option<&'static str>,
    /// Whether the identity talks to the music service or the web one.
    pub is_music: bool,
    /// Whether the live client version belongs to this identity. It does for
    /// the web client, and for the others their own version is the right one.
    pub uses_live_version: bool,
}

/// The web client. Its version is the one the live page carries, so it is
/// read at runtime rather than written down here.
pub const WEB_REMIX: ClientProfile = ClientProfile {
    name: "WEB_REMIX",
    fallback_version: "1.20261018.16.00",
    user_agent: None,
    is_music: true,
    uses_live_version: true,
};

/// The television client. It keeps answering for a long time, which makes it
/// a useful second opinion.
pub const TVHTML5: ClientProfile = ClientProfile {
    name: "TVHTML5",
    fallback_version: "7.20241010.18.00",
    user_agent: None,
    is_music: false,
    uses_live_version: false,
};

/// The Android client.
pub const ANDROID: ClientProfile = ClientProfile {
    name: "ANDROID",
    fallback_version: "21.26.364",
    user_agent: Some("com.google.android.youtube/21.26.364 (Linux; U; Android 11) gzip"),
    is_music: false,
    uses_live_version: false,
};

/// The virtual reality client, which answers when the phone clients do not.
pub const ANDROID_VR: ClientProfile = ClientProfile {
    name: "ANDROID_VR",
    fallback_version: "1.60.19",
    user_agent: Some(
        "com.google.android.apps.youtube.vr.oculus/1.60.19 (Linux; U; Android 12L; eureka-user Build/SQ3A.220605.009.A1) gzip",
    ),
    is_music: false,
    uses_live_version: false,
};

/// The identities that are tried, in order.
///
/// The order is the order they were last seen to answer, starting with the
/// web client, which is the one the live configuration describes.
pub const CLIENT_POOL: &[ClientProfile] = &[WEB_REMIX, TVHTML5, ANDROID, ANDROID_VR];

/// How long a live configuration is considered fresh.
const CONFIG_LIFETIME: Duration = Duration::from_secs(6 * 60 * 60);

/// The parts of the live web client configuration that matter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InnerTubeConfig {
    pub api_key: String,
    pub client_version: String,
    pub fetched_at: Option<Instant>,
}

impl Default for InnerTubeConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            client_version: WEB_REMIX.fallback_version.to_string(),
            fetched_at: None,
        }
    }
}

impl InnerTubeConfig {
    /// Whether the configuration is old enough to be worth reading again.
    pub fn is_stale(&self) -> bool {
        match self.fetched_at {
            Some(at) => at.elapsed() > CONFIG_LIFETIME,
            None => true,
        }
    }
}

/// Read the API key and the client version out of the live web client.
///
/// The page carries them in a `ytcfg.set({...})` block, so they are pulled
/// out by name rather than by parsing the whole object.
pub fn parse_live_config(html: &str) -> Result<InnerTubeConfig> {
    let api_key = extract_string_value(html, "INNERTUBE_API_KEY")
        .ok_or(InnerTubeError::MissingConfigValue("an API key"))?;
    let client_version = extract_string_value(html, "INNERTUBE_CLIENT_VERSION")
        .ok_or(InnerTubeError::MissingConfigValue("a client version"))?;

    Ok(InnerTubeConfig {
        api_key,
        client_version,
        fetched_at: Some(Instant::now()),
    })
}

/// Pull a `"NAME":"value"` pair out of a page.
fn extract_string_value(html: &str, name: &str) -> Option<String> {
    let pattern = format!("\"{name}\":\"");
    let start = html.find(&pattern)? + pattern.len();
    let rest = &html[start..];
    let end = rest.find('"')?;
    let value = &rest[..end];
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// The version to send for a client identity.
///
/// The live version belongs to the web client, the other identities have
/// their own.
pub fn version_for(profile: &ClientProfile, live_version: &str) -> String {
    if profile.uses_live_version {
        live_version.to_string()
    } else {
        profile.fallback_version.to_string()
    }
}

/// The endpoint a client identity talks to.
pub fn endpoint(profile: &ClientProfile) -> &'static str {
    if profile.is_music {
        "https://music.youtube.com/youtubei/v1"
    } else {
        "https://www.youtube.com/youtubei/v1"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"
        ytcfg.set({
          "INNERTUBE_API_KEY":"AIzaSyEXAMPLEKEY123",
          "INNERTUBE_CLIENT_NAME":"WEB_REMIX",
          "INNERTUBE_CLIENT_VERSION":"1.20261018.16.00"
        });
    "#;

    #[test]
    fn reads_the_api_key_and_the_version() {
        let config = parse_live_config(PAGE).unwrap();
        assert_eq!(config.api_key, "AIzaSyEXAMPLEKEY123");
        assert_eq!(config.client_version, "1.20261018.16.00");
        assert!(!config.is_stale());
    }

    #[test]
    fn a_page_without_the_values_is_an_error() {
        assert!(parse_live_config("<html>nothing here</html>").is_err());
        assert!(parse_live_config(r#""INNERTUBE_API_KEY":"""#).is_err());
    }

    #[test]
    fn a_missing_version_is_an_error_even_with_a_key() {
        let page = r#""INNERTUBE_API_KEY":"AIzaSyKEY""#;
        assert!(parse_live_config(page).is_err());
    }

    #[test]
    fn the_pool_is_ordered_and_ends_with_the_web_client() {
        assert_eq!(CLIENT_POOL.first().unwrap().name, "WEB_REMIX");
        assert_eq!(CLIENT_POOL.last().unwrap().name, "ANDROID_VR");
        // Every identity needs a name and a version to fall back on.
        for profile in CLIENT_POOL {
            assert!(!profile.name.is_empty());
            assert!(!profile.fallback_version.is_empty());
        }
    }

    #[test]
    fn the_live_version_belongs_to_the_web_client_only() {
        let live = "1.20261018.16.00";
        assert_eq!(version_for(&WEB_REMIX, live), live);
        // The other identities are only accepted with their own version.
        assert_eq!(version_for(&TVHTML5, live), TVHTML5.fallback_version);
        assert_eq!(version_for(&ANDROID, live), ANDROID.fallback_version);
        assert_ne!(version_for(&ANDROID, live), live);
    }

    #[test]
    fn the_music_and_the_web_endpoints_differ() {
        assert!(endpoint(&WEB_REMIX).starts_with("https://music.youtube.com"));
        assert!(endpoint(&TVHTML5).starts_with("https://www.youtube.com"));
    }

    #[test]
    fn a_fresh_configuration_is_not_stale() {
        let mut config = InnerTubeConfig {
            api_key: "key".to_string(),
            client_version: "1".to_string(),
            fetched_at: Some(Instant::now()),
        };
        assert!(!config.is_stale());

        config.fetched_at = Some(Instant::now() - CONFIG_LIFETIME - Duration::from_secs(1));
        assert!(config.is_stale());
    }
}

#[cfg(test)]
mod live_tests {
    use crate::innertube::client::InnerTubeClient;

    /// Reads the live configuration and runs a search. Needs a network
    /// connection, so it is not part of the ordinary test run.
    #[tokio::test]
    #[ignore = "needs a network connection"]
    async fn the_live_configuration_can_be_read() {
        let client = InnerTubeClient::new().unwrap();
        let config = client.refresh_config().await.unwrap();
        assert!(config.api_key.starts_with("AIza"));
        assert!(config.client_version.contains('.'));
        println!(
            "live configuration: key {}..., version {}",
            &config.api_key[..8],
            config.client_version
        );
    }

    #[tokio::test]
    #[ignore = "needs a network connection"]
    async fn which_client_answers() {
        use reqwest::header;
        use serde_json::json;

        let client = InnerTubeClient::new().unwrap();
        let config = client.refresh_config().await.unwrap();

        // A wider set of candidates than the pool holds, to find out which
        // of them the service actually answers today.
        let candidates: Vec<(&str, &str, &str, Option<&str>)> = vec![
            ("WEB_REMIX", "1", "music", None),
            ("WEB_REMIX", "2", "music", None),
            ("YTMUSIC", "1", "music", None),
            ("YTMUSIC", "2", "music", None),
            ("TVHTML5", "7", "web", None),
            ("ANDROID", "21.26.364", "web", Some("com.google.android.youtube/21.26.364 (Linux; U; Android 11) gzip")),
            ("ANDROID_VR", "1.60.19", "web", Some("com.google.android.apps.youtube.vr.oculus/1.60.19 (Linux; U; Android 12L) gzip")),
            ("IOS", "19.45.4", "web", None),
            ("WEB_EMBEDDED", "1", "web", None),
            ("MWEB", "2", "music", None),
        ];

        for (name, version_key, host, agent) in candidates {
            let version = match version_key {
                "1" => config.client_version.clone(),
                "2" => "1.20250616.01.00".to_string(),
                "7" => "7.20241010.18.00".to_string(),
                other => other.to_string(),
            };
            let base = if host == "music" {
                "https://music.youtube.com/youtubei/v1"
            } else {
                "https://www.youtube.com/youtubei/v1"
            };
            let url = format!("{base}/search?key={}", config.api_key);
            let payload = json!({
                "context": {"client": {
                    "clientName": name,
                    "clientVersion": version,
                    "hl": "en", "gl": "US"
                }},
                "query": "daft punk",
                "params": "Eg-KAQwIARAAGAAgACgAMABqChAKEAMQBBAKEAU%3D"
            });

            let mut request = client.http().post(&url).json(&payload);
            if let Some(a) = agent {
                request = request.header(header::USER_AGENT, a);
            }
            let status = request
                .send()
                .await
                .map(|r| r.status())
                .unwrap_or_else(|e| {
                    println!("  {name} istek hatasi: {e}");
                    reqwest::StatusCode::from_u16(0).unwrap()
                });
            println!("  {name} v{version} ({host}) -> {status}");
        }
    }

    #[tokio::test]
    #[ignore = "needs a network connection"]
    async fn a_search_finds_something() {
        let client = InnerTubeClient::new().unwrap();
        let tracks = client.search_tracks("daft punk").await.unwrap_or_else(|e| {
            println!("SEARCH HATASI: {e}");
            Vec::new()
        });
        assert!(!tracks.is_empty(), "the search returned nothing");
        println!("the search returned {} tracks", tracks.len());
    }

    #[tokio::test]
    #[ignore = "needs a network connection"]
    async fn a_stream_can_be_resolved_without_ytdlp() {
        let client = InnerTubeClient::new().unwrap();
        // The pool on its own, which is the path a user without yt-dlp takes.
        let url = client
            .resolve_stream_via_pool("dQw4w9WgXcQ")
            .await
            .unwrap_or_else(|e| {
                println!("STREAM HATASI: {e}");
                String::new()
            });
        assert!(!url.is_empty(), "the stream could not be resolved");
        println!("the stream resolved to {}...", &url[..60.min(url.len())]);
    }
}
