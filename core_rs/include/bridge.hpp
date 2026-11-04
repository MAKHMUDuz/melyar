#pragma once
#include <cstdint>

#include "rust/cxx.h"

namespace melyar {
    struct CxxTrackMetadata;

    void on_progress_update(double current, double total);
    void on_track_changed(CxxTrackMetadata track);
    void on_state_changed(bool is_playing);
    void on_lyrics_loaded(rust::String track_id, rust::String lyrics_lrc);
    void on_search_results(rust::Vec<CxxTrackMetadata> results);
    void on_import_progress(int32_t current, int32_t total, int32_t matched, int32_t skipped);
    void on_import_event(rust::String event_json);
}
