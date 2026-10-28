#pragma once

#include <QObject>
#include <QString>
#include <QTimer>
#include <QVariantList>
#include <QVariantMap>
#include <cstdint>
#include <memory>
#include <vector>

#include "rust/cxx.h"

namespace melyar {
    struct CxxTrackMetadata;

    // Declarations for Rust -> C++ CXX FFI callbacks
    void on_progress_update(double current, double total);
    void on_track_changed(CxxTrackMetadata track);
    void on_state_changed(bool is_playing);
    void on_lyrics_loaded(rust::String track_id, rust::String lyrics_lrc);
    void on_search_results(rust::Vec<CxxTrackMetadata> results);
    void on_import_progress(int32_t current, int32_t total, int32_t matched, int32_t skipped);
}

class MelyarBridge : public QObject {
    Q_OBJECT

    Q_PROPERTY(QString currentTrackId READ currentTrackId NOTIFY currentTrackChanged)
    Q_PROPERTY(QString currentTitle READ currentTitle NOTIFY currentTrackChanged)
    Q_PROPERTY(QString currentArtist READ currentArtist NOTIFY currentTrackChanged)
    Q_PROPERTY(QString currentAlbum READ currentAlbum NOTIFY currentTrackChanged)
    Q_PROPERTY(QString currentThumbnail READ currentThumbnail NOTIFY currentTrackChanged)
    Q_PROPERTY(double currentDuration READ currentDuration NOTIFY currentTrackChanged)
    Q_PROPERTY(double currentPosition READ currentPosition NOTIFY progressUpdated)
    Q_PROPERTY(bool isPlaying READ isPlaying NOTIFY playbackStateChanged)
    Q_PROPERTY(double volume READ volume WRITE setVolume NOTIFY volumeChanged)
    Q_PROPERTY(QString currentLyrics READ currentLyrics NOTIFY lyricsChanged)
    Q_PROPERTY(QString pendingUri READ pendingUri NOTIFY uriChanged)
    Q_PROPERTY(QVariantMap pendingTrackMeta READ pendingTrackMeta NOTIFY trackMetadataChanged)
    Q_PROPERTY(QVariantMap importProgress READ importProgress NOTIFY importProgressChanged)
    Q_PROPERTY(QString pickedFile READ pickedFile NOTIFY filePicked)
    Q_PROPERTY(QVariantList searchResults READ searchResults NOTIFY searchResultsChanged)
    Q_PROPERTY(QVariantList favorites READ favorites NOTIFY favoritesChanged)
    Q_PROPERTY(QVariantList history READ history NOTIFY historyChanged)
    Q_PROPERTY(QVariantList queue READ queue NOTIFY queueChanged)
    Q_PROPERTY(int queueLength READ queueLength NOTIFY queueChanged)
    Q_PROPERTY(bool shuffleMode READ shuffleMode NOTIFY shuffleModeChanged)
    Q_PROPERTY(int repeatMode READ repeatMode NOTIFY repeatModeChanged)
    Q_PROPERTY(bool sleepTimerActive READ sleepTimerActive NOTIFY sleepTimerChanged)
    Q_PROPERTY(QString sleepTimerText READ sleepTimerText NOTIFY sleepTimerChanged)
    Q_PROPERTY(QVariantList libraryAlbums READ libraryAlbums NOTIFY libraryAlbumsChanged)
    Q_PROPERTY(QVariantList libraryArtists READ libraryArtists NOTIFY libraryArtistsChanged)
    Q_PROPERTY(QVariantList libraryPlaylists READ libraryPlaylists NOTIFY libraryPlaylistsChanged)
    Q_PROPERTY(QVariantList searchHistory READ searchHistory NOTIFY searchHistoryChanged)

public:
    static MelyarBridge* instance();
    explicit MelyarBridge(QObject *parent = nullptr);
    ~MelyarBridge() override;

    QString currentTrackId() const { return m_currentTrackId; }
    QString currentTitle() const { return m_currentTitle; }
    QString currentArtist() const { return m_currentArtist; }
    QString currentAlbum() const { return m_currentAlbum; }
    QString currentThumbnail() const { return m_currentThumbnail; }
    double currentDuration() const { return m_currentDuration; }
    double currentPosition() const { return m_currentPosition; }
    bool isPlaying() const { return m_isPlaying; }
    double volume() const { return m_volume; }
    QString currentLyrics() const { return m_currentLyrics; }
    QString pendingUri() const { return m_pendingUri; }
    QVariantMap pendingTrackMeta() const { return m_pendingTrackMeta; }
    QVariantMap importProgress() const { return m_importProgress; }
    QString pickedFile() const { return m_pickedFile; }
    QVariantList searchResults() const { return m_searchResults; }
    QVariantList favorites() const { return m_favorites; }
    QVariantList history() const { return m_history; }

    QVariantList queue() const { return m_queue; }
    int queueLength() const { return m_queue.length(); }
    bool shuffleMode() const { return m_shuffleMode; }
    int repeatMode() const { return m_repeatMode; }
    bool sleepTimerActive() const { return m_sleepTimerActive; }
    QString sleepTimerText() const { return m_sleepTimerText; }
    QVariantList libraryAlbums() const { return m_libraryAlbums; }
    QVariantList libraryArtists() const { return m_libraryArtists; }
    QVariantList libraryPlaylists() const { return m_libraryPlaylists; }
    QVariantList searchHistory() const { return m_searchHistory; }

    // Invokable from QML
    Q_INVOKABLE void search(const QString &query);
    Q_INVOKABLE void playTrack(const QString &trackId);
    Q_INVOKABLE void togglePlay();
    Q_INVOKABLE void seek(double seconds);
    Q_INVOKABLE void setVolume(double volume);
    Q_INVOKABLE void nextTrack();
    Q_INVOKABLE void previousTrack();
    Q_INVOKABLE void toggleFavorite(const QString &trackId, const QString &title,
                                   const QString &artist, const QString &album,
                                   double duration, const QString &thumb);
    Q_INVOKABLE bool isFavorite(const QString &trackId);
    Q_INVOKABLE void loadFavorites();
    Q_INVOKABLE void loadHistory();
    Q_INVOKABLE void fetchLyrics(const QString &trackId);
    Q_INVOKABLE void toggleShuffle();
    Q_INVOKABLE void cycleRepeat();
    Q_INVOKABLE void setShuffleMode(bool enabled);
    Q_INVOKABLE void setRepeatMode(int mode);
    Q_INVOKABLE void replaceQueue(const QVariantList &tracks, int startIndex);
    Q_INVOKABLE void addToQueue(const QString &trackId);
    /** Insert right after the track that is playing. addToQueue appends, which
     * is a different promise from the one "Play next" makes. */
    Q_INVOKABLE void playNextInQueue(const QString &trackId);
    Q_INVOKABLE void removeFromQueue(int index);
    Q_INVOKABLE void playFromQueue(int index);
    Q_INVOKABLE void clearQueue();
    Q_INVOKABLE void setSleepTimer(int seconds);
    Q_INVOKABLE void cancelSleepTimer();
    Q_INVOKABLE void loadLibrary();
    Q_INVOKABLE void clearSearchHistory();
    Q_INVOKABLE void removeFromSearchHistory(const QString &query);
    Q_INVOKABLE QString loadSettings() const;
    Q_INVOKABLE bool saveSetting(const QString &key, const QString &value);
    Q_INVOKABLE bool removeSetting(const QString &key);
    /** The music sources the core knows about, as a JSON array. Each entry
     *  carries its id, name, whether it is on, and what it is made of. */
    Q_INVOKABLE QString listSources() const;
    /** Switch one source on or off. Returns false for an unknown id, or for
     *  an attempt to switch YouTube off — it is the default and the anchor
     *  every bare track id resolves against. */
    Q_INVOKABLE bool setSourceEnabled(const QString &sourceId, bool enabled);
    Q_INVOKABLE void openUri(const QString &uri);
    Q_INVOKABLE QVariantMap getTrackMetadata(const QString &trackId);
    Q_INVOKABLE void requestFilePick();
    Q_INVOKABLE void setPickedFile(const QString &fileUrl);
    Q_INVOKABLE void importLibrary(const QString &path);
    Q_INVOKABLE void cancelImport();
    Q_INVOKABLE void setEqualizerBands(const QVariantList &bands);
    Q_INVOKABLE void quit();

signals:
    void currentTrackChanged();
    void playbackStateChanged(bool isPlaying);
    void progressUpdated(double current, double total);
    void lyricsChanged();
    void searchResultsChanged();
    void favoritesChanged();
    void historyChanged();
    void volumeChanged();
    void queueChanged();
    void shuffleModeChanged();
    void repeatModeChanged();
    void sleepTimerChanged();
    void libraryAlbumsChanged();
    void libraryArtistsChanged();
    void libraryPlaylistsChanged();
    void searchHistoryChanged();
    void uriChanged();
    void trackMetadataChanged();
    void importProgressChanged();
    void filePickRequested();
    void filePicked(const QString &path);

public slots:
    void handleProgressUpdate(double current, double total);
    void handleTrackChanged(const QString &id, const QString &title,
                            const QString &artist, const QString &album,
                            double duration, const QString &thumb);
    void handleStateChanged(bool isPlaying);
    void handleLyricsLoaded(const QString &trackId, const QString &lrc);
    void handleSearchResults(const QVariantList &results);
    void setImportProgress(const QVariantMap &progress);

private:
    static MelyarBridge* s_instance;

    QString m_currentTrackId;
    QString m_currentTitle{"No Track Playing"};
    QString m_currentArtist{"Select a track to start"};
    QString m_currentAlbum{""};
    QString m_currentThumbnail{""};
    double m_currentDuration{0.0};
    double m_currentPosition{0.0};
    bool m_isPlaying{false};
    double m_volume{100.0};
    QString m_currentLyrics{""};
    QString m_pendingUri{""};
    QVariantMap m_pendingTrackMeta;
    QVariantMap m_importProgress;
    QString m_pickedFile{""};
    QVariantList m_searchResults;
    QVariantList m_favorites;
    QVariantList m_history;
    QVariantList m_queue;
    bool m_shuffleMode{false};
    int m_repeatMode{0}; // 0=off, 1=all, 2=one
    bool m_sleepTimerActive{false};
    QString m_sleepTimerText{""};
    /** One timer the bridge owns. An armed-and-forgotten singleShot cannot be
     * cancelled, and re-arming left two of them racing to pause playback. */
    QTimer *m_sleepTimer{nullptr};
    QVariantList m_libraryAlbums;
    QVariantList m_libraryArtists;
    QVariantList m_libraryPlaylists;
    QVariantList m_searchHistory;

    /** Read the queue back from the core. The core is the single list that
     * drives auto-advance, the media keys and MPRIS, so every mutation is
     * written there first and this mirror is only ever rebuilt from it. */
    QVariantList queueFromCore() const;
};
