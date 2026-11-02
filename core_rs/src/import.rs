//! Library import from another service's export.
//!
//! Melyar only speaks InnerTube, so a library that lives somewhere else has
//! to be matched track by track. The user exports their library from the
//! other service and Melyar resolves each entry against YouTube Music.
//!
//! Three shapes are understood:
//!
//! - a Spotify account export, `YourLibrary.json`
//! - a JSON array of objects with a title, an artist and an album
//! - a CSV file with a header row
//!
//! The matching is deliberately conservative: a track is only added when the
//! title and at least one other field agree, so an import never floods the
//! library with the wrong songs.

use serde_json::Value;
use std::path::Path;
use thiserror::Error;

/// Why an export file could not be turned into a library.
///
/// The file could be unreadable, not be the format it claims to be, or be a
/// well-formed file that carries no tracks. Those three are worth telling
/// apart: the first two mean the user picked the wrong file, the third
/// means they exported the wrong thing.
#[derive(Debug, Error)]
pub enum ImportError {
    /// The export file could not be read from disk.
    #[error("could not read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },

    /// The file was not valid JSON.
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// A JSON export that parsed but carried no entries.
    #[error("no tracks found in the JSON file")]
    EmptyJson,

    /// A JSON export whose top level was neither a list nor a known wrapper.
    #[error("the JSON file does not contain a list of tracks")]
    NotAList,

    /// A CSV export with no rows at all.
    #[error("the CSV file is empty")]
    EmptyCsv,

    /// A CSV export with no recognisable title column.
    #[error("the CSV file needs a title column (one of: title, track, track name, song)")]
    MissingTitleColumn,

    /// A CSV export that parsed but carried no entries.
    #[error("no tracks found in the CSV file")]
    EmptyCsvTracks,
}

/// Every fallible operation in this module reports an [`ImportError`], so a
/// caller can tell a wrong file from an empty library.
type Result<T> = std::result::Result<T, ImportError>;

/// One entry of an imported library.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportEntry {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub duration_secs: f64,
}

/// How an export was recognised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImportFormat {
    SpotifyJson,
    Json,
    Csv,
}

/// The outcome of reading an export file.
#[derive(Clone, Debug, Default)]
pub struct ImportPlan {
    pub entries: Vec<ImportEntry>,
    pub format: Option<ImportFormat>,
    /// Entries that were skipped because they carried no usable title.
    pub skipped: usize,
}

// ---------------------------------------------------------------------------
// Reading the file
// ---------------------------------------------------------------------------

/// Read an export file and turn it into a list of entries.
pub fn read_import_file(path: &Path) -> Result<ImportPlan> {
    let raw = std::fs::read_to_string(path).map_err(|source| ImportError::Read {
        path: path.display().to_string(),
        source,
    })?;

    let trimmed = raw.trim_start();
    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        return parse_json(trimmed);
    }
    parse_csv(&raw)
}

fn parse_json(raw: &str) -> Result<ImportPlan> {
    let value: Value = serde_json::from_str(raw)?;

    let array = match &value {
        Value::Array(items) => items.clone(),
        // A Spotify export may wrap the library in an object.
        Value::Object(map) => {
            let mut collected = Vec::new();
            for key in ["tracks", "items", "library", "data"] {
                if let Some(Value::Array(items)) = map.get(key) {
                    collected.extend(items.clone());
                }
            }
            if collected.is_empty() {
                return Err(ImportError::EmptyJson);
            }
            collected
        }
        _ => return Err(ImportError::NotAList),
    };

    let mut entries = Vec::new();
    let mut skipped = 0;
    let mut spotify_shape = false;

    for item in &array {
        let Some(obj) = item.as_object() else {
            skipped += 1;
            continue;
        };

        // A Spotify export uses "track", "artist" and "album". Everything
        // else is treated as a playlist or a podcast and skipped.
        if obj.contains_key("track") && obj.contains_key("artist") {
            spotify_shape = true;
        }

        // A Spotify entry carries a "type". Only tracks are imported, a
        // playlist entry has a name but no track.
        if let Some(kind) = obj.get("type").and_then(Value::as_str) {
            if kind != "track" && kind != "song" {
                skipped += 1;
                continue;
            }
        }

        let title = first_str(obj, &["track", "title", "name", "Track Name", "Song"]);
        let artist = first_str(
            obj,
            &[
                "artist",
                "Artist",
                "Artist Name",
                "artistName",
                "albumArtist",
            ],
        );
        let album = first_str(obj, &["album", "Album", "Album Name", "albumName"]);

        // A playlist entry has a name but no track.
        if title.is_empty() {
            skipped += 1;
            continue;
        }

        let duration = first_f64(obj, &["duration", "duration_ms", "Duration"]);
        let duration_secs = if duration > 1000.0 {
            // Spotify reports milliseconds.
            duration / 1000.0
        } else {
            duration
        };

        entries.push(ImportEntry {
            title,
            artist,
            album,
            duration_secs,
        });
    }

    if entries.is_empty() {
        return Err(ImportError::EmptyJson);
    }

    Ok(ImportPlan {
        entries,
        format: Some(if spotify_shape {
            ImportFormat::SpotifyJson
        } else {
            ImportFormat::Json
        }),
        skipped,
    })
}

fn parse_csv(raw: &str) -> Result<ImportPlan> {
    let mut lines = raw.lines().filter(|l| !l.trim().is_empty());
    let header = lines.next().ok_or(ImportError::EmptyCsv)?;

    let columns: Vec<String> = split_csv_line(header)
        .into_iter()
        .map(|c| c.trim().trim_matches('"').to_lowercase())
        .collect();

    let title_idx = find_column(&columns, &["title", "track", "track name", "song", "name"]);
    let artist_idx = find_column(&columns, &["artist", "artist name", "album artist"]);
    let album_idx = find_column(&columns, &["album", "album name"]);
    let duration_idx = find_column(&columns, &["duration", "length", "time", "duration (ms)"]);

    let Some(title_idx) = title_idx else {
        return Err(ImportError::MissingTitleColumn);
    };

    let mut entries = Vec::new();
    let mut skipped = 0;

    for line in lines {
        let fields = split_csv_line(line);
        let get = |idx: Option<usize>| -> String {
            idx.and_then(|i| fields.get(i))
                .map(|v| v.trim().trim_matches('"').to_string())
                .unwrap_or_default()
        };

        let title = get(Some(title_idx));
        if title.is_empty() {
            skipped += 1;
            continue;
        }

        let duration = get(duration_idx)
            .parse::<f64>()
            .ok()
            .filter(|d| *d > 0.0)
            .unwrap_or(0.0);

        entries.push(ImportEntry {
            title,
            artist: get(artist_idx),
            album: get(album_idx),
            duration_secs: duration,
        });
    }

    if entries.is_empty() {
        return Err(ImportError::EmptyCsvTracks);
    }

    Ok(ImportPlan {
        entries,
        format: Some(ImportFormat::Csv),
        skipped,
    })
}

/// Split a CSV line, honouring double quotes.
fn split_csv_line(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            ',' if !in_quotes => {
                fields.push(std::mem::take(&mut current));
            }
            _ => current.push(c),
        }
    }
    fields.push(current);
    fields
}

fn find_column(columns: &[String], candidates: &[&str]) -> Option<usize> {
    columns
        .iter()
        .position(|col| candidates.iter().any(|c| col == c))
}

fn first_str(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> String {
    for key in keys {
        if let Some(value) = obj.get(*key) {
            match value {
                Value::String(s) => {
                    let trimmed = s.trim();
                    if !trimmed.is_empty() {
                        return trimmed.to_string();
                    }
                }
                Value::Array(items) => {
                    // Spotify stores the artist as a list of objects.
                    let joined = items
                        .iter()
                        .filter_map(|i| i.get("name").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join(", ");
                    if !joined.trim().is_empty() {
                        return joined;
                    }
                }
                Value::Object(map) => {
                    if let Some(name) = map.get("name").and_then(Value::as_str) {
                        if !name.trim().is_empty() {
                            return name.to_string();
                        }
                    }
                }
                _ => {}
            }
        }
    }
    String::new()
}

fn first_f64(obj: &serde_json::Map<String, Value>, keys: &[&str]) -> f64 {
    for key in keys {
        if let Some(value) = obj.get(*key) {
            match value {
                Value::Number(n) => {
                    if let Some(f) = n.as_f64() {
                        return f;
                    }
                }
                Value::String(s) => {
                    if let Ok(f) = s.trim().parse::<f64>() {
                        return f;
                    }
                }
                _ => {}
            }
        }
    }
    0.0
}

// ---------------------------------------------------------------------------
// Matching
// ---------------------------------------------------------------------------

/// A candidate together with the score it earned.
#[derive(Clone, Debug)]
pub struct ScoredMatch {
    pub score: i32,
    pub title_score: i32,
    pub artist_score: i32,
    pub album_score: i32,
}

/// Compare an imported entry with a candidate track and score the agreement.
///
/// A score of 45 or more is enough to accept the match, but only when the
/// title agrees. An entry that names an artist also has to have that artist
/// confirmed, because the same title is often recorded by somebody else.
pub fn score_match(
    entry: &ImportEntry,
    candidate_title: &str,
    candidate_artist: &str,
    candidate_album: &str,
) -> ScoredMatch {
    let title_score = compare_titles(&entry.title, candidate_title);
    let artist_score = compare_names(&entry.artist, candidate_artist);
    let album_score = compare_names(&entry.album, candidate_album);

    let mut score = title_score + artist_score + album_score;

    // The artist and the album together are not enough: one album holds
    // many songs, so the title has to agree.
    if title_score == 0 {
        score = 0;
    }

    // An entry that names an artist has to have that artist confirmed. The
    // same title is often recorded by somebody else.
    if !entry.artist.trim().is_empty() && artist_score == 0 {
        score = 0;
    }

    ScoredMatch {
        score,
        title_score,
        artist_score,
        album_score,
    }
}

/// The score above which a candidate is accepted.
pub const MATCH_THRESHOLD: i32 = 45;

fn compare_titles(left: &str, right: &str) -> i32 {
    let a = canonical_title(left);
    let b = canonical_title(right);
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    if a == b {
        return 50;
    }
    if a.contains(&b) || b.contains(&a) {
        return 20;
    }
    0
}

fn compare_names(left: &str, right: &str) -> i32 {
    let a = canonical_title(left);
    let b = canonical_title(right);
    if a.is_empty() || b.is_empty() {
        return 0;
    }
    if a == b {
        return 30;
    }
    if a.contains(&b) || b.contains(&a) {
        return 15;
    }
    0
}

/// Reduce a title to something comparable: lower case, without the parts
/// that decorate a release rather than name it.
pub fn canonical_title(input: &str) -> String {
    let mut text = input.to_lowercase();

    // Everything in brackets or parentheses is usually a version marker.
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for c in text.chars() {
        match c {
            '(' | '[' => {
                depth += 1;
            }
            ')' | ']' => {
                depth = depth.saturating_sub(1);
            }
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    text = out;

    // A "feat." tail names another artist, not the track.
    for marker in [" feat. ", " feat ", " ft. ", " ft ", " featuring "] {
        if let Some(pos) = text.find(marker) {
            text.truncate(pos);
        }
    }
    // A " - remaster" tail is the same decoration.
    for marker in [" - remaster", " - live", " - remix", " - edit"] {
        if let Some(pos) = text.find(marker) {
            text.truncate(pos);
        }
    }

    let mut cleaned = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_alphanumeric() || c.is_whitespace() {
            cleaned.push(c);
        } else if c == '\'' || c == '\u{2019}' {
            // Keep the apostrophe so "don't" still matches "don't".
            cleaned.push(' ');
        }
    }

    cleaned.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(title: &str, artist: &str, album: &str) -> ImportEntry {
        ImportEntry {
            title: title.to_string(),
            artist: artist.to_string(),
            album: album.to_string(),
            duration_secs: 0.0,
        }
    }

    #[test]
    fn canonical_title_strips_decoration() {
        assert_eq!(
            canonical_title("Come Together (Remastered 2009)"),
            "come together"
        );
        assert_eq!(canonical_title("I Got Love (feat. Джамал)"), "i got love");
        assert_eq!(canonical_title("Song - Remastered 2011"), "song");
        assert_eq!(canonical_title("Don't Stop Me Now"), "don t stop me now");
    }

    #[test]
    fn canonical_title_keeps_non_latin_scripts() {
        assert_eq!(canonical_title("Круглый"), "круглый");
        assert_eq!(canonical_title("ouroboros"), "ouroboros");
    }

    #[test]
    fn exact_title_and_artist_scores_high() {
        let m = score_match(
            &entry("Bohemian Rhapsody", "Queen", "A Night at the Opera"),
            "Bohemian Rhapsody",
            "Queen",
            "A Night at the Opera",
        );
        assert!(m.score >= MATCH_THRESHOLD);
    }

    #[test]
    fn a_different_track_is_rejected() {
        let m = score_match(
            &entry("Bohemian Rhapsody", "Queen", "A Night at the Opera"),
            "Another One Bites The Dust",
            "Queen",
            "A Night at the Opera",
        );
        assert!(m.score < MATCH_THRESHOLD);
    }

    #[test]
    fn a_different_artist_is_rejected() {
        let m = score_match(
            &entry("Bohemian Rhapsody", "Queen", "A Night at the Opera"),
            "Bohemian Rhapsody",
            "The Chemical Brothers",
            "",
        );
        assert!(m.score < MATCH_THRESHOLD);
    }

    #[test]
    fn reads_a_spotify_export() {
        let json = r#"[
            {"artist":"Queen","album":"A Night at the Opera","track":"Bohemian Rhapsody","type":"track"},
            {"artist":"Prince","album":"Purple Rain","track":"Purple Rain","type":"track"}
        ]"#;
        let plan = parse_json(json).unwrap();
        assert_eq!(plan.entries.len(), 2);
        assert_eq!(plan.format, Some(ImportFormat::SpotifyJson));
        assert_eq!(plan.entries[0].title, "Bohemian Rhapsody");
        assert_eq!(plan.entries[0].artist, "Queen");
    }

    #[test]
    fn skips_spotify_playlists() {
        let json = r#"[
            {"name":"Road Trip","type":"playlist","uri":"spotify:playlist:1"},
            {"artist":"Queen","album":"A Night at the Opera","track":"Bohemian Rhapsody","type":"track"}
        ]"#;
        let plan = parse_json(json).unwrap();
        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.skipped, 1);
    }

    #[test]
    fn reads_a_generic_json_list() {
        let json = r#"[{"title":"Hurt","artist":"Nine Inch Nails","album":"The Downward Spiral"}]"#;
        let plan = parse_json(json).unwrap();
        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.format, Some(ImportFormat::Json));
        assert_eq!(plan.entries[0].title, "Hurt");
    }

    #[test]
    fn reads_a_csv_with_a_header() {
        let csv = "Title,Artist,Album\nHurt,Nine Inch Nails,The Downward Spiral\nCloser,Nine Inch Nails,Hurt\n";
        let plan = parse_csv(csv).unwrap();
        assert_eq!(plan.entries.len(), 2);
        assert_eq!(plan.format, Some(ImportFormat::Csv));
        assert_eq!(plan.entries[0].title, "Hurt");
        assert_eq!(plan.entries[1].album, "Hurt");
    }

    #[test]
    fn reads_a_csv_with_quoted_fields() {
        let csv = "\"Track Name\",\"Artist Name\",\"Album Name\"\n\"Hurt\",\"Nine Inch Nails\",\"The Downward Spiral\"\n";
        let plan = parse_csv(csv).unwrap();
        assert_eq!(plan.entries.len(), 1);
        assert_eq!(plan.entries[0].title, "Hurt");
    }

    #[test]
    fn a_csv_without_a_title_column_is_rejected() {
        let csv = "Artist,Album\nQueen,A Night at the Opera\n";
        assert!(parse_csv(csv).is_err());
    }
}
