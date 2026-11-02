//! Typed errors for the InnerTube layer.
//!
//! Everything that talks to YouTube Music, reads its player JavaScript, or
//! decodes its responses lives under `innertube`, and it all used to collapse
//! into `anyhow::Error`. A caller could print the message but never tell a
//! network timeout from a missing cipher - which is the difference between
//! retrying and giving up.
//!
//! The kind matters down here, so it is carried in the type instead of
//! being flattened into a string.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum InnerTubeError {
    // --- yt-dlp ---------------------------------------------------------
    /// yt-dlp is what resolves a stream, and it is not installed.
    #[error("yt-dlp binary not found")]
    YtDlpMissing,

    /// yt-dlp exists but could not be started.
    #[error("yt-dlp failed to start: {0}")]
    YtDlpSpawn(#[source] std::io::Error),

    /// yt-dlp ran and reported a failure of its own.
    #[error("yt-dlp failed for {video_id}: {message}")]
    YtDlpFailed { video_id: String, message: String },

    /// yt-dlp answered with something that is not JSON.
    #[error("yt-dlp returned invalid JSON: {0}")]
    YtDlpJson(#[source] serde_json::Error),

    /// yt-dlp answered, but no format carried an audio stream.
    #[error("no usable audio stream for {0}")]
    NoUsableStream(String),

    // --- HTTP -----------------------------------------------------------
    /// A request could not be built, sent, or decoded.
    #[error("network request failed: {0}")]
    Request(#[from] reqwest::Error),

    /// The HTTP client itself could not be constructed, which is a local
    /// problem rather than a network one.
    #[error("could not build the HTTP client: {0}")]
    HttpClient(#[source] reqwest::Error),

    // --- live configuration ---------------------------------------------
    /// The live web client page was missing a value the request needs.
    #[error("the web client did not carry {0}")]
    MissingConfigValue(&'static str),

    // --- search ----------------------------------------------------------
    /// Every identity in the pool was tried and none could answer.
    #[error("no InnerTube client could answer the search")]
    NoClientAnswered,

    /// The service answered, but nothing usable came back.
    #[error("the search returned nothing")]
    EmptySearch,

    /// The service answered with a failure status.
    #[error("the search failed with status {0}")]
    SearchStatus(reqwest::StatusCode),

    // --- playback ---------------------------------------------------------
    /// No audio stream could be extracted for a video.
    #[error("failed to extract an audio stream for video {0}")]
    NoAudioStream(String),

    // --- signature cipher ---------------------------------------------------
    /// The cipher string carried no URL to attach a signature to.
    #[error("missing url in cipher string")]
    MissingCipherUrl,

    /// The URL in the cipher string did not parse.
    #[error("invalid url in cipher string: {0}")]
    CipherUrl(#[from] url::ParseError),

    /// A pattern used to read the player JavaScript did not compile.
    #[error("invalid cipher pattern: {0}")]
    CipherPattern(#[from] regex::Error),

    /// player.js did not expose a decipher function in any known shape.
    #[error("could not find the decipher function in player.js")]
    NoDecipherFunction,

    /// The decipher function was found but no operations were read from it.
    #[error("no cipher operations extracted from player.js")]
    NoCipherOperations,

    // --- lyrics ------------------------------------------------------------
    /// Neither the lookup nor the search endpoint had lyrics for the track.
    #[error("no lyrics found for track")]
    NoLyrics,
}

impl InnerTubeError {
    /// Whether trying the same request again could plausibly succeed.
    ///
    /// A transport failure and a rate limit are worth another attempt; a
    /// missing cipher or a missing configuration value are not.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Request(e) => e.is_timeout() || e.is_connect() || e.is_request(),
            Self::SearchStatus(s) => {
                s.is_server_error() || *s == reqwest::StatusCode::TOO_MANY_REQUESTS
            }
            Self::NoClientAnswered => true,
            Self::YtDlpSpawn(_) => false,
            Self::YtDlpFailed { .. } => true,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error as _;

    #[test]
    fn display_is_a_sentence_not_a_debug_dump() {
        assert_eq!(
            InnerTubeError::YtDlpMissing.to_string(),
            "yt-dlp binary not found"
        );
        assert_eq!(
            InnerTubeError::NoLyrics.to_string(),
            "no lyrics found for track"
        );
        assert_eq!(
            InnerTubeError::MissingConfigValue("an API key").to_string(),
            "the web client did not carry an API key"
        );
    }

    #[test]
    fn a_named_variant_reads_as_prose() {
        let err = InnerTubeError::YtDlpFailed {
            video_id: "abc123".into(),
            message: "video unavailable".into(),
        };
        assert_eq!(
            err.to_string(),
            "yt-dlp failed for abc123: video unavailable"
        );
    }

    #[test]
    fn a_status_carries_the_code() {
        let err = InnerTubeError::SearchStatus(reqwest::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            err.to_string(),
            "the search failed with status 429 Too Many Requests"
        );
        assert!(err.is_retryable());
    }

    #[test]
    fn retryability_separates_transport_from_structure() {
        // A 5xx is worth retrying.
        assert!(InnerTubeError::SearchStatus(reqwest::StatusCode::BAD_GATEWAY).is_retryable());
        // Losing every client identity is worth retrying.
        assert!(InnerTubeError::NoClientAnswered.is_retryable());
        // A missing cipher never becomes retryable by trying again.
        assert!(!InnerTubeError::MissingCipherUrl.is_retryable());
        // A missing configuration value is a local fact, not a flake.
        assert!(!InnerTubeError::MissingConfigValue("an API key").is_retryable());
    }

    #[test]
    fn a_cipher_failure_keeps_its_source() {
        let bad = url::Url::parse("not a url").unwrap_err();
        let err = InnerTubeError::CipherUrl(bad);
        assert!(err.to_string().starts_with("invalid url in cipher string:"));
        assert!(err.source().is_some());
    }

    #[test]
    fn it_is_a_real_error() {
        fn assert_error<E: std::error::Error + Send + Sync + 'static>(_: &E) {}
        assert_error(&InnerTubeError::NoLyrics);
    }
}
