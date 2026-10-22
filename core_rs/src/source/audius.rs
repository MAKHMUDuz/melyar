//! Audius, reached through its own open API.
//!
//! This is the cheap kind of source: no key, no login, no process per query.
//! It exists to prove the seam costs nothing when a site is willing to be
//! played — a plain HTTPS round trip for search and another for the stream,
//! which libmpv then follows (the endpoint answers with a redirect to the
//! CDN).
//!
//! The id it mints is `audius:<trackId>`. Nothing else in melyar needs to
//! know what that means until it is time to play it.

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::Value;

use crate::ffi::CxxTrackMetadata;

use super::{Source, SourceKind};

/// One of the discovery providers Audius rotates through. It answers for
/// search and for the redirect that resolves to a stream.
const DISCOVERY_PROVIDER: &str = "https://discoveryprovider.audius.co";
/// Every request carries it; Audius uses it to measure and throttle callers.
const APP_NAME: &str = "melyar";
/// Artwork size to ask for. 480 is the largest square that is still cheap to
/// paint in a list, and every track has it.
const ARTWORK: &str = "480x480";

pub struct AudiusSource {
    http: reqwest::Client,
}

impl AudiusSource {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    fn search_url(query: &str) -> String {
        format!(
            "{DISCOVERY_PROVIDER}/v1/tracks/search?query={}&app_name={APP_NAME}",
            urlencoding::encode(query)
        )
    }

    fn stream_url(track_id: &str) -> String {
        format!("{DISCOVERY_PROVIDER}/v1/tracks/{track_id}/stream?app_name={APP_NAME}")
    }

    /// Turn one Audius track object into the metadata shape the rest of melyar
    /// already understands.
    fn to_meta(item: &Value) -> Option<CxxTrackMetadata> {
        let id = item.get("id")?.as_str()?;
        let title = item.get("title")?.as_str().unwrap_or_default();
        let artist = item
            .get("user")
            .and_then(|u| u.get("name"))
            .and_then(|n| n.as_str())
            .unwrap_or_default();

        // Duration arrives in whole seconds. Anything without one plays as
        // zero-length rather than being dropped — a track is still worth
        // hearing even if its page forgot to say how long it is.
        let duration = item
            .get("duration")
            .and_then(|d| d.as_f64())
            .or_else(|| item.get("duration").and_then(|d| d.as_i64()).map(|i| i as f64))
            .unwrap_or(0.0);

        let thumbnail = item
            .get("artwork")
            .and_then(|a| a.get(ARTWORK))
            .and_then(|t| t.as_str())
            .unwrap_or_default()
            .to_string();

        Some(CxxTrackMetadata {
            id: format!("audius:{id}"),
            title: title.to_string(),
            artist: artist.to_string(),
            album: String::new(),
            duration_secs: duration,
            thumbnail_url: thumbnail,
            lyrics: String::new(),
        })
    }
}

#[async_trait]
impl Source for AudiusSource {
    fn id(&self) -> &'static str {
        "audius"
    }

    fn display_name(&self) -> &'static str {
        "Audius"
    }

    fn description(&self) -> &'static str {
        "Open music from independent artists. No account needed."
    }

    fn kind(&self) -> SourceKind {
        SourceKind::NativeRest
    }

    // Off by default: the rule here is that a source must not slow the common
    // case. Audius would barely register, but "barely" is not "not", and the
    // user can turn it on in one click.
    fn default_enabled(&self) -> bool {
        false
    }

    async fn search(&self, query: &str) -> Result<Vec<CxxTrackMetadata>> {
        let url = Self::search_url(query);
        let resp = self
            .http
            .get(&url)
            .header(reqwest::header::USER_AGENT, APP_NAME)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(anyhow!("Audius search failed: {}", resp.status()));
        }

        // The body is { data: [ ... ] }. A page with no matches comes back as
        // an empty array, which is a valid answer, not an error.
        let body: Value = resp.json().await?;
        let items = body
            .get("data")
            .and_then(|d| d.as_array())
            .map(|a| a.as_slice())
            .unwrap_or(&[]);

        Ok(items.iter().filter_map(Self::to_meta).collect())
    }

    async fn resolve_stream(&self, bare_id: &str) -> Result<String> {
        // Hand libmpv the stream endpoint itself. It answers with a redirect
        // to a signed CDN URL, which libmpv follows the same way it follows any
        // other link — so nothing here has to know about the signature.
        Ok(Self::stream_url(bare_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The search shape Audius actually returns. Offline on purpose: it parses
    /// a trimmed copy of a real response, so no network and no flake.
    #[test]
    fn maps_a_real_search_entry() {
        let item: Value = serde_json::from_str(
            r#"{
                "id": "vZJJz",
                "title": "workit (daftpunk flip)",
                "duration": 190,
                "user": {"name": "chromonicci."},
                "artwork": {"150x150": "a.jpg", "480x480": "b.jpg"}
            }"#,
        )
        .unwrap();

        let meta = AudiusSource::to_meta(&item).expect("entry should map");
        assert_eq!(meta.id, "audius:vZJJz");
        assert_eq!(meta.artist, "chromonicci.");
        assert_eq!(meta.duration_secs, 190.0);
        assert_eq!(meta.thumbnail_url, "b.jpg");
    }

    /// An entry without a duration must still play rather than be dropped.
    #[test]
    fn entry_without_duration_still_maps() {
        let item: Value = serde_json::from_str(
            r#"{"id": "abc", "title": "T", "user": {"name": "A"}}"#,
        )
        .unwrap();
        let meta = AudiusSource::to_meta(&item).expect("should map");
        assert_eq!(meta.duration_secs, 0.0);
    }
}
