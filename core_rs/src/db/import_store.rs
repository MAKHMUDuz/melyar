//! Storage for imported libraries.
//!
//! The import centre writes here and nowhere else. Every row keeps the
//! provider's own identity next to the unified one, so the same recording can
//! arrive from Spotify and from a folder without one overwriting the other.

use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use super::storage::DbStorage;

/// One imported resource, normalised for the unified library.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportItemRow {
    pub item_id: String,
    /// The id that can actually be played, when one exists.
    pub track_id: String,
    pub source: String,
    pub source_id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub album_artist: String,
    pub duration_secs: f64,
    pub thumbnail: String,
    pub isrc: String,
    pub external_url: String,
    pub file_path: String,
    pub track_no: i32,
    pub disc_no: i32,
    pub year: String,
    /// local_file | provider | metadata_only | unavailable
    pub availability: String,
    /// exact | high | possible | ambiguous | not_found | manual | none
    pub match_status: String,
    pub match_confidence: i32,
    pub file_size: i64,
    pub file_mtime: i64,
    pub imported_at: i64,
}

impl ImportItemRow {
    pub fn is_playable(&self) -> bool {
        !self.track_id.is_empty()
            && matches!(self.availability.as_str(), "local_file" | "provider")
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportFolderRow {
    pub path: String,
    pub mode: String,
    pub auto_scan: bool,
    pub last_scan: i64,
    pub track_count: i64,
    pub album_count: i64,
    pub artist_count: i64,
    pub accessible: bool,
    pub status: String,
    pub added_at: i64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportPlaylistRow {
    pub playlist_id: String,
    pub source: String,
    pub name: String,
    pub thumbnail: String,
    pub track_count: i64,
    pub imported_at: i64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProviderAccountRow {
    pub provider: String,
    pub account_name: String,
    pub connected: bool,
    pub updated_at: i64,
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl DbStorage {
    pub fn upsert_import_item(&self, row: &ImportItemRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO imported_items (
                    item_id, track_id, source, source_id, title, artist, album,
                    album_artist, duration_secs, thumbnail, isrc, external_url,
                    file_path, track_no, disc_no, year, availability, match_status,
                    match_confidence, file_size, file_mtime, imported_at
                 ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22)
                 ON CONFLICT(item_id) DO UPDATE SET
                    track_id=excluded.track_id,
                    title=excluded.title,
                    artist=excluded.artist,
                    album=excluded.album,
                    album_artist=excluded.album_artist,
                    duration_secs=excluded.duration_secs,
                    thumbnail=excluded.thumbnail,
                    isrc=excluded.isrc,
                    external_url=excluded.external_url,
                    file_path=excluded.file_path,
                    track_no=excluded.track_no,
                    disc_no=excluded.disc_no,
                    year=excluded.year,
                    availability=excluded.availability,
                    match_status=excluded.match_status,
                    match_confidence=excluded.match_confidence,
                    file_size=excluded.file_size,
                    file_mtime=excluded.file_mtime,
                    imported_at=excluded.imported_at",
                params![
                    row.item_id,
                    row.track_id,
                    row.source,
                    row.source_id,
                    row.title,
                    row.artist,
                    row.album,
                    row.album_artist,
                    row.duration_secs,
                    row.thumbnail,
                    row.isrc,
                    row.external_url,
                    row.file_path,
                    row.track_no,
                    row.disc_no,
                    row.year,
                    row.availability,
                    row.match_status,
                    row.match_confidence,
                    row.file_size,
                    row.file_mtime,
                    if row.imported_at > 0 { row.imported_at } else { now() },
                ],
            )?;
            Ok(())
        })
    }

    pub fn get_import_item(&self, item_id: &str) -> Result<Option<ImportItemRow>> {
        self.with_conn(|conn| {
            let row = conn
                .query_row(
                    "SELECT item_id, track_id, source, source_id, title, artist, album,
                            album_artist, duration_secs, thumbnail, isrc, external_url,
                            file_path, track_no, disc_no, year, availability, match_status,
                            match_confidence, file_size, file_mtime, imported_at
                     FROM imported_items WHERE item_id = ?1",
                    params![item_id],
                    map_item,
                )
                .optional()?;
            Ok(row)
        })
    }

    pub fn list_import_items(&self, limit: usize) -> Result<Vec<ImportItemRow>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT item_id, track_id, source, source_id, title, artist, album,
                        album_artist, duration_secs, thumbnail, isrc, external_url,
                        file_path, track_no, disc_no, year, availability, match_status,
                        match_confidence, file_size, file_mtime, imported_at
                 FROM imported_items
                 ORDER BY imported_at DESC, artist COLLATE NOCASE, album COLLATE NOCASE, disc_no, track_no
                 LIMIT ?1",
            )?;
            let rows = stmt
                .query_map(params![limit as i64], map_item)?
                .filter_map(Result::ok)
                .collect();
            Ok(rows)
        })
    }

    pub fn delete_import_item(&self, item_id: &str) -> Result<bool> {
        self.with_conn(|conn| {
            let removed = conn.execute("DELETE FROM imported_items WHERE item_id = ?1", params![item_id])?;
            conn.execute(
                "DELETE FROM imported_playlist_tracks WHERE item_id = ?1",
                params![item_id],
            )?;
            Ok(removed > 0)
        })
    }

    pub fn delete_import_items_from_folder(&self, folder: &str) -> Result<usize> {
        self.with_conn(|conn| {
            let removed = conn.execute(
                "DELETE FROM imported_items WHERE source = 'local' AND file_path LIKE ?1",
                params![format!("{}/%", folder.trim_end_matches('/'))],
            )?;
            Ok(removed)
        })
    }

    pub fn count_import_items(&self) -> Result<i64> {
        self.with_conn(|conn| {
            let n: i64 = conn.query_row("SELECT COUNT(*) FROM imported_items", [], |r| r.get(0))?;
            Ok(n)
        })
    }

    pub fn import_item_exists(&self, item_id: &str) -> Result<bool> {
        self.with_conn(|conn| {
            let n: i64 = conn.query_row(
                "SELECT COUNT(*) FROM imported_items WHERE item_id = ?1",
                params![item_id],
                |r| r.get(0),
            )?;
            Ok(n > 0)
        })
    }

    pub fn find_import_item_by_source(&self, source: &str, source_id: &str) -> Result<Option<ImportItemRow>> {
        self.with_conn(|conn| {
            let row = conn
                .query_row(
                    "SELECT item_id, track_id, source, source_id, title, artist, album,
                            album_artist, duration_secs, thumbnail, isrc, external_url,
                            file_path, track_no, disc_no, year, availability, match_status,
                            match_confidence, file_size, file_mtime, imported_at
                     FROM imported_items WHERE source = ?1 AND source_id = ?2
                     LIMIT 1",
                    params![source, source_id],
                    map_item,
                )
                .optional()?;
            Ok(row)
        })
    }

    // ---- folders ---------------------------------------------------------

    pub fn upsert_import_folder(&self, row: &ImportFolderRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO import_folders
                    (path, mode, auto_scan, last_scan, track_count, album_count,
                     artist_count, accessible, status, added_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
                 ON CONFLICT(path) DO UPDATE SET
                    mode=excluded.mode,
                    auto_scan=excluded.auto_scan,
                    last_scan=excluded.last_scan,
                    track_count=excluded.track_count,
                    album_count=excluded.album_count,
                    artist_count=excluded.artist_count,
                    accessible=excluded.accessible,
                    status=excluded.status",
                params![
                    row.path,
                    row.mode,
                    row.auto_scan as i32,
                    row.last_scan,
                    row.track_count,
                    row.album_count,
                    row.artist_count,
                    row.accessible as i32,
                    row.status,
                    if row.added_at > 0 { row.added_at } else { now() },
                ],
            )?;
            Ok(())
        })
    }

    pub fn list_import_folders(&self) -> Result<Vec<ImportFolderRow>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT path, mode, auto_scan, last_scan, track_count, album_count,
                        artist_count, accessible, status, added_at
                 FROM import_folders ORDER BY added_at ASC",
            )?;
            let rows = stmt
                .query_map([], |r| {
                    Ok(ImportFolderRow {
                        path: r.get(0)?,
                        mode: r.get(1)?,
                        auto_scan: r.get::<_, i32>(2)? != 0,
                        last_scan: r.get(3)?,
                        track_count: r.get(4)?,
                        album_count: r.get(5)?,
                        artist_count: r.get(6)?,
                        accessible: r.get::<_, i32>(7)? != 0,
                        status: r.get(8)?,
                        added_at: r.get(9)?,
                    })
                })?
                .filter_map(Result::ok)
                .collect();
            Ok(rows)
        })
    }

    pub fn get_import_folder(&self, path: &str) -> Result<Option<ImportFolderRow>> {
        Ok(self.list_import_folders()?.into_iter().find(|f| f.path == path))
    }

    pub fn remove_import_folder(&self, path: &str) -> Result<bool> {
        self.with_conn(|conn| {
            let removed = conn.execute("DELETE FROM import_folders WHERE path = ?1", params![path])?;
            Ok(removed > 0)
        })
    }

    pub fn set_import_folder_auto_scan(&self, path: &str, enabled: bool) -> Result<bool> {
        self.with_conn(|conn| {
            let changed = conn.execute(
                "UPDATE import_folders SET auto_scan = ?1 WHERE path = ?2",
                params![enabled as i32, path],
            )?;
            Ok(changed > 0)
        })
    }

    pub fn update_import_folder_status(&self, path: &str, status: &str, accessible: bool) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE import_folders SET status = ?1, accessible = ?2 WHERE path = ?3",
                params![status, accessible as i32, path],
            )?;
            Ok(())
        })
    }

    // ---- playlists -------------------------------------------------------

    pub fn upsert_import_playlist(&self, row: &ImportPlaylistRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO imported_playlists
                    (playlist_id, source, name, thumbnail, track_count, imported_at)
                 VALUES (?1,?2,?3,?4,?5,?6)
                 ON CONFLICT(playlist_id) DO UPDATE SET
                    name=excluded.name,
                    thumbnail=excluded.thumbnail,
                    track_count=excluded.track_count,
                    imported_at=excluded.imported_at",
                params![
                    row.playlist_id,
                    row.source,
                    row.name,
                    row.thumbnail,
                    row.track_count,
                    if row.imported_at > 0 { row.imported_at } else { now() },
                ],
            )?;
            Ok(())
        })
    }

    pub fn replace_import_playlist_tracks(&self, playlist_id: &str, item_ids: &[String]) -> Result<()> {
        self.with_conn(|conn| {
            let tx = conn.unchecked_transaction()?;
            tx.execute(
                "DELETE FROM imported_playlist_tracks WHERE playlist_id = ?1",
                params![playlist_id],
            )?;
            {
                let mut stmt = tx.prepare(
                    "INSERT OR IGNORE INTO imported_playlist_tracks
                        (playlist_id, position, item_id) VALUES (?1, ?2, ?3)",
                )?;
                for (index, item_id) in item_ids.iter().enumerate() {
                    stmt.execute(params![playlist_id, index as i64, item_id])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    pub fn list_import_playlists(&self) -> Result<Vec<ImportPlaylistRow>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT playlist_id, source, name, thumbnail, track_count, imported_at
                 FROM imported_playlists ORDER BY imported_at DESC, name COLLATE NOCASE",
            )?;
            let rows = stmt
                .query_map([], |r| {
                    Ok(ImportPlaylistRow {
                        playlist_id: r.get(0)?,
                        source: r.get(1)?,
                        name: r.get(2)?,
                        thumbnail: r.get(3)?,
                        track_count: r.get(4)?,
                        imported_at: r.get(5)?,
                    })
                })?
                .filter_map(Result::ok)
                .collect();
            Ok(rows)
        })
    }

    pub fn list_import_playlist_items(&self, playlist_id: &str) -> Result<Vec<ImportItemRow>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT i.item_id, i.track_id, i.source, i.source_id, i.title, i.artist,
                        i.album, i.album_artist, i.duration_secs, i.thumbnail, i.isrc,
                        i.external_url, i.file_path, i.track_no, i.disc_no, i.year,
                        i.availability, i.match_status, i.match_confidence, i.file_size,
                        i.file_mtime, i.imported_at
                 FROM imported_playlist_tracks p
                 JOIN imported_items i ON i.item_id = p.item_id
                 WHERE p.playlist_id = ?1
                 ORDER BY p.position ASC",
            )?;
            let rows = stmt
                .query_map(params![playlist_id], map_item)?
                .filter_map(Result::ok)
                .collect();
            Ok(rows)
        })
    }

    pub fn delete_import_playlist(&self, playlist_id: &str) -> Result<bool> {
        self.with_conn(|conn| {
            let removed = conn.execute(
                "DELETE FROM imported_playlists WHERE playlist_id = ?1",
                params![playlist_id],
            )?;
            conn.execute(
                "DELETE FROM imported_playlist_tracks WHERE playlist_id = ?1",
                params![playlist_id],
            )?;
            Ok(removed > 0)
        })
    }

    // ---- provider accounts ----------------------------------------------

    pub fn upsert_provider_account(&self, row: &ProviderAccountRow) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO provider_accounts (provider, account_name, connected, updated_at)
                 VALUES (?1,?2,?3,?4)
                 ON CONFLICT(provider) DO UPDATE SET
                    account_name=excluded.account_name,
                    connected=excluded.connected,
                    updated_at=excluded.updated_at",
                params![row.provider, row.account_name, row.connected as i32, now()],
            )?;
            Ok(())
        })
    }

    pub fn get_provider_account(&self, provider: &str) -> Result<Option<ProviderAccountRow>> {
        self.with_conn(|conn| {
            let row = conn
                .query_row(
                    "SELECT provider, account_name, connected, updated_at
                     FROM provider_accounts WHERE provider = ?1",
                    params![provider],
                    |r| {
                        Ok(ProviderAccountRow {
                            provider: r.get(0)?,
                            account_name: r.get(1)?,
                            connected: r.get::<_, i32>(2)? != 0,
                            updated_at: r.get(3)?,
                        })
                    },
                )
                .optional()?;
            Ok(row)
        })
    }

    /// A lightweight fingerprint check used to keep a rescan from re-reading
    /// files that have not changed since the last one.
    pub fn import_file_unchanged(&self, path: &str, size: i64, mtime: i64) -> Result<bool> {
        self.with_conn(|conn| {
            let found: Option<(i64, i64)> = conn
                .query_row(
                    "SELECT file_size, file_mtime FROM imported_items
                     WHERE source = 'local' AND file_path = ?1 LIMIT 1",
                    params![path],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()?;
            Ok(matches!(found, Some((s, m)) if s == size && m == mtime))
        })
    }
}

fn map_item(r: &rusqlite::Row<'_>) -> rusqlite::Result<ImportItemRow> {
    Ok(ImportItemRow {
        item_id: r.get(0)?,
        track_id: r.get(1)?,
        source: r.get(2)?,
        source_id: r.get(3)?,
        title: r.get(4)?,
        artist: r.get(5)?,
        album: r.get(6)?,
        album_artist: r.get(7)?,
        duration_secs: r.get(8)?,
        thumbnail: r.get(9)?,
        isrc: r.get(10)?,
        external_url: r.get(11)?,
        file_path: r.get(12)?,
        track_no: r.get(13)?,
        disc_no: r.get(14)?,
        year: r.get(15)?,
        availability: r.get(16)?,
        match_status: r.get(17)?,
        match_confidence: r.get(18)?,
        file_size: r.get(19)?,
        file_mtime: r.get(20)?,
        imported_at: r.get(21)?,
    })
}
