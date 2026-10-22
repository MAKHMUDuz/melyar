//! A thin seam between melyar and the places music comes from.
//!
//! melyar plays whatever URL libmpv is handed, so the only thing that ever
//! knew about YouTube was the resolution layer. This module lifts that into a
//! `Source`: something that can search and turn a track id into a stream URL.
//!
//! Two rules keep it cheap:
//!  - YouTube keeps bare video ids. Every favourite, history row and queued
//!    track that existed before this module still resolves, because an id with
//!    no recognised prefix is YouTube by definition.
//!  - A source is only consulted when it is asked for. Enabling nothing but
//!    YouTube leaves the search path with one participant, exactly as before.

pub mod audius;
pub mod soundcloud;
pub mod youtube;

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;

use crate::ffi::CxxTrackMetadata;

/// What a source is made of. The UI uses it to say why a source is slow or
/// where its data comes from, never to decide behaviour.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    /// Talks a site's own HTTP API directly.
    NativeRest,
    /// Shells out to yt-dlp, which is why it is opt-in: a process spawn per
    /// query is a real cost and the user should choose to pay it.
    YtDlp,
}

/// One place music can come from.
///
/// `bare_id` never carries the prefix; the registry strips it before calling
/// in, so an implementation only ever sees the id its own site produced.
#[async_trait]
pub trait Source: Send + Sync {
    /// Stable key, also the id prefix: `soundcloud` → `soundcloud:<id>`.
    fn id(&self) -> &'static str;
    fn display_name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn kind(&self) -> SourceKind;
    /// On by default. Only YouTube says yes; the rest cost something.
    fn default_enabled(&self) -> bool {
        false
    }

    async fn search(&self, query: &str) -> Result<Vec<CxxTrackMetadata>>;
    /// Resolve `bare_id` to a URL libmpv can open.
    async fn resolve_stream(&self, bare_id: &str) -> Result<String>;
}

/// What the plugins panel renders, one object per source.
pub struct SourceInfo {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub kind: &'static str,
    pub enabled: bool,
    pub default_enabled: bool,
}

/// Holds every source and which of them the user turned on.
///
/// The enabled set is the only mutable state and it lives behind a plain mutex:
/// it is read on every search and written once per toggle, so there is nothing
/// to gain from finer locking.
pub struct SourceRegistry {
    sources: Vec<Arc<dyn Source>>,
    /// The source an unprefixed id belongs to. Always YouTube.
    default: Arc<dyn Source>,
    enabled: Mutex<Vec<String>>,
}

impl SourceRegistry {
    /// Build the registry with YouTube on and everything else remembering
    /// whether the user asked for it. `saved` is the comma list from settings.
    ///
    /// YouTube gets the live client rather than building its own, so the default
    /// path shares the connection pool the app already warmed up.
    pub fn new(innertube: std::sync::Arc<crate::innertube::client::InnerTubeClient>, saved: &str) -> Self {
        let youtube: Arc<dyn Source> = Arc::new(youtube::YouTubeSource::new(innertube));
        let soundcloud: Arc<dyn Source> = Arc::new(soundcloud::SoundCloudSource::new());
        let audius: Arc<dyn Source> = Arc::new(audius::AudiusSource::new());

        let sources: Vec<Arc<dyn Source>> = vec![youtube.clone(), soundcloud, audius];
        let enabled = enabled_ids(&sources, saved);

        Self {
            sources,
            default: youtube,
            enabled: Mutex::new(enabled),
        }
    }

    /// The list the plugins panel shows, in a stable order with YouTube first.
    pub fn info(&self) -> Vec<SourceInfo> {
        let enabled = self.enabled.lock();
        self.sources
            .iter()
            .map(|s| SourceInfo {
                id: s.id().to_string(),
                display_name: s.display_name().to_string(),
                description: s.description().to_string(),
                kind: match s.kind() {
                    SourceKind::NativeRest => "api",
                    SourceKind::YtDlp => "yt-dlp",
                },
                enabled: enabled.iter().any(|e| e == s.id()),
                default_enabled: s.default_enabled(),
            })
            .collect()
    }

    pub fn set_enabled(&self, id: &str, on: bool) -> bool {
        let mut enabled = self.enabled.lock();
        let known = self.sources.iter().any(|s| s.id() == id);
        if !known {
            return false;
        }
        // YouTube is the default and the anchor for every bare id; it cannot be
        // turned off, only ignored by not searching it — which nothing does.
        if id == self.default.id() {
            return false;
        }
        if on && !enabled.iter().any(|e| e == id) {
            enabled.push(id.to_string());
        } else if !on {
            enabled.retain(|e| e != id);
        }
        true
    }

    /// The comma list to persist. Callers store it under `enabledSources`.
    pub fn enabled_serialized(&self) -> String {
        self.enabled.lock().join(",")
    }

    /// Search every enabled source at once.
    ///
    /// They run concurrently, so the wait is the slowest source rather than the
    /// sum. With only YouTube on — the default — this is one future, the same
    /// call the search box used to make.
    pub async fn search_all(&self, query: &str) -> Vec<CxxTrackMetadata> {
        let wanted: Vec<Arc<dyn Source>> = {
            let enabled = self.enabled.lock();
            self.sources
                .iter()
                .filter(|s| enabled.iter().any(|e| e == s.id()))
                .cloned()
                .collect()
        };

        let futures = wanted.iter().map(|s| s.search(query));
        let results = futures::future::join_all(futures).await;

        let mut merged = Vec::new();
        for res in results {
            if let Ok(mut tracks) = res {
                merged.append(&mut tracks);
            }
        }
        merged
    }

    /// Which source owns `full_id`, and the id without its prefix.
    ///
    /// An id with no recognised prefix belongs to YouTube — that is what keeps
    /// the pre-existing library working untouched.
    pub fn source_for_id(&self, full_id: &str) -> (Arc<dyn Source>, String) {
        if let Some((prefix, rest)) = full_id.split_once(':') {
            if let Some(s) = self.sources.iter().find(|s| s.id() == prefix) {
                return (s.clone(), rest.to_string());
            }
        }
        (self.default.clone(), full_id.to_string())
    }

    /// Resolve a full id to a playable URL, whatever source it came from.
    pub async fn resolve_stream(&self, full_id: &str) -> Result<String> {
        let (source, bare) = self.source_for_id(full_id);
        source.resolve_stream(&bare).await
    }
}

/// Work out the starting enabled set from the saved preference.
///
/// YouTube is always in. Anything the user saved that still names a real
/// source is restored; a source that no longer exists is dropped rather than
/// carried as a setting that does nothing.
fn enabled_ids(sources: &[Arc<dyn Source>], saved: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in sources.iter().filter(|s| s.default_enabled()) {
        out.push(s.id().to_string());
    }
    for part in saved.split(',') {
        let part = part.trim();
        if part.is_empty() || out.iter().any(|e| e == part) {
            continue;
        }
        if sources.iter().any(|s| s.id() == part) {
            out.push(part.to_string());
        }
    }
    out
}
