#include "bridge.hpp"

#include <QMetaObject>
#include <QtConcurrent/QtConcurrent>
#include <QUrl>
#include <iostream>
#include <QTimer>

#include "melyar_core/src/lib.rs.h"

MelyarBridge* MelyarBridge::s_instance = nullptr;

MelyarBridge* MelyarBridge::instance() {
    return s_instance;
}

MelyarBridge::MelyarBridge(QObject *parent) : QObject(parent) {
    s_instance = this;
    melyar::init_melyar_core();

    // One timer the bridge owns, so it can be stopped. Arming a bare
    // singleShot left it unreachable: cancel cleared the label but the
    // deadline still fired and paused playback anyway, and setting the timer
    // again stacked a second one.
    m_sleepTimer = new QTimer(this);
    m_sleepTimer->setSingleShot(true);
    connect(m_sleepTimer, &QTimer::timeout, this, [this]() {
        if (!m_sleepTimerActive) return;
        togglePlay();
        m_sleepTimerActive = false;
        m_sleepTimerText.clear();
        emit sleepTimerChanged();
    });

    loadFavorites();
    loadHistory();
    loadLibrary();
}

MelyarBridge::~MelyarBridge() {
    if (s_instance == this) {
        s_instance = nullptr;
    }
}

void MelyarBridge::search(const QString &query) {
    if (query.trimmed().isEmpty()) {
        return;
    }

    // Run async in background thread pool to ensure 0ms UI frame drops
    (void)QtConcurrent::run([this, query]() {
        rust::String r_query(query.toStdString());
        auto results = melyar::search(r_query);

        QVariantList q_results;
        for (const auto &item : results) {
            QVariantMap track;
            track["id"] = QString::fromStdString(std::string(item.id));
            track["title"] = QString::fromStdString(std::string(item.title));
            track["artist"] = QString::fromStdString(std::string(item.artist));
            track["album"] = QString::fromStdString(std::string(item.album));
            track["duration"] = item.duration_secs;
            track["thumbnail"] = QString::fromStdString(std::string(item.thumbnail_url));
            q_results.append(track);
        }

        QMetaObject::invokeMethod(this, [this, q_results]() {
            handleSearchResults(q_results);
        }, Qt::QueuedConnection);
    });
}

void MelyarBridge::playTrack(const QString &trackId) {
    (void)QtConcurrent::run([trackId]() {
        rust::String r_id(trackId.toStdString());
        melyar::play_track(r_id);
    });
}

void MelyarBridge::togglePlay() {
    melyar::toggle_play();
}

void MelyarBridge::seek(double seconds) {
    melyar::seek(seconds);
}

void MelyarBridge::setVolume(double volume) {
    if (m_volume != volume) {
        m_volume = volume;
        melyar::set_volume(volume);
        emit volumeChanged();
    }
}

void MelyarBridge::nextTrack() {
    melyar::next_track();
}

void MelyarBridge::previousTrack() {
    melyar::previous_track();
}

void MelyarBridge::toggleFavorite(const QString &trackId, const QString &title,
                                 const QString &artist, const QString &album,
                                 double duration, const QString &thumb) {
    bool fav = melyar::toggle_favorite(
        rust::String(trackId.toStdString()),
        rust::String(title.toStdString()),
        rust::String(artist.toStdString()),
        rust::String(album.toStdString()),
        duration,
        rust::String(thumb.toStdString())
    );
    Q_UNUSED(fav);
    loadFavorites();
}

bool MelyarBridge::isFavorite(const QString &trackId) {
    return melyar::is_favorite(rust::String(trackId.toStdString()));
}

QVariantMap MelyarBridge::getTrackMetadata(const QString &trackId) {
    // The core resolves the metadata, which may involve a blocking yt-dlp
    // call, so it runs on a worker thread and the result comes back through
    // the track metadata changed signal.
    auto trackIdCopy = trackId;
    (void)QtConcurrent::run([this, trackIdCopy]() {
        auto meta = melyar::get_track_metadata(rust::String(trackIdCopy.toStdString()));

        QVariantMap map;
        map["id"] = QString::fromStdString(std::string(meta.id));
        map["title"] = QString::fromStdString(std::string(meta.title));
        map["artist"] = QString::fromStdString(std::string(meta.artist));
        map["album"] = QString::fromStdString(std::string(meta.album));
        map["duration"] = meta.duration_secs;
        map["thumbnail"] = QString::fromStdString(std::string(meta.thumbnail_url));

        QMetaObject::invokeMethod(this, [this, map]() {
            m_pendingTrackMeta = map;
            emit trackMetadataChanged();
        }, Qt::QueuedConnection);
    });

    return QVariantMap();
}

void MelyarBridge::loadFavorites() {
    (void)QtConcurrent::run([this]() {
        auto favs = melyar::get_favorites();
        QVariantList q_favs;
        for (const auto &item : favs) {
            QVariantMap track;
            track["id"] = QString::fromStdString(std::string(item.id));
            track["title"] = QString::fromStdString(std::string(item.title));
            track["artist"] = QString::fromStdString(std::string(item.artist));
            track["album"] = QString::fromStdString(std::string(item.album));
            track["duration"] = item.duration_secs;
            track["thumbnail"] = QString::fromStdString(std::string(item.thumbnail_url));
            q_favs.append(track);
        }

        QMetaObject::invokeMethod(this, [this, q_favs]() {
            m_favorites = q_favs;
            emit favoritesChanged();
        }, Qt::QueuedConnection);
    });
}

void MelyarBridge::loadHistory() {
    (void)QtConcurrent::run([this]() {
        auto hist = melyar::get_history();
        QVariantList q_hist;
        for (const auto &item : hist) {
            QVariantMap track;
            track["id"] = QString::fromStdString(std::string(item.id));
            track["title"] = QString::fromStdString(std::string(item.title));
            track["artist"] = QString::fromStdString(std::string(item.artist));
            track["album"] = QString::fromStdString(std::string(item.album));
            track["duration"] = item.duration_secs;
            track["thumbnail"] = QString::fromStdString(std::string(item.thumbnail_url));
            q_hist.append(track);
        }

        QMetaObject::invokeMethod(this, [this, q_hist]() {
            m_history = q_hist;
            emit historyChanged();
        }, Qt::QueuedConnection);
    });
}

void MelyarBridge::loadLibrary() {
    (void)QtConcurrent::run([this]() {
        // Load library albums
        auto albums = melyar::get_library_albums();
        QVariantList q_albums;
        for (const auto &item : albums) {
            QVariantMap album;
            album["id"] = QString::fromStdString(std::string(item.id));
            album["title"] = QString::fromStdString(std::string(item.title));
            album["artist"] = QString::fromStdString(std::string(item.artist));
            album["artwork"] = QString::fromStdString(std::string(item.thumbnail_url));
            album["year"] = QString::fromStdString(std::string(item.year));
            album["trackCount"] = item.track_count;
            album["duration"] = item.duration_secs;
            q_albums.append(album);
        }

        // Load library artists
        auto artists = melyar::get_library_artists();
        QVariantList q_artists;
        for (const auto &item : artists) {
            QVariantMap artist;
            artist["id"] = QString::fromStdString(std::string(item.id));
            artist["name"] = QString::fromStdString(std::string(item.name));
            artist["artwork"] = QString::fromStdString(std::string(item.thumbnail_url));
            artist["followerCount"] = static_cast<qlonglong>(item.follower_count);
            q_artists.append(artist);
        }

        // Load library playlists
        auto playlists = melyar::get_library_playlists();
        QVariantList q_playlists;
        for (const auto &item : playlists) {
            QVariantMap playlist;
            playlist["id"] = QString::fromStdString(std::string(item.id));
            playlist["name"] = QString::fromStdString(std::string(item.name));
            playlist["artwork"] = QString::fromStdString(std::string(item.thumbnail_url));
            playlist["trackCount"] = item.track_count;
            playlist["isOwner"] = item.is_owner;
            q_playlists.append(playlist);
        }

        QMetaObject::invokeMethod(this, [this, q_albums, q_artists, q_playlists]() {
            m_libraryAlbums = q_albums;
            m_libraryArtists = q_artists;
            m_libraryPlaylists = q_playlists;
            emit libraryAlbumsChanged();
            emit libraryArtistsChanged();
            emit libraryPlaylistsChanged();
        }, Qt::QueuedConnection);
    });
}

void MelyarBridge::fetchLyrics(const QString &trackId) {
    // Async: cache first, then live lrclib fetch (uses current track title/artist)
    const QString id = trackId;
    const QString title = m_currentTitle;
    const QString artist = m_currentArtist;
    const double dur = m_currentDuration;
    (void)QtConcurrent::run([this, id, title, artist, dur]() {
        rust::String lrc = melyar::fetch_lyrics_live(
            rust::String(id.toStdString()),
            rust::String(title.toStdString()),
            rust::String(artist.toStdString()),
            dur);
        QString q = QString::fromStdString(std::string(lrc));
        QMetaObject::invokeMethod(this, [this, id, q]() {
            if (m_currentTrackId == id) {
                m_currentLyrics = q;
                emit lyricsChanged();
            }
        }, Qt::QueuedConnection);
    });
}

// New playback control functions

QVariantList MelyarBridge::queueFromCore() const {
    QVariantList q_list;
    try {
        for (const auto &item : melyar::get_queue()) {
            QVariantMap track;
            track["id"] = QString::fromStdString(std::string(item.id));
            track["title"] = QString::fromStdString(std::string(item.title));
            track["artist"] = QString::fromStdString(std::string(item.artist));
            track["album"] = QString::fromStdString(std::string(item.album));
            track["duration"] = item.duration_secs;
            track["thumbnail"] = QString::fromStdString(std::string(item.thumbnail_url));
            q_list.append(track);
        }
    } catch (const std::exception &e) {
        std::cerr << "queue read failed: " << e.what() << std::endl;
    }
    return q_list;
}

void MelyarBridge::replaceQueue(const QVariantList &tracks, int startIndex) {
    rust::Vec<melyar::CxxTrackMetadata> vec;
    for (const auto &item : tracks) {
        const QVariantMap t = item.toMap();
        melyar::CxxTrackMetadata meta;
        meta.id = t["id"].toString().toStdString();
        meta.title = t["title"].toString().toStdString();
        meta.artist = t["artist"].toString().toStdString();
        meta.album = t["album"].toString().toStdString();
        meta.duration_secs = t["duration"].toDouble();
        meta.thumbnail_url = t["thumbnail"].toString().toStdString();
        meta.lyrics = "";
        vec.push_back(std::move(meta));
    }
    try {
        melyar::replace_queue(vec, startIndex);
        // Refresh the exposed queue property so the UI queue panel stays in sync
        m_queue = queueFromCore();
        emit queueChanged();
    } catch (const std::exception &e) {
        std::cerr << "replaceQueue failed: " << e.what() << std::endl;
    }
}

void MelyarBridge::toggleShuffle() {
    m_shuffleMode = !m_shuffleMode;
    melyar::set_shuffle(m_shuffleMode);
    emit shuffleModeChanged();
}

void MelyarBridge::cycleRepeat() {
    m_repeatMode = (m_repeatMode + 1) % 3; // 0 -> 1 -> 2 -> 0
    melyar::set_repeat_mode(m_repeatMode);
    emit repeatModeChanged();
}

void MelyarBridge::setShuffleMode(bool enabled) {
    if (m_shuffleMode == enabled) return;
    m_shuffleMode = enabled;
    melyar::set_shuffle(m_shuffleMode);
    emit shuffleModeChanged();
}

void MelyarBridge::setRepeatMode(int mode) {
    const int clamped = qBound(0, mode, 2);
    if (m_repeatMode == clamped) return;
    m_repeatMode = clamped;
    melyar::set_repeat_mode(m_repeatMode);
    emit repeatModeChanged();
}

void MelyarBridge::addToQueue(const QString &trackId) {
    (void)QtConcurrent::run([this, trackId]() {
        rust::String r_id(trackId.toStdString());
        auto track = melyar::get_track_metadata(r_id);
        if (track.id.empty()) return;
        // Write to the core first, then read the list back. Appending only to
        // this mirror drew the track and never played it: the end of the song,
        // the media keys and MPRIS all read the core's list, which knew
        // nothing about it.
        melyar::append_to_queue(track);
        const QVariantList updated = queueFromCore();
        QMetaObject::invokeMethod(this, [this, updated]() {
            m_queue = updated;
            emit queueChanged();
        }, Qt::QueuedConnection);
    });
}

void MelyarBridge::playNextInQueue(const QString &trackId) {
    (void)QtConcurrent::run([this, trackId]() {
        rust::String r_id(trackId.toStdString());
        auto track = melyar::get_track_metadata(r_id);
        if (track.id.empty()) return;
        melyar::insert_next_in_queue(track);
        const QVariantList updated = queueFromCore();
        QMetaObject::invokeMethod(this, [this, updated]() {
            m_queue = updated;
            emit queueChanged();
        }, Qt::QueuedConnection);
    });
}

void MelyarBridge::removeFromQueue(int index) {
    if (!melyar::remove_from_queue(index)) return;
    m_queue = queueFromCore();
    emit queueChanged();
}

void MelyarBridge::playFromQueue(int index) {
    if (index < 0 || index >= m_queue.length()) return;
    auto track = m_queue[index].toMap();
    const QString trackId = track["id"].toString();

    // Tell the core where playback resumed. Its cursor is what the end of the
    // track advances from, and it used to be left wherever it last happened to
    // be, so picking row five carried on from somewhere else entirely.
    // The queue also keeps its order now instead of jumping the clicked row to
    // the front behind the user's back.
    melyar::set_current_queue_index(index);
    playTrack(trackId);
}

void MelyarBridge::clearQueue() {
    melyar::clear_queue();
    m_queue = queueFromCore();
    emit queueChanged();
}

void MelyarBridge::setSleepTimer(int seconds) {
    if (seconds > 0) {
        m_sleepTimerActive = true;
        int mins = seconds / 60;
        if (mins > 0) {
            m_sleepTimerText = QString::number(mins) + " min";
        } else {
            m_sleepTimerText = QString::number(seconds) + " sec";
        }
    } else if (seconds == -1) {
        m_sleepTimerActive = true;
        m_sleepTimerText = "End of track";
    } else {
        m_sleepTimerActive = false;
        m_sleepTimerText = "";
    }
    emit sleepTimerChanged();

    if (seconds > 0) {
        // Restart the one timer rather than arm another. A second call used to
        // leave both deadlines live, and neither could ever be cancelled.
        m_sleepTimer->start(seconds * 1000);
    } else {
        m_sleepTimer->stop();
    }
}

void MelyarBridge::cancelSleepTimer() {
    // Stop the deadline, not only the label. Cancelling cleared the display
    // while the original timer still fired and paused playback on its own.
    m_sleepTimer->stop();
    m_sleepTimerActive = false;
    m_sleepTimerText.clear();
    emit sleepTimerChanged();
}

void MelyarBridge::clearSearchHistory() {
    m_searchHistory.clear();
    melyar::clear_search_history();
    emit searchHistoryChanged();
}

void MelyarBridge::removeFromSearchHistory(const QString &query) {
    m_searchHistory.removeAll(query);
    melyar::remove_from_search_history(rust::String(query.toStdString()));
    emit searchHistoryChanged();
}

QString MelyarBridge::loadSettings() const {
    return QString::fromStdString(std::string(melyar::get_settings_json()));
}

bool MelyarBridge::saveSetting(const QString &key, const QString &value) {
    return melyar::set_setting(rust::String(key.toStdString()), rust::String(value.toStdString()));
}

bool MelyarBridge::removeSetting(const QString &key) {
    return melyar::delete_setting(rust::String(key.toStdString()));
}

QString MelyarBridge::listSources() const {
    return QString::fromStdString(std::string(melyar::list_sources_json()));
}

bool MelyarBridge::setSourceEnabled(const QString &sourceId, bool enabled) {
    return melyar::set_source_enabled(rust::String(sourceId.toStdString()), enabled);
}

void MelyarBridge::openUri(const QString &uri) {
    if (uri.trimmed().isEmpty()) return;
    m_pendingUri = uri;
    emit uriChanged();
}

void MelyarBridge::requestFilePick() {
    m_pickedFile.clear();
    emit filePickRequested();
}

void MelyarBridge::setPickedFile(const QString &fileUrl) {
    // The QML dialog hands over a url, the core wants a path.
    QString path = fileUrl;
    if (path.startsWith("file://")) {
        path = QUrl(path).toLocalFile();
    }
    if (path.isEmpty()) {
        // Cancelled. The answer has to come back even when there is nothing to
        // report: the web side is waiting on this signal, and a promise left
        // waiting keeps its handler connected, so the next dialog would wake the
        // stale handler as well and import the file twice.
        emit filePicked(QString());
        return;
    }
    m_pickedFile = path;
    emit filePicked(path);
}

void MelyarBridge::importLibrary(const QString &path) {
    if (path.trimmed().isEmpty()) return;
    m_importProgress.clear();
    emit importProgressChanged();

    melyar::import_library(rust::String(path.toStdString()));
}

void MelyarBridge::cancelImport() {
    melyar::cancel_import();
}

void MelyarBridge::setImportProgress(const QVariantMap &progress) {
    m_importProgress = progress;
    emit importProgressChanged();
}

void MelyarBridge::quit() {
    QCoreApplication::quit();
}

void MelyarBridge::setEqualizerBands(const QVariantList &bands) {
    rust::Vec<double> gains;
    gains.reserve(static_cast<std::size_t>(bands.size()));
    for (const QVariant &band : bands) {
        gains.push_back(band.toDouble());
    }
    melyar::set_equalizer_bands(std::move(gains));
}

void MelyarBridge::handleProgressUpdate(double current, double total) {
    m_currentPosition = current;
    if (total > 0.0) {
        m_currentDuration = total;
    }
    emit progressUpdated(current, total);
}

void MelyarBridge::handleTrackChanged(const QString &id, const QString &title,
                                     const QString &artist, const QString &album,
                                     double duration, const QString &thumb) {
    m_currentTrackId = id;
    m_currentTitle = title;
    m_currentArtist = artist;
    m_currentAlbum = album;
    m_currentDuration = duration;
    m_currentThumbnail = thumb;
    m_currentLyrics = "";
    emit currentTrackChanged();
    loadHistory();
}

void MelyarBridge::handleStateChanged(bool isPlaying) {
    if (m_isPlaying != isPlaying) {
        m_isPlaying = isPlaying;
        emit playbackStateChanged(isPlaying);
    }
}

void MelyarBridge::handleLyricsLoaded(const QString &trackId, const QString &lrc) {
    if (m_currentTrackId == trackId) {
        m_currentLyrics = lrc;
        emit lyricsChanged();
    }
}

void MelyarBridge::handleSearchResults(const QVariantList &results) {
    m_searchResults = results;
    emit searchResultsChanged();
}

// ----------------------------------------------------------------------------
// CXX Callbacks from Rust
// ----------------------------------------------------------------------------

namespace melyar {

void on_progress_update(double current, double total) {
    auto *b = MelyarBridge::instance();
    if (b) {
        QMetaObject::invokeMethod(b, [b, current, total]() {
            b->handleProgressUpdate(current, total);
        }, Qt::QueuedConnection);
    }
}

void on_track_changed(CxxTrackMetadata track) {
    auto *b = MelyarBridge::instance();
    if (b) {
        QString id = QString::fromStdString(std::string(track.id));
        QString title = QString::fromStdString(std::string(track.title));
        QString artist = QString::fromStdString(std::string(track.artist));
        QString album = QString::fromStdString(std::string(track.album));
        double dur = track.duration_secs;
        QString thumb = QString::fromStdString(std::string(track.thumbnail_url));

        QMetaObject::invokeMethod(b, [b, id, title, artist, album, dur, thumb]() {
            b->handleTrackChanged(id, title, artist, album, dur, thumb);
        }, Qt::QueuedConnection);
    }
}

void on_state_changed(bool is_playing) {
    auto *b = MelyarBridge::instance();
    if (b) {
        QMetaObject::invokeMethod(b, [b, is_playing]() {
            b->handleStateChanged(is_playing);
        }, Qt::QueuedConnection);
    }
}

void on_lyrics_loaded(rust::String track_id, rust::String lyrics_lrc) {
    auto *b = MelyarBridge::instance();
    if (b) {
        QString q_id = QString::fromStdString(std::string(track_id));
        QString q_lrc = QString::fromStdString(std::string(lyrics_lrc));

        QMetaObject::invokeMethod(b, [b, q_id, q_lrc]() {
            b->handleLyricsLoaded(q_id, q_lrc);
        }, Qt::QueuedConnection);
    }
}

void on_search_results(rust::Vec<CxxTrackMetadata> results) {    auto *b = MelyarBridge::instance();
    if (b) {
        QVariantList q_list;
        for (const auto &item : results) {
            QVariantMap track;
            track["id"] = QString::fromStdString(std::string(item.id));
            track["title"] = QString::fromStdString(std::string(item.title));
            track["artist"] = QString::fromStdString(std::string(item.artist));
            track["album"] = QString::fromStdString(std::string(item.album));
            track["duration"] = item.duration_secs;
            track["thumbnail"] = QString::fromStdString(std::string(item.thumbnail_url));
            q_list.append(track);
        }

        QMetaObject::invokeMethod(b, [b, q_list]() {
            b->handleSearchResults(q_list);
        }, Qt::QueuedConnection);
    }
}

void on_import_progress(int32_t current, int32_t total, int32_t matched, int32_t skipped) {
    auto *b = MelyarBridge::instance();
    if (!b) {
        return;
    }

    QVariantMap progress;
    progress["current"] = current;
    progress["total"] = total;
    progress["matched"] = matched;
    progress["skipped"] = skipped;
    progress["done"] = total > 0 && current >= total;

    QMetaObject::invokeMethod(b, [b, progress]() {
        b->setImportProgress(progress);
    }, Qt::QueuedConnection);
}

} // namespace melyar
