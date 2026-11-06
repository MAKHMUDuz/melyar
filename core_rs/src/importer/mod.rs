//! The Import & Transfer Centre backend.
//!
//! This module owns one preview at a time, scans folders, reads export files,
//! matches what it finds against the library, and writes the confirmed result.
//! It never moves or deletes a source file, and it emits every state change as
//! a JSON event so the interface can show exactly what happened.

pub mod formats;
pub mod matching;
pub mod metadata;
pub mod spotify;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use once_cell::sync::Lazy;
use parking_lot::Mutex;
use serde_json::{json, Value};

use crate::db::import_store::{ImportFolderRow, ImportItemRow, ImportPlaylistRow};
use crate::ffi;
use formats::{ParsedExport, ParsedTrack};
use matching::{Candidate, MatchResult};

static CANCEL: AtomicBool = AtomicBool::new(false);
static PREVIEW_SEQ: AtomicU64 = AtomicU64::new(1);
static PREVIEWS: Lazy<Mutex<Vec<Preview>>> = Lazy::new(|| Mutex::new(Vec::new()));

struct PreviewItem {
    row: ImportItemRow,
    duplicate: bool,
    match_state: String,
    match_confidence: i32,
    candidates: Vec<Candidate>,
}

struct Preview {
    id: String,
    kind: String,
    provider: String,
    format: String,
    name: String,
    path: String,
    mode: String,
    copy_target: String,
    items: Vec<PreviewItem>,
    playlists: Vec<formats::ParsedPlaylist>,
    skipped: usize,
    warnings: Vec<String>,
    unreadable: usize,
}

pub fn cancel() -> bool {
    CANCEL.store(true, Ordering::SeqCst);
    true
}

fn cancelled() -> bool {
    CANCEL.load(Ordering::SeqCst)
}

fn emit(value: Value) {
    ffi::on_import_event(value.to_string());
}

fn emit_progress(phase: &str, current: usize, total: usize, detail: Value) {
    emit(json!({
        "type": "progress",
        "phase": phase,
        "current": current,
        "total": total,
        "done": false,
        "detail": detail,
    }));
}

fn emit_done(phase: &str, summary: Value) {
    emit(json!({
        "type": "done",
        "phase": phase,
        "done": true,
        "summary": summary,
    }));
}

fn emit_error(code: &str, message: &str) {
    emit(json!({
        "type": "error",
        "code": code,
        "message": message,
    }));
}

// ---------------------------------------------------------------------------
// Snapshot
// ---------------------------------------------------------------------------

/// Everything the Import & Transfer page needs for its first paint.
pub fn state_json() -> String {
    let core = crate::get_core();
    let storage = core.storage();
    let folders = storage.list_import_folders().unwrap_or_default();
    let items = storage.list_import_items(100_000).unwrap_or_default();
    let playlists = storage.list_import_playlists().unwrap_or_default();
    let local_items = items.iter().filter(|i| i.source == "local").count();
    let playable = items.iter().filter(|i| i.is_playable()).count();
    let metadata_only = items.len().saturating_sub(playable);
    let spotify = spotify::status_json();

    let folder_json: Vec<Value> = folders
        .iter()
        .map(|folder| {
            json!({
                "path": folder.path,
                "mode": folder.mode,
                "autoScan": folder.auto_scan,
                "lastScan": folder.last_scan,
                "tracks": folder.track_count,
                "albums": folder.album_count,
                "artists": folder.artist_count,
                "accessible": folder.accessible,
                "status": folder.status,
            })
        })
        .collect();

    let playlist_json: Vec<Value> = playlists
        .iter()
        .map(|p| {
            json!({
                "id": p.playlist_id,
                "source": p.source,
                "name": p.name,
                "thumbnail": p.thumbnail,
                "tracks": p.track_count,
                "importedAt": p.imported_at,
            })
        })
        .collect();

    json!({
        "folders": folder_json,
        "playlists": playlist_json,
        "stats": {
            "items": items.len(),
            "localItems": local_items,
            "playable": playable,
            "metadataOnly": metadata_only,
        },
        "providers": {
            "local": {
                "status": "ready",
                "folders": folders.len(),
                "tracks": local_items,
            },
            "spotify": spotify,
            "apple": {
                "status": "export_only",
                "reason": "Direct Apple Music authorization needs a developer token and MusicKit, which this Linux desktop build cannot obtain on its own. The iTunes Library XML export is supported.",
            },
            "file": {
                "status": "ready",
                "formats": ["json", "csv", "m3u", "m3u8", "pls", "xspf", "itunes-xml"],
            },
            "youtube": {
                "status": "not_available",
                "reason": "YouTube Music has no supported library export API. A Google Takeout CSV can be imported through Import from File.",
            },
            "yandex": {
                "status": "not_available",
                "reason": "Yandex Music has no supported library export API. A JSON or CSV export can be imported through Import from File.",
            },
        },
    })
    .to_string()
}

// ---------------------------------------------------------------------------
// Local folders
// ---------------------------------------------------------------------------

/// Scan a folder and build a preview. The actual walk runs on the core's
/// runtime so a large collection does not block the interface.
pub fn scan_folder(path: String, options_json: String) -> bool {
    let options: Value = serde_json::from_str(&options_json).unwrap_or_else(|_| json!({}));
    let mode = options
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("index")
        .to_string();
    let copy_target = options
        .get("copyTarget")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let auto_commit = options
        .get("autoCommit")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    CANCEL.store(false, Ordering::SeqCst);
    let core = crate::get_core();
    let storage = core.storage().clone();
    core.runtime().spawn(async move {
        let path_buf = PathBuf::from(&path);
        if !path_buf.exists() {
            emit_error(
                "folder_missing",
                "That folder does not exist or is not accessible.",
            );
            return;
        }
        if !path_buf.is_dir() {
            emit_error("not_a_folder", "The selected path is not a folder.");
            return;
        }

        let files = walk_audio_files(&path_buf, &mut |current, total| {
            if current % 50 == 0 {
                emit_progress("scanning", current, total, json!({}));
            }
        });
        if files.is_empty() {
            emit_error(
                "no_audio",
                "No supported audio files were found in that folder.",
            );
            return;
        }

        let total = files.len();
        emit_progress("scanning", 0, total, json!({"folder": path}));
        let mut items = Vec::new();
        let mut unreadable = 0usize;
        let mut warnings: Vec<String> = Vec::new();

        for (index, file) in files.iter().enumerate() {
            if cancelled() {
                emit(json!({"type": "cancelled", "phase": "scanning"}));
                return;
            }
            let size = file.metadata().map(|m| m.len() as i64).unwrap_or(0);
            let mtime = file
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);

            // A file already indexed with the same size and timestamp is not
            // read again. That is what makes reopening a large folder cheap.
            if storage
                .import_file_unchanged(&file.to_string_lossy(), size, mtime)
                .unwrap_or(false)
            {
                if index % 25 == 0 {
                    emit_progress("scanning", index + 1, total, json!({"folder": path}));
                }
                continue;
            }

            match metadata::read_metadata(file) {
                Ok(meta) => {
                    let thumbnail = meta
                        .picture
                        .as_ref()
                        .and_then(metadata::cache_picture)
                        .unwrap_or_default();
                    let item = item_from_metadata(file, &meta, size, mtime, thumbnail);
                    items.push(item);
                }
                Err(err) => {
                    unreadable += 1;
                    if warnings.len() < 20 {
                        warnings.push(format!(
                            "{}: {}",
                            file.file_name().and_then(|n| n.to_str()).unwrap_or("file"),
                            err
                        ));
                    }
                }
            }
            if index % 25 == 0 || index + 1 == total {
                emit_progress("scanning", index + 1, total, json!({"folder": path}));
            }
        }

        preview_from_local(
            path,
            mode,
            copy_target,
            items,
            unreadable,
            warnings,
            auto_commit,
        );
    });
    true
}

fn preview_from_local(
    folder: String,
    mode: String,
    copy_target: String,
    mut rows: Vec<ImportItemRow>,
    unreadable: usize,
    warnings: Vec<String>,
    auto_commit: bool,
) {
    let core = crate::get_core();
    let storage = core.storage();

    // Duplicate detection: an item already in the library, or two scanned
    // files with the same metadata fingerprint.
    let mut seen_fingerprints: HashSet<String> = HashSet::new();
    let mut duplicates = 0usize;
    let mut preview_items = Vec::new();
    for row in rows.drain(..) {
        let fingerprint = fingerprint(&row.title, &row.artist, &row.album, row.duration_secs);
        let exists = storage.import_item_exists(&row.item_id).unwrap_or(false);
        let repeated = !seen_fingerprints.insert(fingerprint);
        if exists || repeated {
            duplicates += 1;
        }
        preview_items.push(PreviewItem {
            row,
            duplicate: exists || repeated,
            match_state: "exact".into(),
            match_confidence: 100,
            candidates: Vec::new(),
        });
    }

    if preview_items.is_empty() {
        emit_error(
            "no_new_audio",
            "Every file in that folder is already indexed or could not be read.",
        );
        return;
    }

    let preview = Preview {
        id: format!("preview-{}", PREVIEW_SEQ.fetch_add(1, Ordering::SeqCst)),
        kind: "local".into(),
        provider: "local".into(),
        format: "folder".into(),
        name: Path::new(&folder)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&folder)
            .to_string(),
        path: folder,
        mode,
        copy_target,
        items: preview_items,
        playlists: Vec::new(),
        skipped: 0,
        warnings,
        unreadable,
    };

    let payload = preview_json(&preview);
    let id = preview.id.clone();
    let commit_options = json!({
        "previewId": id,
        "skipDuplicates": true,
        "autoCommit": auto_commit,
    });
    remember_preview(preview);
    emit(json!({"type": "preview", "preview": payload}));
    if auto_commit {
        commit_preview(commit_options.to_string());
    }
}

fn item_from_metadata(
    file: &Path,
    meta: &metadata::AudioMetadata,
    size: i64,
    mtime: i64,
    thumbnail: String,
) -> ImportItemRow {
    let path = std::fs::canonicalize(file).unwrap_or_else(|_| file.to_path_buf());
    let item_id = format!("local:{:016x}", fnv(&path.to_string_lossy()));
    ImportItemRow {
        item_id: item_id.clone(),
        track_id: item_id,
        source: "local".into(),
        source_id: path.to_string_lossy().to_string(),
        title: meta.title.clone(),
        artist: meta.artist.clone(),
        album: meta.album.clone(),
        album_artist: if meta.album_artist.is_empty() {
            meta.artist.clone()
        } else {
            meta.album_artist.clone()
        },
        duration_secs: meta.duration_secs,
        thumbnail,
        isrc: meta.isrc.clone(),
        external_url: String::new(),
        file_path: path.to_string_lossy().to_string(),
        track_no: meta.track_no,
        disc_no: meta.disc_no,
        year: meta.year.clone(),
        availability: "local_file".into(),
        match_status: "exact".into(),
        match_confidence: 100,
        file_size: size,
        file_mtime: mtime,
        imported_at: now(),
    }
}

fn walk_audio_files(root: &Path, progress: &mut dyn FnMut(usize, usize)) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    let mut visited: HashSet<PathBuf> = HashSet::new();
    let mut directories = 0usize;
    while let Some(dir) = stack.pop() {
        if directories > 20_000 || out.len() > 200_000 {
            break;
        }
        let canonical = std::fs::canonicalize(&dir).unwrap_or_else(|_| dir.clone());
        if !visited.insert(canonical) {
            continue;
        }
        directories += 1;
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut children: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        children.sort();
        for child in children {
            let name = child.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') {
                continue;
            }
            let Ok(meta) = std::fs::symlink_metadata(&child) else {
                continue;
            };
            if meta.is_dir() {
                stack.push(child);
            } else if meta.is_file() && metadata::is_audio_file(&child) {
                out.push(child);
                progress(out.len(), out.len());
            }
        }
    }
    out.sort();
    out
}

fn fingerprint(title: &str, artist: &str, album: &str, duration: f64) -> String {
    format!(
        "{}|{}|{}|{}",
        matching::normalise(title),
        matching::normalise(artist),
        matching::normalise(album),
        duration.round() as i64
    )
}

// ---------------------------------------------------------------------------
// Export-file preview
// ---------------------------------------------------------------------------

/// Parse an export file and return a preview synchronously. The C++ bridge
/// calls this on a worker thread, so a large file does not block the UI thread.
pub fn preview_file(path: String) -> String {
    let path_buf = PathBuf::from(&path);
    if !path_buf.is_file() {
        return json!({
            "error": {"code": "file_missing", "message": "That file does not exist or is not readable."}
        })
        .to_string();
    }
    let parsed = match formats::parse_file(&path_buf) {
        Ok(parsed) => parsed,
        Err(err) => {
            return json!({
                "error": {"code": "invalid_export", "message": err.to_string()}
            })
            .to_string();
        }
    };
    if parsed.all_track_count() == 0 {
        return json!({
            "error": {"code": "no_tracks", "message": "The file was read, but it contains no tracks."}
        })
        .to_string();
    }

    let name = path_buf
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Import")
        .to_string();
    preview_from_tracks(
        &parsed.provider,
        &parsed.format,
        &name,
        &path,
        parsed.tracks,
        parsed.playlists,
        parsed.skipped,
        parsed.warnings,
    )
    .to_string()
}

/// Turn parsed tracks into a stored preview. Shared by export files and the
/// Spotify fetch, so both go through the same matching and duplicate rules.
pub(crate) fn preview_from_tracks(
    provider: &str,
    format: &str,
    name: &str,
    path: &str,
    tracks: Vec<ParsedTrack>,
    raw_playlists: Vec<formats::ParsedPlaylist>,
    skipped: usize,
    warnings: Vec<String>,
) -> Value {
    let candidates = library_candidates();
    let mut rows = Vec::new();
    let mut duplicates = 0usize;

    for track in &tracks {
        let (row, duplicate, result) = row_from_parsed(track, provider, &candidates);
        if duplicate {
            duplicates += 1;
        }
        rows.push(PreviewItem {
            row,
            duplicate,
            match_state: result.state,
            match_confidence: result.confidence,
            candidates: result
                .candidate
                .into_iter()
                .chain(result.alternatives)
                .collect(),
        });
    }

    let mut playlists = Vec::new();
    for playlist in &raw_playlists {
        let mut ids = Vec::new();
        for track_id in &playlist.track_ids {
            if let Some(track) = tracks.iter().find(|t| &t.source_id == track_id) {
                let (row, duplicate, result) = row_from_parsed(track, provider, &candidates);
                if duplicate {
                    duplicates += 1;
                }
                ids.push(row.item_id.clone());
                if !rows.iter().any(|r| r.row.item_id == row.item_id) {
                    rows.push(PreviewItem {
                        row,
                        duplicate,
                        match_state: result.state,
                        match_confidence: result.confidence,
                        candidates: result
                            .candidate
                            .into_iter()
                            .chain(result.alternatives)
                            .collect(),
                    });
                }
            } else {
                ids.push(track_id.clone());
            }
        }
        playlists.push(formats::ParsedPlaylist {
            name: playlist.name.clone(),
            track_ids: ids,
        });
    }

    if rows.is_empty() {
        return json!({
            "error": {"code": "no_tracks", "message": "The source contained no importable tracks."}
        });
    }

    let preview = Preview {
        id: format!("preview-{}", PREVIEW_SEQ.fetch_add(1, Ordering::SeqCst)),
        kind: "file".into(),
        provider: provider.to_string(),
        format: format.to_string(),
        name: name.to_string(),
        path: path.to_string(),
        mode: "index".into(),
        copy_target: String::new(),
        items: rows,
        playlists,
        skipped,
        warnings,
        unreadable: 0,
    };
    let payload = preview_json(&preview);
    remember_preview(preview);
    let _ = duplicates;
    payload
}

fn library_candidates() -> Vec<Candidate> {
    let storage = crate::get_core().storage();
    let mut candidates = Vec::new();
    for item in storage.list_import_items(20_000).unwrap_or_default() {
        if !item.is_playable() {
            continue;
        }
        candidates.push(Candidate {
            track_id: item.track_id.clone(),
            item_id: item.item_id.clone(),
            title: item.title,
            artist: item.artist,
            album: item.album,
            duration_secs: item.duration_secs,
            thumbnail: item.thumbnail,
            source: item.source,
            isrc: item.isrc,
            availability: item.availability,
        });
    }
    for track in storage.get_favorites().unwrap_or_default() {
        candidates.push(Candidate {
            track_id: track.id.clone(),
            item_id: String::new(),
            title: track.title,
            artist: track.artist,
            album: track.album,
            duration_secs: track.duration_secs,
            thumbnail: track.thumbnail_url,
            source: "youtube".into(),
            isrc: String::new(),
            availability: "provider".into(),
        });
    }
    for track in storage.get_history(500).unwrap_or_default() {
        candidates.push(Candidate {
            track_id: track.id.clone(),
            item_id: String::new(),
            title: track.title,
            artist: track.artist,
            album: track.album,
            duration_secs: track.duration_secs,
            thumbnail: track.thumbnail_url,
            source: "youtube".into(),
            isrc: String::new(),
            availability: "provider".into(),
        });
    }
    candidates
}

fn row_from_parsed(
    track: &ParsedTrack,
    provider: &str,
    candidates: &[Candidate],
) -> (ImportItemRow, bool, MatchResult) {
    let source = if track.source == "file" && provider == "apple" {
        "apple"
    } else if track.source == "file" {
        "file"
    } else {
        track.source.as_str()
    };
    let hash = fnv(&format!("{source}:{}", track.source_id));
    let item_id = match source {
        "local" => format!("local:{hash:016x}"),
        "apple" => format!("apple:{hash:016x}"),
        _ => format!("file:{hash:016x}"),
    };

    let result = matching::match_track(track, candidates);
    let file_exists = !track.file_path.is_empty() && Path::new(&track.file_path).is_file();
    let (availability, track_id) = if file_exists {
        ("local_file".to_string(), item_id.clone())
    } else if let Some(candidate) = result.candidate.as_ref().filter(|c| {
        matches!(c.availability.as_str(), "local_file" | "provider")
            && matches!(result.state.as_str(), "exact" | "high")
    }) {
        (candidate.availability.clone(), candidate.track_id.clone())
    } else {
        ("metadata_only".to_string(), String::new())
    };

    let duplicate = crate::get_core()
        .storage()
        .import_item_exists(&item_id)
        .unwrap_or(false);

    let row = ImportItemRow {
        item_id,
        track_id,
        source: source.to_string(),
        source_id: track.source_id.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        album: track.album.clone(),
        album_artist: if track.album_artist.is_empty() {
            track.artist.clone()
        } else {
            track.album_artist.clone()
        },
        duration_secs: track.duration_secs,
        thumbnail: track.thumbnail.clone(),
        isrc: track.isrc.clone(),
        external_url: track.external_url.clone(),
        file_path: track.file_path.clone(),
        track_no: track.track_no,
        disc_no: track.disc_no,
        year: track.year.clone(),
        availability,
        match_status: result.state.clone(),
        match_confidence: result.confidence,
        file_size: file_size(&track.file_path),
        file_mtime: file_mtime(&track.file_path),
        imported_at: now(),
    };
    (row, duplicate, result)
}

fn preview_json(preview: &Preview) -> Value {
    let mut albums = HashSet::new();
    let mut artists = HashSet::new();
    for item in &preview.items {
        albums.insert(format!("{}|{}", item.row.album, item.row.album_artist));
        artists.insert(item.row.artist.clone());
    }
    let duplicates = preview.items.iter().filter(|i| i.duplicate).count();
    let matched: Vec<Value> = preview
        .items
        .iter()
        .take(2000)
        .map(|item| {
            json!({
                "itemId": item.row.item_id,
                "source": item.row.source,
                "sourceId": item.row.source_id,
                "trackId": item.row.track_id,
                "title": item.row.title,
                "artist": item.row.artist,
                "album": item.row.album,
                "duration": item.row.duration_secs,
                "thumbnail": item.row.thumbnail,
                "trackNo": item.row.track_no,
                "discNo": item.row.disc_no,
                "year": item.row.year,
                "isrc": item.row.isrc,
                "filePath": item.row.file_path,
                "availability": item.row.availability,
                "matchState": item.match_state,
                "confidence": item.match_confidence,
                "duplicate": item.duplicate,
                "playable": item.row.is_playable(),
                "candidates": item.candidates.iter().take(4).map(|c| json!({
                    "trackId": c.track_id,
                    "title": c.title,
                    "artist": c.artist,
                    "album": c.album,
                    "duration": c.duration_secs,
                    "thumbnail": c.thumbnail,
                    "source": c.source,
                    "availability": c.availability,
                })).collect::<Vec<_>>(),
            })
        })
        .collect();

    json!({
        "id": preview.id,
        "kind": preview.kind,
        "provider": preview.provider,
        "format": preview.format,
        "name": preview.name,
        "path": preview.path,
        "mode": preview.mode,
        "copyTarget": preview.copy_target,
        "counts": {
            "tracks": preview.items.len(),
            "albums": albums.len(),
            "artists": artists.len(),
            "playlists": preview.playlists.len(),
            "duplicates": duplicates,
            "unreadable": preview.unreadable,
            "matched": preview.items.iter().filter(|i| matches!(i.match_state.as_str(), "exact" | "high")).count(),
            "review": preview.items.iter().filter(|i| matches!(i.match_state.as_str(), "possible" | "ambiguous")).count(),
            "unavailable": preview.items.iter().filter(|i| !i.row.is_playable()).count(),
        },
        "items": matched,
        "truncated": preview.items.len() > 2000,
        "playlists": preview.playlists.iter().map(|p| json!({
            "name": p.name,
            "tracks": p.track_ids.len(),
        })).collect::<Vec<_>>(),
        "skipped": preview.skipped,
        "warnings": preview.warnings,
    })
}

fn remember_preview(preview: Preview) {
    let mut previews = PREVIEWS.lock();
    previews.push(preview);
    // A wizard keeps one preview alive; older ones are dropped so a session
    // cannot grow without bound.
    if previews.len() > 4 {
        previews.remove(0);
    }
}

// ---------------------------------------------------------------------------
// Commit
// ---------------------------------------------------------------------------

pub fn commit_preview(options_json: String) -> bool {
    let options: Value = serde_json::from_str(&options_json).unwrap_or_else(|_| json!({}));
    let preview_id = options
        .get("previewId")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let skip_duplicates = options
        .get("skipDuplicates")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let selected: Option<HashSet<String>> =
        options.get("itemIds").and_then(Value::as_array).map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        });
    let import_playlists = options
        .get("importPlaylists")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let merge = options
        .get("duplicatePolicy")
        .and_then(Value::as_str)
        .unwrap_or("skip")
        .to_string();

    let preview = {
        let mut previews = PREVIEWS.lock();
        let index = previews.iter().position(|p| p.id == preview_id);
        index.map(|i| previews.remove(i))
    };
    let Some(preview) = preview else {
        emit_error(
            "preview_expired",
            "That preview is no longer available. Choose the source again.",
        );
        return false;
    };

    CANCEL.store(false, Ordering::SeqCst);
    let core = crate::get_core();
    let storage = core.storage().clone();
    core.runtime().spawn(async move {
        let total = preview.items.len();
        let mut imported = 0usize;
        let mut skipped = 0usize;
        let mut failed = 0usize;
        let mut copied = 0usize;
        let mut written_ids: HashMap<String, String> = HashMap::new();

        let managed_dir = if preview.mode == "copy" && !preview.copy_target.is_empty() {
            Some(PathBuf::from(&preview.copy_target))
        } else {
            None
        };

        for (index, item) in preview.items.iter().enumerate() {
            if cancelled() {
                emit(json!({
                    "type": "cancelled",
                    "phase": "import",
                    "summary": {
                        "imported": imported,
                        "skipped": skipped,
                        "failed": failed,
                        "total": total,
                    }
                }));
                return;
            }
            if let Some(selected) = &selected {
                if !selected.contains(&item.row.item_id) {
                    skipped += 1;
                    continue;
                }
            }
            if skip_duplicates && item.duplicate && merge == "skip" {
                skipped += 1;
                emit_progress(
                    "importing",
                    index + 1,
                    total,
                    json!({"skipped": skipped, "imported": imported}),
                );
                continue;
            }

            let mut row = item.row.clone();
            if let Some(dir) = &managed_dir {
                match copy_into_managed(&row, dir, &preview.path) {
                    Ok(new_path) => {
                        row.file_path = new_path.clone();
                        row.source_id = new_path;
                        row.availability = "local_file".into();
                        row.track_id = row.item_id.clone();
                        copied += 1;
                    }
                    Err(err) => {
                        failed += 1;
                        if failed <= 20 {
                            emit(json!({
                                "type": "item_error",
                                "itemId": row.item_id,
                                "message": err.to_string(),
                            }));
                        }
                        continue;
                    }
                }
            }

            match storage.upsert_import_item(&row) {
                Ok(()) => {
                    imported += 1;
                    written_ids.insert(row.item_id.clone(), row.item_id.clone());
                    // A matched item points at the playable recording, but the
                    // imported identity stays its own row.
                    if row.track_id.is_empty() && !row.is_playable() {
                        row.availability = "metadata_only".into();
                    }
                }
                Err(err) => {
                    failed += 1;
                    if failed <= 20 {
                        emit(json!({
                            "type": "item_error",
                            "itemId": row.item_id,
                            "message": err.to_string(),
                        }));
                    }
                }
            }
            emit_progress(
                "importing",
                index + 1,
                total,
                json!({"skipped": skipped, "imported": imported, "failed": failed}),
            );
        }

        if import_playlists {
            for playlist in &preview.playlists {
                let ids: Vec<String> = playlist
                    .track_ids
                    .iter()
                    .filter_map(|id| written_ids.get(id).cloned().or_else(|| Some(id.clone())))
                    .collect();
                if ids.is_empty() {
                    continue;
                }
                let playlist_id = format!(
                    "{}-{}",
                    preview.provider,
                    fnv(&format!("{}:{}", preview.path, playlist.name))
                );
                let row = ImportPlaylistRow {
                    playlist_id: playlist_id.clone(),
                    source: preview.provider.clone(),
                    name: playlist.name.clone(),
                    thumbnail: String::new(),
                    track_count: ids.len() as i64,
                    imported_at: now(),
                };
                let _ = storage.upsert_import_playlist(&row);
                let _ = storage.replace_import_playlist_tracks(&playlist_id, &ids);
            }
        }

        if preview.kind == "local" {
            let (albums, artists) = folder_counts(&storage, &preview.path);
            let folder = ImportFolderRow {
                path: preview.path.clone(),
                mode: preview.mode.clone(),
                auto_scan: true,
                last_scan: now(),
                track_count: imported as i64,
                album_count: albums,
                artist_count: artists,
                accessible: true,
                status: "ready".into(),
                added_at: now(),
            };
            let _ = storage.upsert_import_folder(&folder);
        }

        let summary = json!({
            "imported": imported,
            "skipped": skipped,
            "failed": failed,
            "duplicates": skipped,
            "copied": copied,
            "total": total,
            "playlists": preview.playlists.len(),
            "provider": preview.provider,
        });
        emit(json!({
            "type": "complete",
            "phase": "import",
            "done": true,
            "summary": summary,
        }));
        emit_done("import", summary);
        let _ = copied;
    });
    true
}

fn copy_into_managed(row: &ImportItemRow, dir: &Path, root: &str) -> anyhow::Result<String> {
    let source = Path::new(&row.file_path);
    if !source.is_file() {
        return Err(anyhow::anyhow!("source file is no longer available"));
    }
    let relative = source.strip_prefix(root).unwrap_or(source);
    let mut target = dir.join(relative);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if target.exists() {
        // Never overwrite an existing managed file. A suffix keeps both.
        let stem = target
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("track");
        let ext = target
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        let mut n = 2;
        loop {
            let name = if ext.is_empty() {
                format!("{stem} ({n})")
            } else {
                format!("{stem} ({n}).{ext}")
            };
            let candidate = target.with_file_name(name);
            if !candidate.exists() {
                target = candidate;
                break;
            }
            n += 1;
        }
    }
    // Copy through a temporary name in the same directory, then rename, so an
    // interrupted copy cannot leave a half-written track behind.
    let tmp = target.with_extension("melyar-part");
    std::fs::copy(source, &tmp)?;
    std::fs::rename(&tmp, &target)?;
    Ok(target.to_string_lossy().to_string())
}

fn folder_counts(storage: &crate::db::storage::DbStorage, root: &str) -> (i64, i64) {
    let items = storage.list_import_items(100_000).unwrap_or_default();
    let mut albums = HashSet::new();
    let mut artists = HashSet::new();
    let prefix = root.trim_end_matches('/');
    for item in items.iter().filter(|i| {
        i.source == "local"
            && (i.file_path == prefix || i.file_path.starts_with(&format!("{prefix}/")))
    }) {
        albums.insert(format!("{}|{}", item.album, item.album_artist));
        artists.insert(item.artist.clone());
    }
    (albums.len() as i64, artists.len() as i64)
}

// ---------------------------------------------------------------------------
// Folder management
// ---------------------------------------------------------------------------

pub fn rescan_folder(path: String) -> bool {
    scan_folder(
        path,
        json!({"mode": "index", "autoCommit": true}).to_string(),
    )
}

pub fn remove_folder(path: String) -> bool {
    let core = crate::get_core();
    let storage = core.storage();
    let removed = storage.delete_import_items_from_folder(&path).unwrap_or(0);
    let _ = storage.remove_import_folder(&path);
    emit(json!({
        "type": "folder_removed",
        "path": path,
        "removedItems": removed,
        "filesUntouched": true,
    }));
    true
}

pub fn set_folder_auto_scan(path: String, enabled: bool) -> bool {
    crate::get_core()
        .storage()
        .set_import_folder_auto_scan(&path, enabled)
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Library views
// ---------------------------------------------------------------------------

pub fn list_items_json() -> String {
    let items = crate::get_core()
        .storage()
        .list_import_items(20_000)
        .unwrap_or_default();
    serde_json::Value::Array(
        items
            .iter()
            .map(|item| {
                json!({
                    "itemId": item.item_id,
                    "trackId": item.track_id,
                    "source": item.source,
                    "sourceId": item.source_id,
                    "title": item.title,
                    "artist": item.artist,
                    "album": item.album,
                    "albumArtist": item.album_artist,
                    "duration": item.duration_secs,
                    "thumbnail": item.thumbnail,
                    "isrc": item.isrc,
                    "externalUrl": item.external_url,
                    "filePath": item.file_path,
                    "trackNo": item.track_no,
                    "discNo": item.disc_no,
                    "year": item.year,
                    "availability": item.availability,
                    "matchState": item.match_status,
                    "confidence": item.match_confidence,
                    "playable": item.is_playable(),
                })
            })
            .collect(),
    )
    .to_string()
}

pub fn list_playlists_json() -> String {
    serde_json::Value::Array(
        crate::get_core()
            .storage()
            .list_import_playlists()
            .unwrap_or_default()
            .iter()
            .map(|p| {
                json!({
                    "id": p.playlist_id,
                    "source": p.source,
                    "name": p.name,
                    "thumbnail": p.thumbnail,
                    "tracks": p.track_count,
                })
            })
            .collect(),
    )
    .to_string()
}

pub fn playlist_items_json(playlist_id: String) -> String {
    serde_json::Value::Array(
        crate::get_core()
            .storage()
            .list_import_playlist_items(&playlist_id)
            .unwrap_or_default()
            .iter()
            .map(|item| {
                json!({
                    "trackId": if item.track_id.is_empty() { item.item_id.clone() } else { item.track_id.clone() },
                    "itemId": item.item_id,
                    "title": item.title,
                    "artist": item.artist,
                    "album": item.album,
                    "duration": item.duration_secs,
                    "thumbnail": item.thumbnail,
                    "playable": item.is_playable(),
                })
            })
            .collect(),
    )
    .to_string()
}

pub fn delete_item(item_id: String) -> bool {
    crate::get_core()
        .storage()
        .delete_import_item(&item_id)
        .unwrap_or(false)
}

pub fn review_item(item_id: String, candidate_id: String) -> bool {
    let storage = crate::get_core().storage();
    let Ok(Some(mut item)) = storage.get_import_item(&item_id) else {
        return false;
    };
    let candidates = library_candidates();
    let Some(candidate) = candidates.into_iter().find(|c| c.track_id == candidate_id) else {
        return false;
    };
    item.track_id = candidate.track_id;
    item.availability = candidate.availability;
    item.match_status = "manual".into();
    item.match_confidence = 100;
    storage.upsert_import_item(&item).is_ok()
}

/// Choose a candidate while the preview is still open. The row the commit
/// writes is the one the user reviewed, not the one matching guessed.
pub fn pick_candidate(preview_id: String, item_id: String, candidate_id: String) -> bool {
    let mut previews = PREVIEWS.lock();
    let Some(preview) = previews.iter_mut().find(|p| p.id == preview_id) else {
        return false;
    };
    let Some(item) = preview.items.iter_mut().find(|i| i.row.item_id == item_id) else {
        return false;
    };
    let Some(candidate) = item
        .candidates
        .iter()
        .find(|c| c.track_id == candidate_id)
        .cloned()
    else {
        return false;
    };
    item.row.track_id = candidate.track_id;
    item.row.availability = candidate.availability;
    item.row.match_status = "manual".into();
    item.row.match_confidence = 100;
    item.match_state = "manual".into();
    item.match_confidence = 100;
    true
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn fnv(input: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn file_size(path: &str) -> i64 {
    if path.is_empty() {
        return 0;
    }
    std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0)
}

fn file_mtime(path: &str) -> i64 {
    if path.is_empty() {
        return 0;
    }
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// The playable URL for an imported track, used by the audio path.
pub fn playable_path(item_id: &str) -> Option<String> {
    let item = crate::get_core()
        .storage()
        .get_import_item(item_id)
        .ok()??;
    if item.availability != "local_file" {
        return None;
    }
    if item.file_path.is_empty() || !Path::new(&item.file_path).is_file() {
        return None;
    }
    Some(item.file_path)
}

pub fn imported_track_metadata(item_id: &str) -> Option<crate::ffi::CxxTrackMetadata> {
    let item = crate::get_core()
        .storage()
        .get_import_item(item_id)
        .ok()??;
    Some(crate::ffi::CxxTrackMetadata {
        id: item.item_id,
        title: item.title,
        artist: item.artist,
        album: item.album,
        duration_secs: item.duration_secs,
        thumbnail_url: item.thumbnail,
        lyrics: String::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprints_ignore_case_and_punctuation() {
        assert_eq!(
            fingerprint("Don't Stop", "Queen", "Album", 120.0),
            fingerprint("dont stop", "QUEEN", "album", 120.4)
        );
    }

    #[test]
    fn fnv_is_stable() {
        assert_eq!(fnv("local:/music/a.mp3"), fnv("local:/music/a.mp3"));
        assert_ne!(fnv("local:/music/a.mp3"), fnv("local:/music/b.mp3"));
    }
}
