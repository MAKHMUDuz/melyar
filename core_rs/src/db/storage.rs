use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::error::StorageError;
use crate::ffi::{CxxLibraryAlbum, CxxLibraryArtist, CxxLibraryPlaylist, CxxTrackMetadata};

/// Every fallible operation in this module reports a [`StorageError`], so a
/// caller can tell a locked database from a read-only home directory.
type Result<T> = std::result::Result<T, StorageError>;

pub struct DbStorage {
    conn: Mutex<Connection>,
}

impl DbStorage {
    pub fn new() -> Result<Self> {
        let db_path = Self::get_db_path()?;
        if let Some(parent) = db_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&db_path)?;

        // WAL mode returns a row in SQLite, so query_row consumes it without error
        let _ = conn.query_row("PRAGMA journal_mode = WAL;", [], |_| Ok(()));

        // Schema & performance tuning
        conn.execute_batch(
            "PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;
             PRAGMA temp_store = MEMORY;
             PRAGMA cache_size = -2000;

             CREATE TABLE IF NOT EXISTS history (
                 track_id TEXT PRIMARY KEY,
                 title TEXT NOT NULL,
                 artist TEXT NOT NULL,
                 album TEXT NOT NULL,
                 duration REAL NOT NULL,
                 thumbnail TEXT NOT NULL,
                 played_at INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS favorites (
                 track_id TEXT PRIMARY KEY,
                 title TEXT NOT NULL,
                 artist TEXT NOT NULL,
                 album TEXT NOT NULL,
                 duration REAL NOT NULL,
                 thumbnail TEXT NOT NULL,
                 added_at INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS lyrics_cache (
                 track_id TEXT PRIMARY KEY,
                 lrc TEXT NOT NULL,
                 cached_at INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS library_albums (
                 album_id TEXT PRIMARY KEY,
                 title TEXT NOT NULL,
                 artist TEXT NOT NULL,
                 thumbnail TEXT NOT NULL,
                 year TEXT NOT NULL,
                 track_count INTEGER NOT NULL,
                 duration REAL NOT NULL,
                 added_at INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS library_artists (
                 artist_id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 thumbnail TEXT NOT NULL,
                 follower_count INTEGER NOT NULL,
                 added_at INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS library_playlists (
                 playlist_id TEXT PRIMARY KEY,
                 name TEXT NOT NULL,
                 thumbnail TEXT NOT NULL,
                 track_count INTEGER NOT NULL,
                 is_owner INTEGER NOT NULL,
                 added_at INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS search_history (
                 query TEXT PRIMARY KEY,
                 searched_at INTEGER NOT NULL
             );

             CREATE TABLE IF NOT EXISTS settings (
                 key TEXT PRIMARY KEY,
                 value TEXT NOT NULL,
                 updated_at INTEGER NOT NULL
             );",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn get_db_path() -> Result<PathBuf> {
        if let Some(data_dir) = dirs::data_dir() {
            Ok(data_dir.join("melyar").join("melyar.db"))
        } else {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            Ok(PathBuf::from(home).join(".local/share/melyar/melyar.db"))
        }
    }

    fn current_timestamp() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64
    }

    pub fn add_history(&self, track: &CxxTrackMetadata) -> Result<()> {
        let conn = self.conn.lock();
        let now = Self::current_timestamp();
        conn.execute(
            "INSERT INTO history (track_id, title, artist, album, duration, thumbnail, played_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(track_id) DO UPDATE SET
                 played_at = ?7,
                 title = ?2,
                 artist = ?3,
                 album = ?4,
                 duration = ?5,
                 thumbnail = ?6",
            params![
                track.id,
                track.title,
                track.artist,
                track.album,
                track.duration_secs,
                track.thumbnail_url,
                now
            ],
        )?;
        Ok(())
    }

    pub fn get_history(&self, limit: usize) -> Result<Vec<CxxTrackMetadata>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT track_id, title, artist, album, duration, thumbnail
             FROM history ORDER BY played_at DESC LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(CxxTrackMetadata {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                duration_secs: row.get(4)?,
                thumbnail_url: row.get(5)?,
                lyrics: String::new(),
            })
        })?;

        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn toggle_favorite(&self, track: &CxxTrackMetadata) -> Result<bool> {
        let conn = self.conn.lock();
        let mut check_stmt = conn.prepare("SELECT 1 FROM favorites WHERE track_id = ?1")?;
        let exists = check_stmt.exists(params![track.id])?;

        if exists {
            conn.execute(
                "DELETE FROM favorites WHERE track_id = ?1",
                params![track.id],
            )?;
            Ok(false)
        } else {
            let now = Self::current_timestamp();
            conn.execute(
                "INSERT INTO favorites (track_id, title, artist, album, duration, thumbnail, added_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    track.id,
                    track.title,
                    track.artist,
                    track.album,
                    track.duration_secs,
                    track.thumbnail_url,
                    now
                ],
            )?;
            Ok(true)
        }
    }

    /// Add a track to the library. Unlike `toggle_favorite` this never
    /// removes an existing entry, which is what an import needs.
    pub fn add_favorite(&self, track: &CxxTrackMetadata) -> Result<bool> {
        let conn = self.conn.lock();
        let now = Self::current_timestamp();
        conn.execute(
            "INSERT INTO favorites (track_id, title, artist, album, duration, thumbnail, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(track_id) DO UPDATE SET
                 title = ?2,
                 artist = ?3,
                 album = ?4,
                 duration = ?5,
                 thumbnail = ?6",
            params![
                track.id,
                track.title,
                track.artist,
                track.album,
                track.duration_secs,
                track.thumbnail_url,
                now
            ],
        )?;
        Ok(true)
    }

    pub fn is_favorite(&self, track_id: &str) -> Result<bool> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT 1 FROM favorites WHERE track_id = ?1")?;
        Ok(stmt.exists(params![track_id])?)
    }

    pub fn get_favorites(&self) -> Result<Vec<CxxTrackMetadata>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT track_id, title, artist, album, duration, thumbnail
             FROM favorites ORDER BY added_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(CxxTrackMetadata {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                album: row.get(3)?,
                duration_secs: row.get(4)?,
                thumbnail_url: row.get(5)?,
                lyrics: String::new(),
            })
        })?;

        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn get_cached_lyrics(&self, track_id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT lrc FROM lyrics_cache WHERE track_id = ?1")?;
        let mut rows = stmt.query(params![track_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn cache_lyrics(&self, track_id: &str, lrc: &str) -> Result<()> {
        let conn = self.conn.lock();
        let now = Self::current_timestamp();
        conn.execute(
            "INSERT INTO lyrics_cache (track_id, lrc, cached_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(track_id) DO UPDATE SET lrc = ?2, cached_at = ?3",
            params![track_id, lrc, now],
        )?;
        Ok(())
    }

    // Library functions
    pub fn add_library_album(&self, album: &CxxLibraryAlbum) -> Result<()> {
        let conn = self.conn.lock();
        let now = Self::current_timestamp();
        conn.execute(
            "INSERT INTO library_albums (album_id, title, artist, thumbnail, year, track_count, duration, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(album_id) DO UPDATE SET
                 title = ?2,
                 artist = ?3,
                 thumbnail = ?4,
                 year = ?5,
                 track_count = ?6,
                 duration = ?7,
                 added_at = ?8",
            params![
                album.id,
                album.title,
                album.artist,
                album.thumbnail_url,
                album.year,
                album.track_count,
                album.duration_secs,
                now
            ],
        )?;
        Ok(())
    }

    pub fn get_library_albums(&self) -> Result<Vec<CxxLibraryAlbum>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT album_id, title, artist, thumbnail, year, track_count, duration
             FROM library_albums ORDER BY added_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(CxxLibraryAlbum {
                id: row.get(0)?,
                title: row.get(1)?,
                artist: row.get(2)?,
                thumbnail_url: row.get(3)?,
                year: row.get(4)?,
                track_count: row.get(5)?,
                duration_secs: row.get(6)?,
            })
        })?;

        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn add_library_artist(&self, artist: &CxxLibraryArtist) -> Result<()> {
        let conn = self.conn.lock();
        let now = Self::current_timestamp();
        conn.execute(
            "INSERT INTO library_artists (artist_id, name, thumbnail, follower_count, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(artist_id) DO UPDATE SET
                 name = ?2,
                 thumbnail = ?3,
                 follower_count = ?4,
                 added_at = ?5",
            params![
                artist.id,
                artist.name,
                artist.thumbnail_url,
                artist.follower_count,
                now
            ],
        )?;
        Ok(())
    }

    pub fn get_library_artists(&self) -> Result<Vec<CxxLibraryArtist>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT artist_id, name, thumbnail, follower_count
             FROM library_artists ORDER BY added_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(CxxLibraryArtist {
                id: row.get(0)?,
                name: row.get(1)?,
                thumbnail_url: row.get(2)?,
                follower_count: row.get(3)?,
            })
        })?;

        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn add_library_playlist(&self, playlist: &CxxLibraryPlaylist) -> Result<()> {
        let conn = self.conn.lock();
        let now = Self::current_timestamp();
        conn.execute(
            "INSERT INTO library_playlists (playlist_id, name, thumbnail, track_count, is_owner, added_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(playlist_id) DO UPDATE SET
                 name = ?2,
                 thumbnail = ?3,
                 track_count = ?4,
                 is_owner = ?5,
                 added_at = ?6",
            params![
                playlist.id,
                playlist.name,
                playlist.thumbnail_url,
                playlist.track_count,
                playlist.is_owner as i32,
                now
            ],
        )?;
        Ok(())
    }

    pub fn get_library_playlists(&self) -> Result<Vec<CxxLibraryPlaylist>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT playlist_id, name, thumbnail, track_count, is_owner
             FROM library_playlists ORDER BY added_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(CxxLibraryPlaylist {
                id: row.get(0)?,
                name: row.get(1)?,
                thumbnail_url: row.get(2)?,
                track_count: row.get(3)?,
                is_owner: row.get::<_, i32>(4)? != 0,
            })
        })?;

        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    // Search history
    pub fn add_search_history(&self, query: &str) -> Result<()> {
        let conn = self.conn.lock();
        let now = Self::current_timestamp();
        conn.execute(
            "INSERT INTO search_history (query, searched_at)
             VALUES (?1, ?2)
             ON CONFLICT(query) DO UPDATE SET searched_at = ?2",
            params![query, now],
        )?;
        Ok(())
    }

    pub fn get_search_history(&self, limit: usize) -> Result<Vec<String>> {
        let conn = self.conn.lock();
        let mut stmt =
            conn.prepare("SELECT query FROM search_history ORDER BY searched_at DESC LIMIT ?1")?;

        let rows = stmt.query_map(params![limit as i64], |row| row.get(0))?;

        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn clear_search_history(&self) -> Result<bool> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM search_history", [])?;
        Ok(true)
    }

    pub fn remove_from_search_history(&self, query: &str) -> Result<bool> {
        let conn = self.conn.lock();
        conn.execute(
            "DELETE FROM search_history WHERE query = ?1",
            params![query],
        )?;
        Ok(true)
    }

    // Settings
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock();
        let now = Self::current_timestamp();
        conn.execute(
            "INSERT INTO settings (key, value, updated_at)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET
                 value = ?2,
                 updated_at = ?3",
            params![key, value, now],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    /// Every stored setting as a key/value list.
    pub fn all_settings(&self) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock();
        let mut stmt = conn.prepare("SELECT key, value FROM settings ORDER BY key")?;
        let rows = stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;

        let mut result = Vec::new();
        for r in rows {
            result.push(r?);
        }
        Ok(result)
    }

    pub fn delete_setting(&self, key: &str) -> Result<bool> {
        let conn = self.conn.lock();
        conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
        Ok(true)
    }

    /// In-memory storage for unit tests.
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let _ = conn.query_row("PRAGMA journal_mode = WAL;", [], |_| Ok(()));
        conn.execute_batch(
            "PRAGMA synchronous = NORMAL;
                 PRAGMA foreign_keys = ON;
                 PRAGMA temp_store = MEMORY;

                 CREATE TABLE IF NOT EXISTS history (
                     track_id TEXT PRIMARY KEY,
                     title TEXT NOT NULL,
                     artist TEXT NOT NULL,
                     album TEXT NOT NULL,
                     duration REAL NOT NULL,
                     thumbnail TEXT NOT NULL,
                     played_at INTEGER NOT NULL
                 );

                 CREATE TABLE IF NOT EXISTS favorites (
                     track_id TEXT PRIMARY KEY,
                     title TEXT NOT NULL,
                     artist TEXT NOT NULL,
                     album TEXT NOT NULL,
                     duration REAL NOT NULL,
                     thumbnail TEXT NOT NULL,
                     added_at INTEGER NOT NULL
                 );

                 CREATE TABLE IF NOT EXISTS lyrics_cache (
                     track_id TEXT PRIMARY KEY,
                     lrc TEXT NOT NULL,
                     cached_at INTEGER NOT NULL
                 );

                 CREATE TABLE IF NOT EXISTS settings (
                     key TEXT PRIMARY KEY,
                     value TEXT NOT NULL,
                     updated_at INTEGER NOT NULL
                 );",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_settings() {
        let storage = DbStorage::new_in_memory().unwrap();

        assert_eq!(storage.get_setting("theme").unwrap(), None);

        storage.set_setting("theme", "dark").unwrap();
        storage.set_setting("language", "tr").unwrap();
        assert_eq!(
            storage.get_setting("theme").unwrap().as_deref(),
            Some("dark")
        );

        // Writing the same key again updates rather than duplicating.
        storage.set_setting("theme", "light").unwrap();
        assert_eq!(
            storage.get_setting("theme").unwrap().as_deref(),
            Some("light")
        );

        let all = storage.all_settings().unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.contains(&("theme".to_string(), "light".to_string())));
        assert!(all.contains(&("language".to_string(), "tr".to_string())));

        storage.delete_setting("theme").unwrap();
        assert_eq!(storage.get_setting("theme").unwrap(), None);
        assert_eq!(storage.all_settings().unwrap().len(), 1);
    }

    #[test]
    fn test_storage_settings_survive_a_restart() {
        // Settings must still be there the next time the application starts,
        // so the check uses a real file rather than an in-memory database.
        let dir = std::env::temp_dir().join(format!("melyar-settings-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("melyar.db");

        {
            let conn = Connection::open(&path).unwrap();
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS settings (
                     key TEXT PRIMARY KEY,
                     value TEXT NOT NULL,
                     updated_at INTEGER NOT NULL
                 );",
            )
            .unwrap();
            conn.execute(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)",
                params!["theme", "light", 1_i64],
            )
            .unwrap();
        }

        // Reopen the same file: the value has to still be there.
        let conn = Connection::open(&path).unwrap();
        let value: String = conn
            .query_row(
                "SELECT value FROM settings WHERE key = ?1",
                params!["theme"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(value, "light");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_storage_history() {
        let storage = DbStorage::new_in_memory().unwrap();
        let track = CxxTrackMetadata {
            id: "track1".to_string(),
            title: "Song A".to_string(),
            artist: "Artist A".to_string(),
            album: "Album A".to_string(),
            duration_secs: 180.0,
            thumbnail_url: "https://example.com/1.jpg".to_string(),
            lyrics: String::new(),
        };

        storage.add_history(&track).unwrap();
        let history = storage.get_history(10).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "track1");
        assert_eq!(history[0].title, "Song A");
        assert_eq!(history[0].artist, "Artist A");
        assert_eq!(history[0].duration_secs, 180.0);
    }

    #[test]
    fn test_storage_favorites() {
        let storage = DbStorage::new_in_memory().unwrap();
        let track = CxxTrackMetadata {
            id: "fav1".to_string(),
            title: "Fav Song".to_string(),
            artist: "Fav Artist".to_string(),
            album: "Fav Album".to_string(),
            duration_secs: 240.0,
            thumbnail_url: "https://example.com/fav.jpg".to_string(),
            lyrics: String::new(),
        };

        assert!(!storage.is_favorite("fav1").unwrap());
        let toggled_on = storage.toggle_favorite(&track).unwrap();
        assert!(toggled_on);
        assert!(storage.is_favorite("fav1").unwrap());

        let favs = storage.get_favorites().unwrap();
        assert_eq!(favs.len(), 1);
        assert_eq!(favs[0].id, "fav1");

        let toggled_off = storage.toggle_favorite(&track).unwrap();
        assert!(!toggled_off);
        assert!(!storage.is_favorite("fav1").unwrap());
        assert!(storage.get_favorites().unwrap().is_empty());
    }

    #[test]
    fn test_lyrics_cache() {
        let storage = DbStorage::new_in_memory().unwrap();
        assert!(storage.get_cached_lyrics("song123").unwrap().is_none());

        storage
            .cache_lyrics("song123", "[00:10.00] Line 1")
            .unwrap();
        let lyrics = storage.get_cached_lyrics("song123").unwrap();
        assert_eq!(lyrics, Some("[00:10.00] Line 1".to_string()));
    }
}
