import { usePlayer } from "@/store";
import { register } from "@/lib/registry";
import { useSettings } from "@/lib/settings";
import { usePlugins } from "@/lib/plugins";
import { bindImportProgress } from "@/lib/import";
import type { RepeatMode, TrackMeta } from "@/lib/types";

let bridgeInstance: any = null;

function clamp01(v: number) {
  return Math.max(0, Math.min(1, Number.isFinite(v) ? v : 0));
}

function toTracks(raw: unknown): TrackMeta[] {
  if (!Array.isArray(raw)) return [];
  const out: TrackMeta[] = [];
  for (const item of raw) {
    if (!item || typeof item.id !== "string" || !item.id) continue;
    out.push(
      register({
        id: item.id,
        title: typeof item.title === "string" ? item.title : "",
        artist: typeof item.artist === "string" ? item.artist : "",
        album: typeof item.album === "string" ? item.album : "",
        duration: typeof item.duration === "number" ? item.duration : 0,
        thumbnail: typeof item.thumbnail === "string" ? item.thumbnail : "",
      }),
    );
  }
  return out;
}

export function getBridge() {
  return bridgeInstance;
}

export function initBridge() {
  if (typeof window === "undefined") return;
  const start = () => {
    const transport = (window.qt as any)?.webChannelTransport;
    if (!transport || typeof window.QWebChannel !== "function") {
      console.error("[Melyar] webChannelTransport or QWebChannel missing (browser mode?)");
      usePlayer.getState().patch({ bridgeError: "native" });
      return;
    }
    new window.QWebChannel(transport, (channel: any) => {
      const b = channel?.objects?.bridge;
      if (!b) {
        console.error("[Melyar] channel.objects.bridge not found");
        usePlayer.getState().patch({ bridgeError: "bridge" });
        return;
      }
      bridgeInstance = b;
      window.bridge = b;
      console.log("[Melyar] Native bridge connected");

      const s = usePlayer.getState();
      s.patch({
        volume: clamp01((b.volume ?? 80) / 100),
        shuffle: Boolean(b.shuffleMode),
        repeat: (Math.max(0, Math.min(2, Number(b.repeatMode) || 0)) as RepeatMode),
        booted: true,
      });

      // ---- signals: connected exactly once; handlers never re-connect ----
      b.progressUpdated.connect((current: number, total: number) => {
        const st = usePlayer.getState();
        // Ignore position updates from a different track (race between
        // playTrack setting current and currentTrackChanged firing).
        if (st.current && st.current.id !== b.currentTrackId) return;
        usePlayer.setState({
          position: Number(current) || 0,
          duration: Number(total) > 0 ? Number(total) : st.duration,
        });
        if (st.lyricsOpen) void 0; // lyric index derives from position reactively
      });

      b.playbackStateChanged.connect((playing: boolean) => {
        usePlayer.setState({ playing: Boolean(playing) });
      });

      b.currentTrackChanged.connect(() => {
        console.log('[Melyar] currentTrackChanged fired, id=', b.currentTrackId);
        const id: string = b.currentTrackId || "";
        if (!id) return;
        const meta = register({
          id,
          title: b.currentTitle || "",
          artist: b.currentArtist || "",
          album: b.currentAlbum || "",
          duration: Number(b.currentDuration) || 0,
          thumbnail: b.currentThumbnail || "",
        });
        // New request id guards against stale polls overwriting a newer track's lyrics.
        const requestId = Date.now();
        usePlayer.setState({
          current: meta,
          playing: Boolean(b.isPlaying),
          duration: Number(b.currentDuration) || 0,
          position: 0,
          lyricsRaw: null,
          lyricsRequestId: requestId,
          lyricsAutoFollow: true, // fresh track => re-center on the playing line
        });
        if (typeof b.fetchLyrics === "function") {
          b.fetchLyrics(id);
          let polls = 0;
          const poll = () => {
            // Ignore this poll if a newer track has already requested lyrics.
            if (usePlayer.getState().lyricsRequestId !== requestId) return;
            const lrc = b.currentLyrics;
            if (lrc && lrc.trim()) {
              usePlayer.getState().patch({ lyricsRaw: lrc });
            } else if (polls < 30) {
              polls++;
              setTimeout(poll, 500);
            }
          };
          setTimeout(poll, 500);
        } else {
          console.log('[Melyar] fetchLyrics NOT a function');
        }
      });

      b.searchResultsChanged.connect(() => {
        usePlayer.setState({ results: toTracks(b.searchResults), searching: false });
      });

      b.favoritesChanged.connect(() => {
        usePlayer.setState({ liked: toTracks(b.favorites) });
      });

      b.historyChanged.connect(() => {
        usePlayer.setState({ history: toTracks(b.history) });
      });

      b.queueChanged.connect(() => {
        usePlayer.setState({ queue: toTracks(b.queue) });
      });

      b.volumeChanged.connect(() => {
        usePlayer.setState({ volume: clamp01((b.volume ?? 80) / 100) });
      });

      b.lyricsChanged.connect(() => {
        // No id check here on purpose. The bridge only ever assigns lyrics whose
        // track matches the current one (MelyarBridge::handleLyricsLoaded) and
        // clears them on every track change, so the signal carries the right
        // lyrics by construction. The guard this used to have compared the
        // stored request id against Date.now() at delivery time - two different
        // milliseconds, always - so it rejected every signal and lyrics only
        // arrived through the 500 ms poll, half a second late.
        usePlayer.setState({ lyricsRaw: (b.currentLyrics || "") || null });
      });

      b.shuffleModeChanged.connect(() => {
        usePlayer.setState({ shuffle: Boolean(b.shuffleMode) });
      });

      b.repeatModeChanged.connect(() => {
        usePlayer.setState({ repeat: Math.max(0, Math.min(2, Number(b.repeatMode) || 0)) as RepeatMode });
      });

      // Deep links: the desktop entry declares the melyar:// handler, so the
      // application has to act on them.
      b.uriChanged.connect(() => {
        const uri = String(b.pendingUri || "");
        if (!uri) return;
        b.pendingUri = "";
        void handleUri(uri);
      });

      b.trackMetadataChanged.connect(() => {
        const meta = b.pendingTrackMeta;
        if (!meta || !meta.id) return;
        b.pendingTrackMeta = {};
        const st = usePlayer.getState();
        st.playTrack(
          register({
            id: String(meta.id),
            title: String(meta.title || ""),
            artist: String(meta.artist || ""),
            album: String(meta.album || ""),
            duration: Number(meta.duration) || 0,
            thumbnail: String(meta.thumbnail || ""),
          }),
          [],
        );
      });

      // A deep link that arrived before the interface was ready is still
      // waiting on the bridge.
      const pendingUri = String(b.pendingUri || "");
      if (pendingUri) {
        b.pendingUri = "";
        void handleUri(pendingUri);
      }

      // ---- initial data pulls (signals already connected: nothing is lost) ----
      if (typeof b.loadFavorites === "function") b.loadFavorites();
      if (typeof b.loadHistory === "function") b.loadHistory();

      // The library import reports its progress through the bridge.
      bindImportProgress();

      // ---- persisted settings ----
      // The settings live in the core's database, so they survive a restart.
      // Loading is asynchronous, so the playback preferences are restored
      // once it resolves.
      void useSettings
        .getState()
        .load()
        .then(() => {
          const stored = useSettings.getState().settings;
          const b = getBridge();
          if (!b) return;

          if (typeof b.setVolume === "function") b.setVolume(Math.round(stored.volume * 100));
          usePlayer.setState({ volume: clamp01(stored.volume) });

          if (typeof b.setShuffleMode === "function" && stored.shuffle) b.setShuffleMode(true);
          if (typeof b.setRepeatMode === "function" && stored.repeat > 0) {
            b.setRepeatMode(stored.repeat);
          }
        });

      // Restore current track if the core already has one
      if (b.currentTrackId) {
        const meta = register({
          id: b.currentTrackId,
          title: b.currentTitle || "",
          artist: b.currentArtist || "",
          album: b.currentAlbum || "",
          duration: Number(b.currentDuration) || 0,
          thumbnail: b.currentThumbnail || "",
        });
        usePlayer.setState({
          current: meta,
          playing: Boolean(b.isPlaying),
          duration: Number(b.currentDuration) || 0,
        });
        if (typeof b.fetchLyrics === "function") b.fetchLyrics(b.currentTrackId);
      }

      // ---- automation hooks (used by end-to-end tests) ----
      const w = window as any;
      w.__testSearch = (q: string) => {
        usePlayer.getState().setQuery(q);
        runSearch(q);
      };
      w.__testPlay = (i: number) => {
        const st = usePlayer.getState();
        const t = st.results[i];
        if (t) st.playTrack(t, st.results);
      };
      w.__testLike = (i: number) => {
        const st = usePlayer.getState();
        const t = st.results[i] ?? st.current;
        if (t) st.toggleLike(t);
      };
      w.__testToggle = () => usePlayer.getState().toggle();
      w.__testSetSetting = (k: string, v: string) =>
        useSettings.getState().set(k as never, v as never);
      w.__testSettings = () => useSettings.getState().settings;
      w.__setTheme = (theme: string) =>
        useSettings.getState().set("theme", theme as never);

      // Navigation and overlay hooks, so the interface can be driven from
      // outside for the automated checks.
      w.__navigate = (view: string) => {
        const st = usePlayer.getState();
        st.navigate({ name: view } as never);
      };
      w.__openExpandedPlayer = () => usePlayer.getState().setExpandedPlayerOpen(true);
      w.__openLyrics = () => usePlayer.getState().setLyricsOpen(true);
      w.__openQueue = () => usePlayer.getState().setQueueOpen(true);
      w.__openPlugins = () => usePlugins.getState().setOpen(true);
      w.__pluginsState = () => {
        const s = usePlugins.getState();
        return { open: s.open, sources: s.sources, loading: s.loading, pending: s.pending };
      };
      w.__testSetState = (partial: Record<string, unknown>) =>
        usePlayer.setState(partial as never);
      w.__openMenuOnFirstTrack = () => {
        const st = usePlayer.getState();
        const track = st.results[0] ?? st.liked[0] ?? st.history[0];
        if (!track) return false;
        st.openMenu({ track, x: 200, y: 200 });
        return true;
      };

      w.__testState = () => {
        const s = usePlayer.getState();
        return {
          view: s.view,
          results: s.results,
          liked: s.liked,
          queue: s.queue,
          playing: s.playing,
          position: s.position,
          duration: s.duration,
          current: s.current,
          lyricsRaw: s.lyricsRaw,
          menuOpen: s.menu !== null,
          queueOpen: s.queueOpen,
          lyricsOpen: s.lyricsOpen,
          expandedPlayerOpen: s.expandedPlayerOpen,
          bridgeError: s.bridgeError,
        };
      };
    });
  };

  if (document.readyState === "loading") {
    window.addEventListener("DOMContentLoaded", start, { once: true });
  } else {
    start();
  }
}

export function runSearch(query: string) {
  const b = bridgeInstance;
  if (!b) return;
  usePlayer.setState({ searching: true });
  b.search(query);
}

/**
 * Act on a `melyar://` link.
 *
 *   melyar://settings          opens the settings panel
 *   melyar://track/<id>        plays a track by its YouTube video id
 *   melyar://search/<query>    runs a search
 */
export async function handleUri(uri: string) {
  const raw = uri.replace(/^melyar:\/\//i, "");
  const [head, ...rest] = raw.split("/");
  const tail = decodeURIComponent(rest.join("/"));
  const st = usePlayer.getState();

  switch (head.toLowerCase()) {
    case "settings":
      useSettings.getState().setOpen(true);
      return;

    case "search":
      if (!tail) return;
      st.setQuery(tail);
      st.navigate({ name: "search" });
      runSearch(tail);
      return;

    case "track": {
      if (!tail) return;
      const b = bridgeInstance;
      if (!b || typeof b.getTrackMetadata !== "function") return;
      // The metadata is resolved asynchronously by the core; the result
      // arrives through the trackMetadataChanged signal.
      b.getTrackMetadata(tail);
      return;
    }

    default:
      console.warn("[Melyar] unknown uri:", uri);
  }
}
