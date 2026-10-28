pub mod audio;
pub mod db;
pub mod import;
pub mod innertube;
pub mod linux;
pub mod source;

use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::runtime::Runtime;

use crate::audio::engine::{AudioEventHandler, MelyarAudioEngine};
use crate::audio::queue::TrackQueue;
use crate::db::storage::DbStorage;
use crate::innertube::client::InnerTubeClient;
use crate::linux::mpris::{MprisCommandHandler, MprisManager};
use crate::linux::waybar::{WaybarCommandHandler, WaybarIpc};

#[cxx::bridge(namespace = "melyar")]
pub mod ffi {
    #[derive(Clone, Debug, Default)]
    pub struct CxxTrackMetadata {
        pub id: String,
        pub title: String,
        pub artist: String,
        pub album: String,
        pub duration_secs: f64,
        pub thumbnail_url: String,
        pub lyrics: String,
    }

    #[derive(Clone, Debug, Default)]
    pub struct CxxLibraryAlbum {
        pub id: String,
        pub title: String,
        pub artist: String,
        pub thumbnail_url: String,
        pub year: String,
        pub track_count: i32,
        pub duration_secs: f64,
    }

    #[derive(Clone, Debug, Default)]
    pub struct CxxLibraryArtist {
        pub id: String,
        pub name: String,
        pub thumbnail_url: String,
        pub follower_count: i64,
    }

    #[derive(Clone, Debug, Default)]
    pub struct CxxLibraryPlaylist {
        pub id: String,
        pub name: String,
        pub thumbnail_url: String,
        pub track_count: i32,
        pub is_owner: bool,
    }

    unsafe extern "C++" {
        include!("bridge.hpp");

        fn on_progress_update(current: f64, total: f64);
        fn on_track_changed(track: CxxTrackMetadata);
        fn on_state_changed(is_playing: bool);
        fn on_lyrics_loaded(track_id: String, lyrics_lrc: String);
        fn on_search_results(results: Vec<CxxTrackMetadata>);
        fn on_import_progress(current: i32, total: i32, matched: i32, skipped: i32);
    }

    extern "Rust" {
        fn init_melyar_core() -> bool;
        fn search(query: String) -> Vec<CxxTrackMetadata>;
        fn play_track(track_id: String) -> bool;
        fn toggle_play() -> bool;
        fn seek(seconds: f64) -> bool;
        fn set_volume(volume: f64) -> bool;
        fn next_track() -> bool;
        fn previous_track() -> bool;
        fn fetch_lyrics(track_id: String) -> String;
        fn fetch_lyrics_live(
            track_id: String,
            title: String,
            artist: String,
            duration: f64,
        ) -> String;
        fn toggle_favorite(
            track_id: String,
            title: String,
            artist: String,
            album: String,
            duration: f64,
            thumb: String,
        ) -> bool;
        fn is_favorite(track_id: String) -> bool;
        fn get_favorites() -> Vec<CxxTrackMetadata>;
        fn get_history() -> Vec<CxxTrackMetadata>;
        fn set_shuffle(enabled: bool) -> bool;
        fn set_repeat_mode(mode: i32) -> bool;
        fn replace_queue(tracks: Vec<CxxTrackMetadata>, start_index: i32) -> bool;
        fn get_queue() -> Vec<CxxTrackMetadata>;
        fn append_to_queue(track: CxxTrackMetadata) -> bool;
        fn insert_next_in_queue(track: CxxTrackMetadata) -> bool;
        fn remove_from_queue(index: i32) -> bool;
        fn clear_queue() -> bool;
        fn set_current_queue_index(index: i32) -> bool;
        fn get_track_metadata(track_id: String) -> CxxTrackMetadata;
        fn get_library_albums() -> Vec<CxxLibraryAlbum>;
        fn get_library_artists() -> Vec<CxxLibraryArtist>;
        fn get_library_playlists() -> Vec<CxxLibraryPlaylist>;
        fn clear_search_history() -> bool;
        fn remove_from_search_history(query: String) -> bool;
        fn get_settings_json() -> String;
        fn set_setting(key: String, value: String) -> bool;
        fn delete_setting(key: String) -> bool;
        fn import_library(path: String) -> bool;
        fn cancel_import() -> bool;
        fn set_equalizer_bands(bands: Vec<f64>) -> bool;
        fn list_sources_json() -> String;
        fn set_source_enabled(source_id: String, enabled: bool) -> bool;
    }
}

pub struct MelyarCore {
    innertube: Arc<InnerTubeClient>,
    /// Where music can come from. YouTube is one of these, not a special case,
    /// so a track id from anywhere resolves the same way.
    sources: Arc<crate::source::SourceRegistry>,
    queue: Arc<TrackQueue>,
    storage: Arc<DbStorage>,
    audio: Mutex<Option<Arc<MelyarAudioEngine>>>,
    /// Keeps the MPRIS service alive, and answers the desktop when it asks
    /// what is playing.
    mpris: Mutex<Option<Arc<MprisManager>>>,
    _waybar: Mutex<Option<Arc<WaybarIpc>>>,
    runtime: Arc<Runtime>,
}

struct EngineEvents {
    queue: Arc<TrackQueue>,
    /// Stream resolution goes through `sources` — it is the one seam that knows
    /// which site a track id belongs to. The innertube client itself is no
    /// longer read here; it lives on the registry's YouTube source.
    sources: Arc<crate::source::SourceRegistry>,
    audio_holder: Mutex<Option<Arc<MelyarAudioEngine>>>,
    mpris_holder: Mutex<Option<Arc<MprisManager>>>,
    waybar_holder: Mutex<Option<Arc<WaybarIpc>>>,
    storage: Arc<DbStorage>,
    runtime: Arc<Runtime>,
}

impl AudioEventHandler for EngineEvents {
    fn on_progress(&self, current_secs: f64, total_secs: f64) {
        if let Some(ref mpris) = *self.mpris_holder.lock() {
            mpris.update_position(current_secs);
        }
        if let Some(ref waybar) = *self.waybar_holder.lock() {
            waybar.update_position(current_secs);
        }
        ffi::on_progress_update(current_secs, total_secs);
    }

    fn on_state_changed(&self, is_playing: bool) {
        if let Some(ref mpris) = *self.mpris_holder.lock() {
            mpris.update_playback_state(is_playing);
        }
        if let Some(ref waybar) = *self.waybar_holder.lock() {
            waybar.update_playback_state(is_playing);
        }
        ffi::on_state_changed(is_playing);
    }

    fn on_track_ended(&self) {
        let audio_opt = self.audio_holder.lock().clone();
        // Gapless already appended the next file and mpv started it on its own.
        // Replacing it here would restart it from zero, and resolve it twice.
        let already_started = audio_opt
            .as_ref()
            .map(|audio| audio.take_prefetched())
            .unwrap_or(false);

        let Some(next) = self.queue.next_track() else {
            // The queue is empty. mpv ran the last file out and goes idle
            // without ever changing `pause`, so nothing corrects the engine's
            // own flag - it keeps saying `true` while nothing is audible, and
            // `toggle_play` answers the next press from that stale value.
            if let Some(audio) = audio_opt.as_ref() {
                audio.mark_stopped();
            }
            ffi::on_state_changed(false);
            return;
        };

        let sources = self.sources.clone();
        let storage = self.storage.clone();
        let mpris_opt = self.mpris_holder.lock().clone();
        let waybar_opt = self.waybar_holder.lock().clone();
        let rt = self.runtime.clone();

        rt.spawn(async move {
            let mut started = false;
            if already_started {
                started = true;
            } else {
                // The old file has run out, but mpv never reports a `pause`
                // change for running dry - so this flag is the only thing that
                // still believes something is playing. Dropping it before the
                // resolve means every failure below leaves the desktop on
                // "stopped" instead of claiming a track that never started,
                // and `play_url` raises it again on the one path that works.
                if let Some(audio) = &audio_opt {
                    audio.mark_stopped();
                }
                // The next track carries its own id prefix, so a SoundCloud or
                // Audius track keeps being itself across a track boundary
                // instead of being reinterpreted as a YouTube video id.
                match sources.resolve_stream(&next.id).await {
                    Ok(url) => {
                        if let Some(audio) = audio_opt {
                            match audio.play_url(&url) {
                                Ok(()) => started = true,
                                Err(err) => {
                                    // The index has already moved. Saying so beats leaving the
                                    // desktop on "Playing" with nothing audible.
                                    tracing::error!("failed to start {}: {err}", next.id);
                                    ffi::on_state_changed(false);
                                }
                            }
                        }
                    }
                    Err(err) => {
                        tracing::error!("no stream for {}: {err}", next.id);
                        ffi::on_state_changed(false);
                    }
                }
            }

            if !started {
                return;
            }

            let _ = storage.add_history(&next);
            if let Some(ref mpris) = mpris_opt {
                mpris.update_track(Some(next.clone()));
            }
            if let Some(ref waybar) = waybar_opt {
                waybar.update_track(Some(next.clone()));
            }
            ffi::on_track_changed(next.clone());

            if let Ok(lrc) = innertube::lyrics::fetch_lyrics(
                innertube::lyrics::lyrics_client(),
                &next.title,
                &next.artist,
                Some(next.duration_secs),
            )
            .await
            {
                let _ = storage.cache_lyrics(&next.id, &lrc);
                ffi::on_lyrics_loaded(next.id.clone(), lrc);
            }
        });
    }

    fn on_gapless_prefetch_needed(&self) {
        if let Some(next) = self.queue.peek_next_track() {
            let sources = self.sources.clone();
            let audio_opt = self.audio_holder.lock().clone();
            let rt = self.runtime.clone();

            rt.spawn(async move {
                // Prefetch resolves through the same seam as playback, so a
                // queued SoundCloud track is warmed with its own stream rather
                // than a YouTube one that would fail at the boundary.
                if let Ok(url) = sources.resolve_stream(&next.id).await {
                    if let Some(audio) = audio_opt {
                        let _ = audio.queue_next_url(&url);
                    }
                }
            });
        }
    }
}

#[allow(dead_code)]
struct MprisHandlerImpl {
    queue: Arc<TrackQueue>,
    innertube: Arc<InnerTubeClient>,
    audio_holder: Arc<Mutex<Option<Arc<MelyarAudioEngine>>>>,
    storage: Arc<DbStorage>,
    runtime: Arc<Runtime>,
}

impl MprisCommandHandler for MprisHandlerImpl {
    fn on_play_pause(&self) {
        let _ = toggle_play();
    }
    fn on_play(&self) {
        if let Some(ref audio) = *self.audio_holder.lock() {
            if !audio.is_playing() {
                let _ = audio.toggle_play();
            }
        }
    }
    fn on_pause(&self) {
        if let Some(ref audio) = *self.audio_holder.lock() {
            if audio.is_playing() {
                let _ = audio.toggle_play();
            }
        }
    }
    fn on_stop(&self) {
        if let Some(ref audio) = *self.audio_holder.lock() {
            let _ = audio.stop();
        }
    }
    fn on_next(&self) {
        let _ = next_track();
    }
    fn on_previous(&self) {
        let _ = previous_track();
    }
    fn on_seek_absolute(&self, seconds: f64) {
        let _ = seek(seconds);
    }
    fn on_set_volume(&self, volume: f64) {
        let _ = set_volume(volume * 100.0);
    }
    fn shuffle(&self) -> bool {
        self.queue.shuffle()
    }
    fn repeat_mode(&self) -> i32 {
        self.queue.repeat_mode()
    }
}

impl WaybarCommandHandler for MprisHandlerImpl {
    fn on_toggle(&self) {
        let _ = toggle_play();
    }
    fn on_next(&self) {
        let _ = next_track();
    }
    fn on_prev(&self) {
        let _ = previous_track();
    }
    fn on_stop(&self) {
        if let Some(ref audio) = *self.audio_holder.lock() {
            let _ = audio.stop();
        }
    }
}

static CORE: OnceCell<Arc<MelyarCore>> = OnceCell::new();

fn get_core() -> &'static Arc<MelyarCore> {
    CORE.get_or_init(|| {
        let runtime = Arc::new(Runtime::new().expect("Failed to create tokio runtime"));
        let innertube =
            Arc::new(InnerTubeClient::new().expect("Failed to initialize InnerTube client"));
        let queue = Arc::new(TrackQueue::new());
        let storage = Arc::new(DbStorage::new().expect("Failed to initialize SQLite storage"));

        // The user's source choices survive restarts as a comma list. Anything
        // unparseable reads as "just YouTube", which is the safe default.
        let saved_sources = storage
            .get_setting("enabledSources")
            .ok()
            .flatten()
            .unwrap_or_default();
        let sources = Arc::new(crate::source::SourceRegistry::new(
            innertube.clone(),
            &saved_sources,
        ));

        let events = Arc::new(EngineEvents {
            queue: queue.clone(),
            sources: sources.clone(),
            audio_holder: Mutex::new(None),
            mpris_holder: Mutex::new(None),
            waybar_holder: Mutex::new(None),
            storage: storage.clone(),
            runtime: runtime.clone(),
        });

        let audio = match MelyarAudioEngine::new(events.clone()) {
            Ok(eng) => {
                let arc_eng = Arc::new(eng);
                *events.audio_holder.lock() = Some(arc_eng.clone());
                Some(arc_eng)
            }
            Err(e) => {
                eprintln!("Failed to initialize audio engine: {}", e);
                None
            }
        };

        let audio_holder_for_mpris = Arc::new(Mutex::new(audio.clone()));
        let mpris_handler = Arc::new(MprisHandlerImpl {
            queue: queue.clone(),
            innertube: innertube.clone(),
            audio_holder: audio_holder_for_mpris,
            storage: storage.clone(),
            runtime: runtime.clone(),
        });

        let mpris = Arc::new(MprisManager::new(mpris_handler.clone(), runtime.clone()));
        *events.mpris_holder.lock() = Some(mpris.clone());

        let waybar = Arc::new(WaybarIpc::new(mpris_handler, runtime.clone()));
        *events.waybar_holder.lock() = Some(waybar.clone());

        Arc::new(MelyarCore {
            innertube,
            sources,
            queue,
            storage,
            audio: Mutex::new(audio),
            mpris: Mutex::new(Some(mpris)),
            _waybar: Mutex::new(Some(waybar)),
            runtime,
        })
    })
}

pub fn init_melyar_core() -> bool {
    let _ = get_core();
    true
}

pub fn search(query: String) -> Vec<ffi::CxxTrackMetadata> {
    let core = get_core();
    let sources = core.sources.clone();
    let q_clone = query.clone();

    // Every enabled source answers at once; the wait is the slowest one, not the
    // sum. With only YouTube on — the default — this is a single future, the
    // same call the search box always made.
    let tracks = core
        .runtime
        .block_on(async move { sources.search_all(&q_clone).await });

    // Deliberately not pushed into the queue. Typing in the search box is not
    // a request to replace what is playing: auto-advance and the media keys
    // follow the queue, so pushing here turned a search into a queue reset.
    // The UI pushes a queue on purpose through replace_queue().
    ffi::on_search_results(tracks.clone());
    tracks
}

pub fn play_track(track_id: String) -> bool {
    let core = get_core();
    let innertube = core.innertube.clone();
    let sources = core.sources.clone();
    let audio_opt = core.audio.lock().clone();
    let storage = core.storage.clone();
    let mpris_opt = core.mpris.lock().clone();
    let waybar_opt = core._waybar.lock().clone();
    let queue = core.queue.clone();
    let rt = core.runtime.clone();

    let current_meta = queue.list().into_iter().find(|t| t.id == track_id);
    let track_id_clone = track_id.clone();

    rt.spawn(async move {
        // Metadata: queue lookup first; otherwise resolve once. Only a bare
        // YouTube id is handed to yt-dlp — an `audius:`/`soundcloud:` id would
        // be nonsense there, and those sources already returned full metadata
        // with their search results.
        let meta = match current_meta {
            Some(m) => m,
            None if sources.source_for_id(&track_id_clone).1 == track_id_clone => {
                let innertube_meta = innertube.clone();
                let tid = track_id_clone.clone();
                match tokio::task::spawn_blocking(move || {
                    innertube::client::with_ytdlp_lock(|| {
                        innertube_meta.resolve_video_blocking(&tid)
                    })
                })
                .await
                {
                    Ok(Ok(resolved)) => resolved.to_meta(),
                    Ok(Err(e)) => {
                        eprintln!("Metadata resolve failed for {}: {}", track_id_clone, e);
                        innertube::client::ResolvedVideo::default().to_meta()
                    }
                    Err(e) => {
                        eprintln!("Metadata resolve task failed for {}: {}", track_id_clone, e);
                        innertube::client::ResolvedVideo::default().to_meta()
                    }
                }
            }
            // A prefixed id played straight from a deep link, with no queue to
            // read from. The id is all we know; a title can wait until the
            // source's own metadata arrives.
            None => ffi::CxxTrackMetadata {
                id: track_id_clone.clone(),
                ..Default::default()
            },
        };

        match sources.resolve_stream(&track_id_clone).await {
            Ok(url) => {
                if let Some(audio) = audio_opt {
                    if let Err(e) = audio.play_url(&url) {
                        eprintln!("Error playing stream URL: {}", e);
                    } else {
                        let _ = storage.add_history(&meta);
                        if let Some(ref mpris) = mpris_opt {
                            mpris.update_track(Some(meta.clone()));
                        }
                        if let Some(ref waybar) = waybar_opt {
                            waybar.update_track(Some(meta.clone()));
                        }
                        ffi::on_track_changed(meta.clone());

                        let title = meta.title.clone();
                        let artist = meta.artist.clone();
                        let dur = meta.duration_secs;
                        let tid = meta.id.clone();
                        let st = storage.clone();

                        tokio::spawn(async move {
                            if let Ok(Some(cached)) = st.get_cached_lyrics(&tid) {
                                ffi::on_lyrics_loaded(tid, cached);
                            } else if let Ok(lrc) = innertube::lyrics::fetch_lyrics(
                                innertube::lyrics::lyrics_client(),
                                &title,
                                &artist,
                                Some(dur),
                            )
                            .await
                            {
                                let _ = st.cache_lyrics(&tid, &lrc);
                                ffi::on_lyrics_loaded(tid, lrc);
                            }
                        });
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to get stream url for {}: {}", track_id_clone, e);
            }
        }
    });

    true
}

pub fn toggle_play() -> bool {
    let core = get_core();
    if let Some(ref audio) = *core.audio.lock() {
        audio.toggle_play().unwrap_or(false)
    } else {
        false
    }
}

pub fn seek(seconds: f64) -> bool {
    let core = get_core();
    if let Some(ref audio) = *core.audio.lock() {
        audio.seek(seconds).is_ok()
    } else {
        false
    }
}

pub fn set_volume(volume: f64) -> bool {
    let core = get_core();
    let ok = if let Some(ref audio) = *core.audio.lock() {
        audio.set_volume(volume).is_ok()
    } else {
        false
    };
    // The desktop's own volume slider has to move too, and it speaks 0-1.
    if let Some(ref mpris) = *core.mpris.lock() {
        mpris.update_volume(volume / 100.0);
    }
    ok
}

pub fn next_track() -> bool {
    let core = get_core();
    if let Some(next) = core.queue.next_track() {
        play_track(next.id)
    } else {
        false
    }
}

pub fn previous_track() -> bool {
    let core = get_core();
    if let Some(prev) = core.queue.previous_track() {
        play_track(prev.id)
    } else {
        false
    }
}

pub fn fetch_lyrics(track_id: String) -> String {
    let core = get_core();
    if let Ok(Some(cached)) = core.storage.get_cached_lyrics(&track_id) {
        return cached;
    }
    String::new()
}

/// Cache-first, then live lrclib fetch. Called from a Qt worker thread.
pub fn fetch_lyrics_live(track_id: String, title: String, artist: String, duration: f64) -> String {
    let core = get_core();
    if let Ok(Some(cached)) = core.storage.get_cached_lyrics(&track_id) {
        if !cached.trim().is_empty() {
            return cached;
        }
    }
    if title.trim().is_empty() {
        return String::new();
    }
    let res = core.runtime.block_on(async {
        innertube::lyrics::fetch_lyrics(
            innertube::lyrics::lyrics_client(),
            &title,
            &artist,
            Some(duration),
        )
        .await
    });
    match res {
        Ok(lrc) => {
            if !lrc.trim().is_empty() {
                let _ = core.storage.cache_lyrics(&track_id, &lrc);
            }
            lrc
        }
        Err(_) => String::new(),
    }
}

pub fn toggle_favorite(
    track_id: String,
    title: String,
    artist: String,
    album: String,
    duration: f64,
    thumb: String,
) -> bool {
    let core = get_core();
    let track = ffi::CxxTrackMetadata {
        id: track_id,
        title,
        artist,
        album,
        duration_secs: duration,
        thumbnail_url: thumb,
        lyrics: String::new(),
    };
    core.storage.toggle_favorite(&track).unwrap_or(false)
}

pub fn is_favorite(track_id: String) -> bool {
    let core = get_core();
    core.storage.is_favorite(&track_id).unwrap_or(false)
}

pub fn get_favorites() -> Vec<ffi::CxxTrackMetadata> {
    let core = get_core();
    core.storage.get_favorites().unwrap_or_default()
}

pub fn get_history() -> Vec<ffi::CxxTrackMetadata> {
    let core = get_core();
    core.storage.get_history(100).unwrap_or_default()
}

pub fn set_shuffle(enabled: bool) -> bool {
    let core = get_core();
    core.queue.set_shuffle(enabled);
    if let Some(ref mpris) = *core.mpris.lock() {
        mpris.update_queue_modes();
    }
    true
}

pub fn set_repeat_mode(mode: i32) -> bool {
    let core = get_core();
    core.queue.set_repeat_mode(mode);
    if let Some(ref mpris) = *core.mpris.lock() {
        mpris.update_queue_modes();
    }
    true
}

/// Web UI sends its playback context (a list/album/search page) so that
/// media keys, MPRIS and auto-advance all follow the same queue.
pub fn replace_queue(tracks: Vec<ffi::CxxTrackMetadata>, start_index: i32) -> bool {
    let core = get_core();
    core.queue.set_queue(tracks, start_index.max(0) as usize);
    true
}

pub fn get_queue() -> Vec<ffi::CxxTrackMetadata> {
    let core = get_core();
    core.queue.list()
}

/// The interface used to keep its own copy of the queue for adding, removing
/// and clearing, and never told the core about any of it. What the panel
/// showed and what actually played were two different lists: a track you
/// queued was drawn and skipped, and one you removed came back at the end of
/// the album. All three now land here, and the interface reads the list back.
pub fn append_to_queue(track: ffi::CxxTrackMetadata) -> bool {
    get_core().queue.append(track);
    true
}

pub fn insert_next_in_queue(track: ffi::CxxTrackMetadata) -> bool {
    get_core().queue.insert_next(track);
    true
}

pub fn remove_from_queue(index: i32) -> bool {
    if index < 0 {
        return false;
    }
    get_core().queue.remove(index as usize)
}

pub fn clear_queue() -> bool {
    get_core().queue.clear();
    true
}

pub fn set_current_queue_index(index: i32) -> bool {
    if index < 0 {
        return false;
    }
    get_core().queue.set_current_index(index as usize)
}

pub fn get_track_metadata(track_id: String) -> ffi::CxxTrackMetadata {
    let core = get_core();
    let queue = core.queue.list();
    if let Some(m) = queue.into_iter().find(|t| t.id == track_id) {
        return m;
    }
    // Fallback: resolve metadata via yt-dlp (blocking; caller runs on a worker thread).
    match innertube::client::with_ytdlp_lock(|| core.innertube.resolve_video_blocking(&track_id)) {
        Ok(resolved) => resolved.to_meta(),
        Err(_) => ffi::CxxTrackMetadata::default(),
    }
}

pub fn get_library_albums() -> Vec<ffi::CxxLibraryAlbum> {
    let core = get_core();
    core.storage.get_library_albums().unwrap_or_default()
}

pub fn get_library_artists() -> Vec<ffi::CxxLibraryArtist> {
    let core = get_core();
    core.storage.get_library_artists().unwrap_or_default()
}

pub fn get_library_playlists() -> Vec<ffi::CxxLibraryPlaylist> {
    let core = get_core();
    core.storage.get_library_playlists().unwrap_or_default()
}

pub fn clear_search_history() -> bool {
    let core = get_core();
    core.storage.clear_search_history().unwrap_or(false)
}

pub fn remove_from_search_history(query: String) -> bool {
    let core = get_core();
    core.storage
        .remove_from_search_history(&query)
        .unwrap_or(false)
}

/// Every stored setting as a JSON object, so the bridge can hand the whole
/// set to the interface in one call.
pub fn get_settings_json() -> String {
    let core = get_core();
    let mut map = serde_json::Map::new();
    if let Ok(entries) = core.storage.all_settings() {
        for (key, value) in entries {
            map.insert(key, serde_json::Value::String(value));
        }
    }
    serde_json::Value::Object(map).to_string()
}

pub fn set_setting(key: String, value: String) -> bool {
    let core = get_core();
    core.storage.set_setting(&key, &value).is_ok()
}

/// The sources the plugins panel lists, as one JSON array.
///
/// Everything the panel needs to draw a row — name, what it is made of,
/// whether it is on — comes from here, so the panel itself holds no list of
/// sites to keep in sync.
pub fn list_sources_json() -> String {
    let core = get_core();
    let infos = core.sources.info();
    let arr: Vec<serde_json::Value> = infos.iter().map(SourceInfoJson::from_info).collect();
    serde_json::Value::Array(arr).to_string()
}

/// Turn one source on or off. The choice is written straight to settings, so
/// it survives a restart without the panel having to remember it.
pub fn set_source_enabled(source_id: String, enabled: bool) -> bool {
    let core = get_core();
    if !core.sources.set_enabled(&source_id, enabled) {
        // An unknown id, or an attempt to switch YouTube off. Either way the
        // registry refused and there is nothing worth writing.
        return false;
    }
    let _ = core
        .storage
        .set_setting("enabledSources", &core.sources.enabled_serialized());
    true
}

/// Small adapter so the JSON shape is written in one place rather than being
/// spelled out inside the registry, which does not depend on serde derives.
struct SourceInfoJson;

impl SourceInfoJson {
    fn from_info(info: &crate::source::SourceInfo) -> serde_json::Value {
        serde_json::json!({
            "id": info.id,
            "name": info.display_name,
            "description": info.description,
            "kind": info.kind,
            "enabled": info.enabled,
            "default": info.default_enabled,
        })
    }
}

pub fn delete_setting(key: String) -> bool {
    let core = get_core();
    core.storage.delete_setting(&key).unwrap_or(false)
}

/// A flag the import loop checks so it can be stopped.
static IMPORT_CANCELLED: AtomicBool = AtomicBool::new(false);

/// Import a library from another service's export file.
///
/// The file is read, every entry is resolved against InnerTube and the
/// matches are added to the library. The work happens on the runtime so the
/// interface stays responsive, and the progress is reported back through
/// `on_import_progress`.
pub fn import_library(path: String) -> bool {
    let core = get_core();
    IMPORT_CANCELLED.store(false, Ordering::Relaxed);

    let source = PathBuf::from(path);
    let plan = match import::read_import_file(&source) {
        Ok(plan) => plan,
        Err(e) => {
            eprintln!("Library import failed: {e}");
            ffi::on_import_progress(0, 0, 0, 0);
            return false;
        }
    };

    let total = plan.entries.len() as i32;
    let innertube = core.innertube.clone();
    let storage = core.storage.clone();
    let runtime = core.runtime.clone();

    runtime.spawn(async move {
        let mut matched = 0i32;
        let mut skipped = 0i32;

        for (index, entry) in plan.entries.iter().enumerate() {
            if IMPORT_CANCELLED.load(Ordering::Relaxed) {
                break;
            }

            let current = (index + 1) as i32;
            let query = if entry.artist.is_empty() {
                entry.title.clone()
            } else {
                format!("{} {}", entry.title, entry.artist)
            };

            let found = innertube.search_tracks(&query).await.unwrap_or_default();

            let mut best: Option<(i32, crate::ffi::CxxTrackMetadata)> = None;
            for candidate in &found {
                let scored = import::score_match(
                    entry,
                    &candidate.title,
                    &candidate.artist,
                    &candidate.album,
                );
                if scored.score >= import::MATCH_THRESHOLD {
                    let better = best
                        .as_ref()
                        .map(|(s, _)| scored.score > *s)
                        .unwrap_or(true);
                    if better {
                        best = Some((scored.score, candidate.clone()));
                    }
                }
            }

            match best {
                Some((_, track)) => {
                    let _ = storage.add_favorite(&track);
                    matched += 1;
                }
                None => {
                    skipped += 1;
                }
            }

            ffi::on_import_progress(current, total, matched, skipped);
        }

        println!("Library import finished: {matched} matched, {skipped} not found");
        ffi::on_import_progress(total, total, matched, skipped);
    });

    true
}

pub fn cancel_import() -> bool {
    IMPORT_CANCELLED.store(true, Ordering::Relaxed);
    true
}

/// Set the gains of the graphic equaliser, in decibels. An empty list turns
/// the equaliser off.
pub fn set_equalizer_bands(bands: Vec<f64>) -> bool {
    let core = get_core();
    let audio = core.audio.lock().clone();
    match audio {
        Some(engine) => match engine.set_equalizer_bands(&bands) {
            Ok(()) => true,
            Err(e) => {
                // The error would otherwise be swallowed, and a filter that
                // mpv refused would look like it had been applied.
                eprintln!("Could not set the equaliser: {e}");
                false
            }
        },
        None => false,
    }
}
