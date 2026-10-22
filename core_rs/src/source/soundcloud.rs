//! SoundCloud, reached by asking yt-dlp rather than speaking its API.
//!
//! This is the expensive kind of source and it is honest about it: every query
//! starts a process. That is a real cost, so it is off until the user asks for
//! it. What it buys is reach — yt-dlp already knows the site's shifting
//! internals, so nothing here breaks when SoundCloud changes a header.
//!
//! Search goes through yt-dlp's `scsearchN:` prefix, which yields a flat list
//! without downloading anything. The id it mints is `soundcloud:<trackId>`,
//! and playback asks yt-dlp for that id's audio, preferring a progressive MP3
//! and settling for the HLS stream SoundCloud mostly serves now — libmpv reads
//! either.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use serde_json::Value;

use crate::ffi::CxxTrackMetadata;

use super::{Source, SourceKind};

/// Where the yt-dlp binary might be. Mirrors the search the InnerTube client
/// already does, so a user who set up melyar once does not have to twice.
static YTDLP: Lazy<Option<PathBuf>> = Lazy::new(find_yt_dlp);

fn find_yt_dlp() -> Option<PathBuf> {
    for name in ["yt-dlp", "yt-dlp_linux"] {
        if let Ok(out) = Command::new("which").arg(name).output() {
            if out.status.success() {
                let p = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !p.is_empty() {
                    return Some(PathBuf::from(p));
                }
            }
        }
    }
    let home = std::env::var("HOME").ok()?;
    for candidate in [
        PathBuf::from(&home).join(".local/bin/yt-dlp"),
        PathBuf::from("/usr/bin/yt-dlp"),
        PathBuf::from("/usr/local/bin/yt-dlp"),
    ] {
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

pub struct SoundCloudSource;

impl SoundCloudSource {
    pub fn new() -> Self {
        Self
    }

    /// Run yt-dlp and give it back as text. Blocking, so callers put it on the
    /// blocking pool — a process wait has no place on an async worker.
    fn run(args: &[&str]) -> Result<String> {
        let bin = YTDLP
            .as_ref()
            .ok_or_else(|| anyhow!("yt-dlp is not installed"))?;
        let out = Command::new(bin)
            .args(args)
            .output()
            .context("failed to run yt-dlp")?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(anyhow!("yt-dlp failed: {}", err.trim()));
        }
        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    /// Ask for a flat JSON list of matches without touching their media.
    fn search_blocking(query: &str, limit: u32) -> Result<Vec<CxxTrackMetadata>> {
        let spec = format!("scsearch{limit}:{query}");
        let stdout = Self::run(&["--flat-playlist", "-J", "--no-warnings", &spec])?;
        let data: Value =
            serde_json::from_str(&stdout).context("yt-dlp returned malformed JSON")?;

        let entries = data
            .get("entries")
            .and_then(|e| e.as_array())
            .cloned()
            .unwrap_or_default();

        Ok(entries
            .iter()
            .filter_map(|e| {
                // The id arrives as a string on most entries and as a number on
                // some; dropping the row for the wrong JSON type would silently
                // shorten every search.
                let id = match e.get("id") {
                    Some(Value::String(s)) => s.clone(),
                    Some(Value::Number(n)) => n.to_string(),
                    _ => return None,
                };
                let title = e.get("title")?.as_str().unwrap_or_default().to_string();
                // A flat entry names the uploader, not an "artist" field — the
                // uploader is who it plays as everywhere else on the site.
                let artist = e
                    .get("uploader")
                    .or_else(|| e.get("channel"))
                    .and_then(|a| a.as_str())
                    .unwrap_or_default()
                    .to_string();
                let duration = e.get("duration").and_then(|d| d.as_f64()).unwrap_or(0.0);
                let thumbnail = e
                    .get("thumbnails")
                    .and_then(|t| t.as_array())
                    .and_then(|t| t.last())
                    .and_then(|t| t.get("url"))
                    .and_then(|u| u.as_str())
                    .unwrap_or_default()
                    .to_string();

                Some(CxxTrackMetadata {
                    id: format!("soundcloud:{id}"),
                    title,
                    artist,
                    album: String::new(),
                    duration_secs: duration,
                    thumbnail_url: thumbnail,
                    lyrics: String::new(),
                })
            })
            .collect())
    }

    /// Turn a SoundCloud track id or page URL into a URL libmpv can open.
    fn resolve_blocking(bare_id: &str) -> Result<String> {
        // A bare number resolves through the API endpoint, which answers for
        // any track id. `soundcloud.com/tracks/<id>` — the path that reads more
        // natural — is a 404: the site only knows pages by their slug, not by
        // id. A full page URL (what search used to hand back) goes through
        // untouched, since yt-dlp reads those itself.
        let target = if bare_id.starts_with("http") {
            bare_id.to_string()
        } else {
            format!("https://api.soundcloud.com/tracks/{bare_id}")
        };

        // Prefer a progressive MP3: libmpv plays it directly, no manifest. Fall
        // back to whatever best audio exists rather than nothing.
        let stdout = Self::run(&[
            "-f",
            "bestaudio[protocol^=http][ext=mp3]/bestaudio[ext=m4a]/bestaudio",
            "--get-url",
            "--no-warnings",
            &target,
        ])?;
        let url = stdout
            .lines()
            .map(str::trim)
            .find(|l| !l.is_empty())
            .ok_or_else(|| anyhow!("yt-dlp found no audio for {bare_id}"))?;
        Ok(url.to_string())
    }
}

#[async_trait]
impl Source for SoundCloudSource {
    fn id(&self) -> &'static str {
        "soundcloud"
    }

    fn display_name(&self) -> &'static str {
        "SoundCloud"
    }

    fn description(&self) -> &'static str {
        "Searches and plays from SoundCloud via yt-dlp."
    }

    fn kind(&self) -> SourceKind {
        SourceKind::YtDlp
    }

    async fn search(&self, query: &str) -> Result<Vec<CxxTrackMetadata>> {
        let q = query.to_string();
        // yt-dlp waits on a process; that belongs on the blocking pool, never
        // on an async worker that other requests are sharing.
        tokio::task::spawn_blocking(move || Self::search_blocking(&q, 15))
            .await
            .map_err(|e| anyhow!("search task failed: {e}"))?
    }

    async fn resolve_stream(&self, bare_id: &str) -> Result<String> {
        let id = bare_id.to_string();
        tokio::task::spawn_blocking(move || Self::resolve_blocking(&id))
            .await
            .map_err(|e| anyhow!("resolve task failed: {e}"))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Resolution goes through the API endpoint, not the page path. This runs
    /// the exact function playback runs, against a track id from a real search.
    /// Marked `--ignored`: it needs the network, and `cargo test` on a plane
    /// should not fail for it. Run with `cargo test -- --ignored soundcloud`.
    #[test]
    #[ignore]
    fn resolves_a_real_track_id_to_a_playable_url() {
        if YTDLP.is_none() {
            eprintln!("yt-dlp not installed; skipping");
            return;
        }
        let url = SoundCloudSource::resolve_blocking("293").expect("resolve failed");
        assert!(url.starts_with("http"), "not a URL: {url}");
        // The page-path guess (soundcloud.com/tracks/<id>) is a 404; the fix
        // must keep pointing at the API or the player gets nothing to open.
        assert!(!url.contains("soundcloud.com/tracks/"), "page path leaked: {url}");
        eprintln!("resolved: {url}");
    }
}
