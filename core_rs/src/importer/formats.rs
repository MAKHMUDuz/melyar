//! Export-file parsers for the import centre.
//!
//! Every format here is parsed and tested; a file whose shape is not
//! understood is rejected with a message that says what was expected. The
//! parsers never touch the source service and never rewrite the input.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde_json::Value;
use url::Url;

use super::metadata;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParsedTrack {
    pub source: String,
    pub source_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub duration_secs: f64,
    pub track_no: i32,
    pub disc_no: i32,
    pub year: String,
    pub isrc: String,
    pub external_url: String,
    pub thumbnail: String,
    /// Set when the export points at a file that exists on this machine.
    pub file_path: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParsedPlaylist {
    pub name: String,
    /// Source ids, in the order the export lists them.
    pub track_ids: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ParsedExport {
    pub provider: String,
    pub format: String,
    pub tracks: Vec<ParsedTrack>,
    pub playlists: Vec<ParsedPlaylist>,
    pub skipped: usize,
    pub warnings: Vec<String>,
}

impl ParsedExport {
    pub fn all_track_count(&self) -> usize {
        self.tracks.len()
            + self
                .playlists
                .iter()
                .map(|p| p.track_ids.len())
                .sum::<usize>()
    }
}

pub fn parse_file(path: &Path) -> Result<ParsedExport> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();

    match ext.as_str() {
        "m3u" | "m3u8" => parse_m3u(path),
        "pls" => parse_pls(path),
        "xspf" => parse_xspf(path),
        "xml" => parse_xml_file(path),
        "json" => parse_json_file(path),
        "csv" => parse_csv_file(path),
        "zip" => Err(anyhow!(
            "archive imports are not supported yet; extract the archive and choose the JSON or CSV file inside"
        )),
        _ => Err(anyhow!(
            "unsupported import file type .{ext}; supported: json, csv, m3u, m3u8, pls, xspf, xml"
        )),
    }
}

// ---------------------------------------------------------------------------
// M3U / M3U8
// ---------------------------------------------------------------------------

fn parse_m3u(path: &Path) -> Result<ParsedExport> {
    let raw = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&raw);
    let base = path.parent().unwrap_or(Path::new("."));
    let mut tracks = Vec::new();
    let mut pending: Option<(String, String, f64)> = None; // artist, title, duration

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            if let Some(rest) = line.strip_prefix("#EXTINF:") {
                let (duration, label) = rest.split_once(',').unwrap_or((rest, ""));
                let duration = duration.trim().parse::<f64>().unwrap_or(0.0);
                let (artist, title) = label
                    .split_once(" - ")
                    .map(|(a, t)| (a.trim().to_string(), t.trim().to_string()))
                    .unwrap_or_else(|| (String::new(), label.trim().to_string()));
                pending = Some((artist, title, duration));
            }
            continue;
        }

        let resolved = resolve_local_path(base, line);
        let (mut artist, mut title, mut duration) = pending
            .take()
            .unwrap_or_else(|| (String::new(), String::new(), 0.0));
        let mut album = String::new();
        let mut track_no = 0;
        let mut file_path = String::new();
        let mut source_id;

        if let Some(local) = resolved.as_ref().filter(|p| p.is_file()) {
            file_path = local.to_string_lossy().to_string();
            if metadata::is_audio_file(local) {
                if let Ok(meta) = metadata::read_metadata(local) {
                    if title.is_empty() {
                        title = meta.title;
                    }
                    if artist.is_empty() {
                        artist = meta.artist;
                    }
                    album = meta.album;
                    track_no = meta.track_no;
                    if duration <= 0.0 {
                        duration = meta.duration_secs;
                    }
                }
            }
            source_id = format!("path:{}", file_path);
        } else {
            source_id = format!(
                "text:{}|{}|{}",
                title.to_lowercase(),
                artist.to_lowercase(),
                album.to_lowercase()
            );
        }

        if title.is_empty() {
            title = Path::new(line)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Untitled")
                .to_string();
        }
        if artist.is_empty() {
            artist = "Unknown artist".to_string();
        }
        if album.is_empty() {
            album = "Unknown album".to_string();
        }
        if source_id.is_empty() {
            source_id = format!("text:{title}");
        }

        tracks.push(ParsedTrack {
            source: "file".into(),
            source_id,
            title,
            artist,
            album,
            duration_secs: duration,
            track_no,
            file_path,
            ..Default::default()
        });
    }

    if tracks.is_empty() {
        return Err(anyhow!("the playlist does not contain any tracks"));
    }
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported playlist")
        .to_string();
    let track_ids = tracks.iter().map(|t| t.source_id.clone()).collect();
    Ok(ParsedExport {
        provider: "file".into(),
        format: "m3u".into(),
        tracks,
        playlists: vec![ParsedPlaylist { name, track_ids }],
        skipped: 0,
        warnings: Vec::new(),
    })
}

fn resolve_local_path(base: &Path, entry: &str) -> Option<PathBuf> {
    if let Ok(url) = Url::parse(entry) {
        if url.scheme() == "file" {
            return url.to_file_path().ok();
        }
    }
    let candidate = PathBuf::from(entry);
    if candidate.is_absolute() {
        Some(candidate)
    } else {
        Some(base.join(candidate))
    }
}

// ---------------------------------------------------------------------------
// PLS
// ---------------------------------------------------------------------------

fn parse_pls(path: &Path) -> Result<ParsedExport> {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let mut entries: std::collections::BTreeMap<usize, ParsedTrack> = Default::default();
    let base = path.parent().unwrap_or(Path::new("."));

    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        let lower = key.to_lowercase();
        let index: Option<usize> = lower
            .strip_prefix("file")
            .and_then(|n| n.trim().parse().ok());
        if let Some(index) = index {
            let entry = entries.entry(index).or_default();
            entry.source = "file".into();
            entry.source_id = format!("path:{value}");
            let resolved = resolve_local_path(base, value);
            if let Some(local) = resolved.filter(|p| p.is_file()) {
                entry.file_path = local.to_string_lossy().to_string();
                if let Ok(meta) = metadata::read_metadata(&local) {
                    entry.title = meta.title;
                    entry.artist = meta.artist;
                    entry.album = meta.album;
                    entry.duration_secs = meta.duration_secs;
                    entry.track_no = meta.track_no;
                }
            } else {
                entry.title = Path::new(value)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string();
                entry.artist = "Unknown artist".into();
                entry.album = "Unknown album".into();
            }
            continue;
        }
        for (prefix, field) in [
            ("title", "title"),
            ("length", "duration"),
            ("artist", "artist"),
            ("album", "album"),
        ] {
            if let Some(n) = lower.strip_prefix(prefix) {
                if let Ok(i) = n.trim().parse::<usize>() {
                    let entry = entries.entry(i).or_default();
                    if field == "title" {
                        entry.title = value.to_string();
                    } else if field == "duration" {
                        entry.duration_secs = value.parse().unwrap_or(0.0);
                    } else if field == "artist" {
                        entry.artist = value.to_string();
                    } else {
                        entry.album = value.to_string();
                    }
                }
            }
        }
    }

    let tracks: Vec<ParsedTrack> = entries
        .into_values()
        .filter(|t| !t.title.is_empty() || !t.file_path.is_empty())
        .map(|mut t| {
            if t.title.is_empty() {
                t.title = Path::new(&t.file_path)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Untitled")
                    .to_string();
            }
            if t.artist.is_empty() {
                t.artist = "Unknown artist".into();
            }
            if t.album.is_empty() {
                t.album = "Unknown album".into();
            }
            t
        })
        .collect();
    if tracks.is_empty() {
        return Err(anyhow!("the PLS playlist does not contain any tracks"));
    }
    let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported playlist")
        .to_string();
    let track_ids = tracks.iter().map(|t| t.source_id.clone()).collect();
    Ok(ParsedExport {
        provider: "file".into(),
        format: "pls".into(),
        tracks,
        playlists: vec![ParsedPlaylist { name, track_ids }],
        skipped: 0,
        warnings: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// XSPF
// ---------------------------------------------------------------------------

fn parse_xspf(path: &Path) -> Result<ParsedExport> {
    let text = std::fs::read_to_string(path)?;
    let base = path.parent().unwrap_or(Path::new("."));
    let mut tracks = Vec::new();
    let mut playlist_name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Imported playlist")
        .to_string();
    let mut top_title: Option<String> = None;
    let mut current: Option<ParsedTrack> = None;
    let mut duration_ms = 0.0;
    let mut field_stack: Vec<String> = Vec::new();

    for event in parse_xml(&text) {
        match event {
            XmlEvent::Start { name, .. } => {
                if name == "track" {
                    current = Some(ParsedTrack {
                        source: "file".into(),
                        ..Default::default()
                    });
                    duration_ms = 0.0;
                }
                field_stack.push(name);
            }
            XmlEvent::Text(text) => {
                let text = text.trim();
                if text.is_empty() {
                    continue;
                }
                let field = field_stack.last().map(String::as_str).unwrap_or("");
                if let Some(track) = current.as_mut() {
                    match field {
                        "location" => {
                            let resolved = resolve_local_path(base, text);
                            if let Some(local) = resolved.filter(|p| p.is_file()) {
                                track.file_path = local.to_string_lossy().to_string();
                                track.source_id = format!("path:{}", track.file_path);
                                if let Ok(meta) = metadata::read_metadata(&local) {
                                    track.title = meta.title;
                                    track.artist = meta.artist;
                                    track.album = meta.album;
                                    track.duration_secs = meta.duration_secs;
                                }
                            } else {
                                track.source_id = format!("url:{text}");
                                track.external_url = text.to_string();
                            }
                        }
                        "title" => {
                            if track.title.is_empty() {
                                track.title = text.to_string();
                            }
                        }
                        "creator" => track.artist = text.to_string(),
                        "album" => track.album = text.to_string(),
                        "duration" => duration_ms = text.parse().unwrap_or(0.0),
                        "identifier" => {
                            if track.source_id.is_empty() {
                                track.source_id = text.to_string();
                            }
                        }
                        _ => {}
                    }
                } else if field == "title" && top_title.is_none() {
                    top_title = Some(text.to_string());
                }
            }
            XmlEvent::End { name } => {
                field_stack.pop();
                if name == "track" {
                    if let Some(mut track) = current.take() {
                        if track.title.is_empty() {
                            track.title = Path::new(&track.file_path)
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Untitled")
                                .to_string();
                        }
                        if track.artist.is_empty() {
                            track.artist = "Unknown artist".into();
                        }
                        if track.album.is_empty() {
                            track.album = "Unknown album".into();
                        }
                        if track.source_id.is_empty() {
                            track.source_id = format!(
                                "text:{}|{}|{}",
                                track.title.to_lowercase(),
                                track.artist.to_lowercase(),
                                track.album.to_lowercase()
                            );
                        }
                        if track.duration_secs <= 0.0 && duration_ms > 0.0 {
                            track.duration_secs = duration_ms / 1000.0;
                        }
                        tracks.push(track);
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(title) = top_title.filter(|t| !t.trim().is_empty()) {
        playlist_name = title;
    }
    if tracks.is_empty() {
        return Err(anyhow!("the XSPF playlist does not contain any tracks"));
    }
    let track_ids = tracks.iter().map(|t| t.source_id.clone()).collect();
    Ok(ParsedExport {
        provider: "file".into(),
        format: "xspf".into(),
        tracks,
        playlists: vec![ParsedPlaylist {
            name: playlist_name,
            track_ids,
        }],
        skipped: 0,
        warnings: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// JSON
// ---------------------------------------------------------------------------

fn parse_json_file(path: &Path) -> Result<ParsedExport> {
    let raw = std::fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&raw).map_err(|e| anyhow!("invalid JSON: {e}"))?;
    let mut export = ParsedExport {
        provider: "file".into(),
        format: "json".into(),
        ..Default::default()
    };

    let mut tracks = Vec::new();
    let mut playlists = Vec::new();

    collector(&value, &mut tracks, &mut playlists, &mut export.skipped);

    if tracks.is_empty() && playlists.is_empty() {
        return Err(anyhow!("no tracks or playlists found in the JSON file"));
    }
    export.tracks = tracks;
    export.playlists = playlists;
    Ok(export)
}

fn collector(
    value: &Value,
    tracks: &mut Vec<ParsedTrack>,
    playlists: &mut Vec<ParsedPlaylist>,
    skipped: &mut usize,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                if let Some(track) = track_from_json(item) {
                    tracks.push(track);
                } else if let Some(playlist) = playlist_from_json(item) {
                    playlists.push(playlist);
                }
            }
        }
        Value::Object(map) => {
            if let Some(Value::Array(items)) = map.get("playlists") {
                for item in items {
                    if let Some(playlist) = playlist_from_json(item) {
                        playlists.push(playlist);
                    }
                }
            }
            for key in ["tracks", "items", "library", "data", "songs"] {
                if let Some(Value::Array(items)) = map.get(key) {
                    for item in items {
                        if let Some(track) = track_from_json(item) {
                            tracks.push(track);
                        } else {
                            *skipped += 1;
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn first_string<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(str::trim).filter(|s| !s.is_empty())
}

fn first_number(value: &Value, path: &[&str]) -> Option<f64> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_f64()
}

fn track_from_json(value: &Value) -> Option<ParsedTrack> {
    let obj = value.as_object()?;
    // Spotify API wraps the track in `track`, old exports use flat fields.
    let inner = obj.get("track").filter(|v| v.is_object()).unwrap_or(value);
    let title = first_string(inner, &["name"])
        .or_else(|| first_string(inner, &["title"]))
        .or_else(|| first_string(inner, &["track"]))
        .or_else(|| first_string(inner, &["Track Name"]))
        .or_else(|| first_string(inner, &["Song Title"]))?;

    let artist = first_string(inner, &["artist"])
        .map(str::to_string)
        .or_else(|| {
            inner
                .get("artists")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|a| a.get("name").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .filter(|s| !s.is_empty())
        })
        .or_else(|| first_string(inner, &["Artist Name"]).map(str::to_string))
        .unwrap_or_else(|| "Unknown artist".into());

    let album_name = inner
        .get("album")
        .and_then(|a| a.get("name"))
        .and_then(Value::as_str)
        .or_else(|| inner.get("album").and_then(Value::as_str))
        .or_else(|| first_string(inner, &["Album Name"]))
        .unwrap_or("Unknown album")
        .to_string();

    let duration_raw = first_number(inner, &["duration_ms"])
        .or_else(|| first_number(inner, &["duration"]))
        .or_else(|| first_number(inner, &["Length"]))
        .unwrap_or(0.0);
    let duration_secs = if duration_raw > 1000.0 {
        duration_raw / 1000.0
    } else {
        duration_raw
    };

    let source_id = first_string(inner, &["id"])
        .or_else(|| first_string(inner, &["uri"]))
        .map(str::to_string)
        .or_else(|| first_string(inner, &["Video ID"]).map(str::to_string))
        .unwrap_or_else(|| {
            format!(
                "text:{}|{}|{}",
                title.to_lowercase(),
                artist.to_lowercase(),
                album_name.to_lowercase()
            )
        });

    let isrc = first_string(inner, &["external_ids", "isrc"])
        .map(str::to_string)
        .unwrap_or_default();
    let external_url = first_string(inner, &["external_urls", "spotify"])
        .map(str::to_string)
        .unwrap_or_default();
    let thumbnail = first_string(inner, &["thumbnail"])
        .map(str::to_string)
        .or_else(|| {
            inner
                .get("album")
                .and_then(|a| a.get("images"))
                .and_then(Value::as_array)
                .and_then(|items| items.first())
                .and_then(|i| i.get("url"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_default();

    Some(ParsedTrack {
        source: "file".into(),
        source_id,
        title: title.to_string(),
        artist,
        album: album_name,
        album_artist: first_string(inner, &["album_artist"])
            .unwrap_or("")
            .to_string(),
        duration_secs,
        track_no: first_number(inner, &["track_number"]).unwrap_or(0.0) as i32,
        disc_no: first_number(inner, &["disc_number"]).unwrap_or(0.0) as i32,
        year: first_string(inner, &["year"]).unwrap_or("").to_string(),
        isrc,
        external_url,
        thumbnail,
        file_path: String::new(),
    })
}

fn playlist_from_json(value: &Value) -> Option<ParsedPlaylist> {
    let obj = value.as_object()?;
    let name = obj.get("name").and_then(Value::as_str)?;
    let items = obj
        .get("tracks")
        .or_else(|| obj.get("items"))
        .and_then(Value::as_array)?;
    let mut track_ids = Vec::new();
    for item in items {
        if let Some(track) = track_from_json(item) {
            track_ids.push(track.source_id);
        } else if let Some(id) = item.get("id").and_then(Value::as_str) {
            track_ids.push(id.to_string());
        }
    }
    Some(ParsedPlaylist {
        name: name.to_string(),
        track_ids,
    })
}

// ---------------------------------------------------------------------------
// CSV
// ---------------------------------------------------------------------------

fn parse_csv_file(path: &Path) -> Result<ParsedExport> {
    let raw = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&raw);
    let records = parse_csv_records(&text);
    if records.is_empty() {
        return Err(anyhow!("the CSV file is empty"));
    }
    let header: Vec<String> = records[0]
        .iter()
        .map(|h| h.trim().trim_matches('"').to_lowercase())
        .collect();
    let find = |names: &[&str]| -> Option<usize> {
        header.iter().position(|h| names.iter().any(|n| h == n))
    };
    let title_idx = find(&["title", "track", "track name", "song", "song title", "name"])
        .ok_or_else(|| anyhow!("the CSV file needs a title column (title, track, song, name)"))?;
    let artist_idx = find(&["artist", "artist name", "album artist"]);
    let album_idx = find(&["album", "album name"]);
    let duration_idx = find(&["duration", "length", "time", "duration (ms)"]);
    let isrc_idx = find(&["isrc"]);
    let url_idx = find(&["url", "external url", "spotify url"]);
    let video_idx = find(&["video id", "youtube id"]);

    let mut tracks = Vec::new();
    let mut skipped = 0;
    for record in records.iter().skip(1) {
        let get = |idx: Option<usize>| -> String {
            idx.and_then(|i| record.get(i))
                .map(|v| v.trim().trim_matches('"').to_string())
                .unwrap_or_default()
        };
        let title = get(Some(title_idx));
        if title.is_empty() {
            skipped += 1;
            continue;
        }
        let artist = get(artist_idx);
        let album = get(album_idx);
        let duration = get(duration_idx).parse::<f64>().unwrap_or(0.0);
        let duration_secs = if duration > 1000.0 {
            duration / 1000.0
        } else {
            duration
        };
        let artist = if artist.is_empty() {
            "Unknown artist".into()
        } else {
            artist
        };
        let album = if album.is_empty() {
            "Unknown album".into()
        } else {
            album
        };
        let video = get(video_idx);
        let source_id = if !video.is_empty() {
            video
        } else {
            format!(
                "text:{}|{}|{}",
                title.to_lowercase(),
                artist.to_lowercase(),
                album.to_lowercase()
            )
        };
        tracks.push(ParsedTrack {
            source: "file".into(),
            source_id,
            title,
            artist,
            album,
            duration_secs,
            isrc: get(isrc_idx),
            external_url: get(url_idx),
            ..Default::default()
        });
    }
    if tracks.is_empty() {
        return Err(anyhow!("no tracks found in the CSV file"));
    }
    Ok(ParsedExport {
        provider: "file".into(),
        format: "csv".into(),
        tracks,
        playlists: Vec::new(),
        skipped,
        warnings: Vec::new(),
    })
}

fn parse_csv_records(text: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    field.push('"');
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            '"' => in_quotes = true,
            ',' if !in_quotes => {
                record.push(std::mem::take(&mut field));
            }
            '\r' if !in_quotes => {}
            '\n' if !in_quotes => {
                record.push(std::mem::take(&mut field));
                if record.iter().any(|f| !f.trim().is_empty()) {
                    records.push(std::mem::take(&mut record));
                } else {
                    record.clear();
                }
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        records.push(record);
    }
    records
}

// ---------------------------------------------------------------------------
// XML (iTunes library + XSPF)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
enum XmlEvent {
    Start {
        name: String,
        attrs: Vec<(String, String)>,
    },
    End {
        name: String,
    },
    Text(String),
}

fn parse_xml(input: &str) -> Vec<XmlEvent> {
    let mut events = Vec::new();
    let bytes: Vec<char> = input.chars().collect();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] != '<' {
            let start = i;
            while i < bytes.len() && bytes[i] != '<' {
                i += 1;
            }
            let text: String = bytes[start..i].iter().collect();
            let text = decode_entities(&text);
            if !text.trim().is_empty() {
                events.push(XmlEvent::Text(text));
            }
            continue;
        }
        if starts_with_at(&bytes, i, "<?") {
            i = find_from(&bytes, i, "?>")
                .map(|p| p + 2)
                .unwrap_or(bytes.len());
            continue;
        }
        if starts_with_at(&bytes, i, "<!--") {
            i = find_from(&bytes, i, "-->")
                .map(|p| p + 3)
                .unwrap_or(bytes.len());
            continue;
        }
        if starts_with_at(&bytes, i, "<![CDATA[") {
            let start = i + 9;
            let end = find_from(&bytes, start, "]]>").unwrap_or(bytes.len());
            events.push(XmlEvent::Text(bytes[start..end].iter().collect()));
            i = (end + 3).min(bytes.len());
            continue;
        }
        if starts_with_at(&bytes, i, "<!") {
            i = find_from(&bytes, i, ">")
                .map(|p| p + 1)
                .unwrap_or(bytes.len());
            continue;
        }
        let end = find_from(&bytes, i, ">").unwrap_or(bytes.len());
        let raw: String = bytes[i + 1..end].iter().collect();
        i = (end + 1).min(bytes.len());
        let is_end = raw.trim_start().starts_with('/');
        let raw = raw.trim_start().trim_start_matches('/');
        let self_closing = raw.trim_end().ends_with('/');
        let raw = raw.trim_end().trim_end_matches('/').trim().to_string();
        let mut parts = raw.splitn(2, char::is_whitespace);
        let name = parts
            .next()
            .unwrap_or("")
            .split(':')
            .last()
            .unwrap_or("")
            .to_lowercase();
        let attrs = parts.next().map(parse_attrs).unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        if is_end {
            events.push(XmlEvent::End { name });
            continue;
        }
        events.push(XmlEvent::Start {
            name: name.clone(),
            attrs,
        });
        if self_closing {
            events.push(XmlEvent::End { name });
        }
    }
    events
}

fn starts_with_at(chars: &[char], at: usize, needle: &str) -> bool {
    let needle: Vec<char> = needle.chars().collect();
    at + needle.len() <= chars.len() && chars[at..at + needle.len()] == needle[..]
}

fn find_from(chars: &[char], from: usize, needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    if needle.is_empty() || from >= chars.len() {
        return None;
    }
    (from..=chars.len().saturating_sub(needle.len()))
        .find(|&i| chars[i..i + needle.len()] == needle[..])
}

fn parse_attrs(input: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let mut rest = input.trim();
    while !rest.is_empty() {
        let Some(eq) = rest.find('=') else {
            break;
        };
        let key = rest[..eq].trim().to_string();
        rest = rest[eq + 1..].trim_start();
        let Some(quote) = rest.chars().next() else {
            break;
        };
        if quote != '"' && quote != '\'' {
            break;
        }
        let value_start = 1;
        let Some(end) = rest[value_start..].find(quote) else {
            break;
        };
        let value = decode_entities(&rest[value_start..value_start + end]);
        attrs.push((key.to_lowercase(), value));
        rest = rest[value_start + end + 1..].trim_start();
    }
    attrs
}

fn decode_entities(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        rest = &rest[pos..];
        let Some(end) = rest.find(';') else {
            out.push_str(rest);
            return out;
        };
        let entity = &rest[1..end];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => {
                if let Some(hex) = entity
                    .strip_prefix("#x")
                    .or_else(|| entity.strip_prefix("#X"))
                {
                    u32::from_str_radix(hex, 16).ok().and_then(char::from_u32)
                } else if let Some(dec) = entity.strip_prefix('#') {
                    dec.parse::<u32>().ok().and_then(char::from_u32)
                } else {
                    None
                }
            }
        };
        if let Some(c) = decoded {
            out.push(c);
        } else {
            out.push_str(&rest[..=end]);
        }
        rest = &rest[end + 1..];
    }
    out.push_str(rest);
    out
}

#[derive(Clone, Debug)]
enum PlistValue {
    String(String),
    Integer(i64),
    Array(Vec<PlistValue>),
    Dict(Vec<(String, PlistValue)>),
    Bool(bool),
    Data,
}

fn parse_xml_file(path: &Path) -> Result<ParsedExport> {
    let text = std::fs::read_to_string(path)?;
    if !text.contains("<plist") && !text.contains("Tracks") {
        return Err(anyhow!(
            "the XML file is not an iTunes library export; XSPF playlists must use the .xspf extension"
        ));
    }
    let events = parse_xml(&text);
    let mut pos = 0usize;
    let mut plist = None;
    while pos < events.len() {
        if let XmlEvent::Start { name, .. } = &events[pos] {
            if name == "plist" {
                pos += 1;
                plist = parse_plist_value(&events, &mut pos);
                break;
            }
        }
        pos += 1;
    }
    let Some(PlistValue::Dict(root)) = plist else {
        return Err(anyhow!("the iTunes library file has no plist dictionary"));
    };

    let tracks_dict = root
        .iter()
        .find(|(k, _)| k == "Tracks")
        .and_then(|(_, v)| match v {
            PlistValue::Dict(d) => Some(d),
            _ => None,
        })
        .ok_or_else(|| anyhow!("the iTunes library file has no Tracks section"))?;

    let mut tracks = Vec::new();
    let mut by_id: std::collections::HashMap<String, usize> = Default::default();
    for (track_id, value) in tracks_dict {
        let PlistValue::Dict(fields) = value else {
            continue;
        };
        let get = |key: &str| -> Option<&str> {
            fields
                .iter()
                .find(|(k, _)| k == key)
                .and_then(|(_, v)| match v {
                    PlistValue::String(s) => Some(s.as_str()),
                    _ => None,
                })
        };
        let get_int = |key: &str| -> Option<i64> {
            fields
                .iter()
                .find(|(k, _)| k == key)
                .and_then(|(_, v)| match v {
                    PlistValue::Integer(i) => Some(*i),
                    PlistValue::String(s) => s.parse().ok(),
                    _ => None,
                })
        };
        let Some(title) = get("Name") else {
            continue;
        };
        let artist = get("Artist").unwrap_or("Unknown artist").to_string();
        let album = get("Album").unwrap_or("Unknown album").to_string();
        let duration_ms = get_int("Total Time").unwrap_or(0);
        let location = get("Location").unwrap_or("");
        let file_path = location
            .parse::<Url>()
            .ok()
            .filter(|u| u.scheme() == "file")
            .and_then(|u| u.to_file_path().ok())
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let external_url = if file_path.is_empty() {
            location.to_string()
        } else {
            String::new()
        };
        let source_id = get("Persistent ID")
            .map(str::to_string)
            .unwrap_or_else(|| format!("itunes:{track_id}"));
        let mut track = ParsedTrack {
            source: "file".into(),
            source_id: source_id.clone(),
            title: title.to_string(),
            artist,
            album,
            duration_secs: duration_ms as f64 / 1000.0,
            track_no: get_int("Track Number").unwrap_or(0) as i32,
            disc_no: get_int("Disc Number").unwrap_or(0) as i32,
            year: get("Year").unwrap_or("").to_string(),
            isrc: get("ISRC").unwrap_or("").to_string(),
            external_url,
            thumbnail: String::new(),
            file_path,
            album_artist: get("Album Artist").unwrap_or("").to_string(),
        };
        if track.title.is_empty() {
            continue;
        }
        by_id.insert(track_id.clone(), tracks.len());
        tracks.push(track.clone());
    }

    let mut playlists = Vec::new();
    if let Some((_, PlistValue::Array(items))) = root.iter().find(|(k, _)| k == "Playlists") {
        for item in items {
            let PlistValue::Dict(fields) = item else {
                continue;
            };
            let name = fields
                .iter()
                .find(|(k, _)| k == "Name")
                .and_then(|(_, v)| match v {
                    PlistValue::String(s) => Some(s.clone()),
                    _ => None,
                })
                .unwrap_or_else(|| "Imported playlist".into());
            let Some((_, PlistValue::Array(entries))) =
                fields.iter().find(|(k, _)| k == "Playlist Items")
            else {
                continue;
            };
            let mut ids = Vec::new();
            for entry in entries {
                let PlistValue::Dict(e) = entry else {
                    continue;
                };
                let track_id = e
                    .iter()
                    .find(|(k, _)| k == "Track ID")
                    .and_then(|(_, v)| match v {
                        PlistValue::Integer(i) => Some(i.to_string()),
                        PlistValue::String(s) => Some(s.clone()),
                        _ => None,
                    });
                if let Some(track_id) = track_id {
                    if let Some(index) = by_id.get(&track_id) {
                        ids.push(tracks[*index].source_id.clone());
                    }
                }
            }
            if !ids.is_empty() {
                playlists.push(ParsedPlaylist {
                    name,
                    track_ids: ids,
                });
            }
        }
    }

    if tracks.is_empty() {
        return Err(anyhow!(
            "the iTunes library file does not contain any songs"
        ));
    }
    Ok(ParsedExport {
        provider: "apple".into(),
        format: "itunes-xml".into(),
        tracks,
        playlists,
        skipped: 0,
        warnings: Vec::new(),
    })
}

fn parse_plist_value(events: &[XmlEvent], pos: &mut usize) -> Option<PlistValue> {
    while *pos < events.len() {
        match &events[*pos] {
            XmlEvent::Start { name, .. } => {
                let name = name.clone();
                *pos += 1;
                return Some(match name.as_str() {
                    "dict" => {
                        let mut entries = Vec::new();
                        loop {
                            if *pos >= events.len() {
                                break;
                            }
                            match &events[*pos] {
                                XmlEvent::End { name } if name == "dict" => {
                                    *pos += 1;
                                    break;
                                }
                                XmlEvent::Start { name, .. } if name == "key" => {
                                    *pos += 1;
                                    let key = match events.get(*pos) {
                                        Some(XmlEvent::Text(t)) => t.trim().to_string(),
                                        _ => String::new(),
                                    };
                                    *pos += 1;
                                    if matches!(events.get(*pos), Some(XmlEvent::End { name }) if name == "key")
                                    {
                                        *pos += 1;
                                    }
                                    let value = parse_plist_value(events, pos)
                                        .unwrap_or(PlistValue::String(String::new()));
                                    entries.push((key, value));
                                }
                                _ => {
                                    *pos += 1;
                                }
                            }
                        }
                        PlistValue::Dict(entries)
                    }
                    "array" => {
                        let mut items = Vec::new();
                        loop {
                            if *pos >= events.len() {
                                break;
                            }
                            match &events[*pos] {
                                XmlEvent::End { name } if name == "array" => {
                                    *pos += 1;
                                    break;
                                }
                                XmlEvent::Start { .. } => {
                                    if let Some(value) = parse_plist_value(events, pos) {
                                        items.push(value);
                                    }
                                }
                                _ => {
                                    *pos += 1;
                                }
                            }
                        }
                        PlistValue::Array(items)
                    }
                    "string" | "date" => {
                        let text = match events.get(*pos) {
                            Some(XmlEvent::Text(t)) => t.trim().to_string(),
                            _ => String::new(),
                        };
                        if matches!(events.get(*pos + 1), Some(XmlEvent::End { name }) if name == "string" || name == "date")
                        {
                            *pos += 2;
                        } else {
                            *pos += 1;
                        }
                        PlistValue::String(text)
                    }
                    "integer" => {
                        let text = match events.get(*pos) {
                            Some(XmlEvent::Text(t)) => t.trim().to_string(),
                            _ => String::new(),
                        };
                        if matches!(events.get(*pos + 1), Some(XmlEvent::End { name }) if name == "integer")
                        {
                            *pos += 2;
                        } else {
                            *pos += 1;
                        }
                        PlistValue::Integer(text.parse().unwrap_or(0))
                    }
                    "true" | "false" => {
                        if matches!(events.get(*pos), Some(XmlEvent::End { name: n }) if n == &name)
                        {
                            *pos += 1;
                        }
                        PlistValue::Bool(name == "true")
                    }
                    "data" => {
                        if matches!(events.get(*pos), Some(XmlEvent::End { name }) if name == "data")
                        {
                            *pos += 1;
                        }
                        PlistValue::Data
                    }
                    _ => {
                        // Unknown leaf; consume its end tag if present.
                        if matches!(events.get(*pos), Some(XmlEvent::End { name: n }) if n == &name)
                        {
                            *pos += 1;
                        }
                        PlistValue::String(String::new())
                    }
                });
            }
            XmlEvent::End { .. } => {
                *pos += 1;
                return None;
            }
            XmlEvent::Text(_) => {
                *pos += 1;
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str, contents: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "melyar-import-test-{}-{}",
            std::process::id(),
            name
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn m3u_preserves_order_and_extinf() {
        let path = temp_file(
            "list.m3u8",
            "#EXTM3U\n#EXTINF:123,Artist One - Song One\n/missing/one.mp3\n#EXTINF:45,Artist Two - Song Two\n/missing/two.mp3\n",
        );
        let parsed = parse_m3u(&path).unwrap();
        assert_eq!(parsed.tracks.len(), 2);
        assert_eq!(parsed.tracks[0].title, "Song One");
        assert_eq!(parsed.tracks[0].artist, "Artist One");
        assert_eq!(parsed.tracks[0].duration_secs, 123.0);
        assert_eq!(parsed.tracks[1].title, "Song Two");
        let ids: Vec<_> = parsed.playlists[0].track_ids.clone();
        assert_eq!(ids.len(), 2);
        assert!(ids[0].contains("one"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn csv_handles_quoted_commas_and_newlines() {
        let path = temp_file(
            "list.csv",
            "Title,Artist,Album\n\"Song, One\",\"Artist, One\",Album\n\"Two\nLines\",Artist Two,Album\n",
        );
        let parsed = parse_csv_file(&path).unwrap();
        assert_eq!(parsed.tracks.len(), 2);
        assert_eq!(parsed.tracks[0].title, "Song, One");
        assert_eq!(parsed.tracks[1].title, "Two\nLines");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn spotify_json_export_is_read() {
        let path = temp_file(
            "YourLibrary.json",
            r#"{"tracks":[{"artist":"Queen","album":"A Night at the Opera","track":"Bohemian Rhapsody","type":"track"}]}"#,
        );
        let parsed = parse_json_file(&path).unwrap();
        assert_eq!(parsed.tracks.len(), 1);
        assert_eq!(parsed.tracks[0].title, "Bohemian Rhapsody");
        assert_eq!(parsed.tracks[0].artist, "Queen");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn apple_playlist_order_is_preserved() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>Tracks</key><dict>
<key>1</key><dict><key>Name</key><string>First</string><key>Artist</key><string>A</string><key>Album</key><string>X</string><key>Total Time</key><integer>1000</integer></dict>
<key>2</key><dict><key>Name</key><string>Second</string><key>Artist</key><string>B</string><key>Album</key><string>X</string><key>Total Time</key><integer>2000</integer></dict>
</dict>
<key>Playlists</key><array>
<dict><key>Name</key><string>Mix</string><key>Playlist Items</key><array>
<dict><key>Track ID</key><integer>2</integer></dict>
<dict><key>Track ID</key><integer>1</integer></dict>
</array></dict>
</array>
</dict></plist>"#;
        let path = temp_file("Library.xml", xml);
        let parsed = parse_xml_file(&path).unwrap();
        assert_eq!(parsed.tracks.len(), 2);
        assert_eq!(parsed.playlists.len(), 1);
        let first_id = parsed.playlists[0].track_ids[0].clone();
        let track = parsed
            .tracks
            .iter()
            .find(|t| t.source_id == first_id)
            .unwrap();
        assert_eq!(track.title, "Second");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn xspf_reads_tracks_and_playlist_name() {
        let xml = r#"<?xml version="1.0"?><playlist xmlns="http://xspf.org/ns/0/" version="1">
<title>Road</title><trackList>
<track><location>/missing/one.mp3</location><title>One</title><creator>Artist</creator><duration>90000</duration></track>
</trackList></playlist>"#;
        let path = temp_file("list.xspf", xml);
        let parsed = parse_xspf(&path).unwrap();
        assert_eq!(parsed.playlists[0].name, "Road");
        assert_eq!(parsed.tracks[0].title, "One");
        assert!((parsed.tracks[0].duration_secs - 90.0).abs() < 0.01);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn unsupported_zip_is_rejected_with_a_clear_message() {
        let path = temp_file("export.zip", "PK");
        let err = parse_file(&path).unwrap_err().to_string();
        assert!(err.contains("not supported"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
