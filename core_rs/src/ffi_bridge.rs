use crate::{
    append_to_queue, cancel_import, clear_queue, clear_search_history, connect_account,
    delete_setting, disconnect_account, fetch_lyrics, fetch_lyrics_live, get_favorites,
    get_history, get_library_albums, get_library_artists, get_library_playlists, get_queue,
    get_settings_json, get_track_metadata, import_account, import_cancel, import_center_state,
    import_commit, import_delete_item, import_library, import_list_items, import_list_playlists,
    import_pick_candidate, import_playlist_items, import_preview_file, import_remove_folder,
    import_rescan_folder, import_review_item, import_scan_folder, import_set_folder_auto_scan,
    import_spotify_connect, import_spotify_disconnect, import_spotify_preview,
    import_spotify_set_client_id, import_spotify_status, init_melyar_core, insert_next_in_queue,
    is_favorite, list_accounts_json, list_sources_json, next_track, play_track, previous_track,
    remove_from_queue, remove_from_search_history, replace_queue, search, seek,
    set_current_queue_index, set_equalizer_bands, set_repeat_mode, set_setting, set_shuffle,
    set_source_enabled, set_volume, toggle_favorite, toggle_play,
};

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
        fn on_import_event(event_json: String);
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
        fn list_accounts_json() -> String;
        fn connect_account(provider_id: String) -> bool;
        fn disconnect_account(provider_id: String) -> bool;
        fn import_account(provider_id: String, options_json: String) -> bool;
        fn import_center_state() -> String;
        fn import_list_items() -> String;
        fn import_playlist_items(playlist_id: String) -> String;
        fn import_list_playlists() -> String;
        fn import_scan_folder(path: String, options_json: String) -> bool;
        fn import_preview_file(path: String) -> String;
        fn import_commit(options_json: String) -> bool;
        fn import_cancel() -> bool;
        fn import_rescan_folder(path: String) -> bool;
        fn import_remove_folder(path: String) -> bool;
        fn import_set_folder_auto_scan(path: String, enabled: bool) -> bool;
        fn import_delete_item(item_id: String) -> bool;
        fn import_review_item(item_id: String, candidate_id: String) -> bool;
        fn import_pick_candidate(preview_id: String, item_id: String, candidate_id: String)
            -> bool;
        fn import_spotify_status() -> String;
        fn import_spotify_set_client_id(id: String) -> bool;
        fn import_spotify_connect() -> bool;
        fn import_spotify_disconnect() -> bool;
        fn import_spotify_preview(options_json: String) -> bool;
    }
}
