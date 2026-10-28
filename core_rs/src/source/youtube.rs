//! YouTube, the default and the only source that is on out of the box.
//!
//! This is not a new client — it is the existing `InnerTubeClient` wearing the
//! `Source` hat, so the path a bare video id takes is byte-for-byte what it was
//! before sources existed.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::ffi::CxxTrackMetadata;
use crate::innertube::client::InnerTubeClient;

use super::{Source, SourceKind};

pub struct YouTubeSource {
    innertube: Arc<InnerTubeClient>,
}

impl YouTubeSource {
    pub fn new(innertube: Arc<InnerTubeClient>) -> Self {
        Self { innertube }
    }
}

#[async_trait]
impl Source for YouTubeSource {
    fn id(&self) -> &'static str {
        "youtube"
    }

    fn display_name(&self) -> &'static str {
        "YouTube"
    }

    fn description(&self) -> &'static str {
        "Built in. Searches and streams from YouTube."
    }

    fn kind(&self) -> SourceKind {
        SourceKind::NativeRest
    }

    fn default_enabled(&self) -> bool {
        true
    }

    async fn search(&self, query: &str) -> Result<Vec<CxxTrackMetadata>> {
        self.innertube.search_tracks(query).await
    }

    async fn resolve_stream(&self, bare_id: &str) -> Result<String> {
        // A bare id with no prefix lands here, exactly as play_track used to
        // call it. The pool and yt-dlp fallback live inside.
        self.innertube.get_stream_url(bare_id).await
    }
}
