import { usePlaybackStore } from "./playbackStore";
import { useUIStore } from "./uiStore";
import { useLibraryStore } from "./libraryStore";
import type { TrackMeta, View } from "@/lib/types";
import { t } from "@/lib/i18n";

function bridge(): any | null {
  return (typeof window !== "undefined" && window.bridge) || null;
}

type Menu = { track: TrackMeta; x: number; y: number } | null;

// This is a compatibility hook that combines all three granular stores
// Components should migrate to using usePlaybackStore, useUIStore, useLibraryStore directly
// for better performance (fewer re-renders).
export function usePlayer() {
  const playback = usePlaybackStore();
  const ui = useUIStore();
  const library = useLibraryStore();

  // Combined actions that span multiple stores
  const playTrack = (track: TrackMeta, context?: TrackMeta[]) => {
    playback.playTrack(track, context);
    ui.closeMenu();
  };

  const next = () => bridge()?.nextTrack();
  const prev = () => bridge()?.previousTrack();

  // The bridge call, the snapshot and the Undo toast all live in the
  // library store, so this only forwards. Calling the bridge here as well
  // would clear the queue twice.
  const clearQueue = () => library.clearQueue();

  return {
    // State
    view: ui.view,
    navStack: ui.navStack,
    navIndex: ui.navIndex,
    query: ui.query,
    results: ui.results,
    searching: ui.searching,
    liked: library.liked,
    history: library.history,
    queue: library.queue,
    current: playback.current,
    playing: playback.playing,
    position: playback.position,
    duration: playback.duration,
    volume: playback.volume,
    muted: playback.muted,
    shuffle: playback.shuffle,
    repeat: playback.repeat,
    lyricsRaw: ui.lyricsRaw, // Note: lyricsRaw is not in any store yet
    lyricsOpen: ui.lyricsOpen,
    lyricsAutoFollow: ui.lyricsAutoFollow,
    lyricsUserScrolling: ui.lyricsUserScrolling,
    lyricsRequestId: ui.lyricsRequestId,
    queueOpen: ui.queueOpen,
    expandedPlayerOpen: ui.expandedPlayerOpen,
    menu: ui.menu,
    toast: ui.toast,
    toastAction: ui.toastAction,
    booted: ui.booted,
    bridgeError: ui.bridgeError,

    // Actions
    navigate: ui.navigate,
    goBack: ui.goBack,
    goForward: ui.goForward,
    setQuery: ui.setQuery,
    playTrack,
    toggle: playback.togglePlay,
    next,
    prev,
    seekRatio: playback.seekRatio,
    seekSec: playback.seekSec,
    setVolume: playback.setVolume,
    toggleMute: playback.toggleMute,
    toggleShuffle: playback.toggleShuffle,
    cycleRepeat: playback.cycleRepeat,
    toggleLike: library.toggleLike,
    playNextInQueue: library.playNextInQueue,
    addToQueue: library.addToQueue,
    removeFromQueue: library.removeFromQueue,
    playQueueIndex: (index: number) => {
      const track = library.queue[index];
      if (track) {
        playback.setCurrent(track);
        playback.setPlaying(true);
        playback.setPosition(0);
        ui.setQueueOpen(false);
      } else {
        ui.setQueueOpen(false);
      }
      bridge()?.playFromQueue(index);
    },
    clearQueue,
    openMenu: ui.openMenu,
    closeMenu: ui.closeMenu,
    setLyricsOpen: ui.setLyricsOpen,
    setLyricsAutoFollow: ui.setLyricsAutoFollow,
    setLyricsUserScrolling: ui.setLyricsUserScrolling,
    setLyricsRequestId: ui.setLyricsRequestId,
    setQueueOpen: ui.setQueueOpen,
    setExpandedPlayerOpen: ui.setExpandedPlayerOpen,
    toastNote: ui.toastNote,
    loadFavorites: library.loadFavorites,
    patch: (partial: any) => {
      // Distribute patch to appropriate stores
      const playbackKeys = ["current", "playing", "position", "duration", "volume", "muted", "shuffle", "repeat"];
      const uiKeys = ["view", "navStack", "navIndex", "query", "results", "searching", "queueOpen", "expandedPlayerOpen", "lyricsOpen", "lyricsAutoFollow", "lyricsUserScrolling", "lyricsRequestId", "menu", "toast", "toastAction", "booted", "bridgeError"];
      const libraryKeys = ["liked", "history", "queue"];

      const playbackPatch: any = {};
      const uiPatch: any = {};
      const libraryPatch: any = {};

      for (const [key, value] of Object.entries(partial)) {
        if (playbackKeys.includes(key)) playbackPatch[key] = value;
        else if (uiKeys.includes(key)) uiPatch[key] = value;
        else if (libraryKeys.includes(key)) libraryPatch[key] = value;
      }

      if (Object.keys(playbackPatch).length) usePlaybackStore.getState().patch?.(playbackPatch);
      if (Object.keys(uiPatch).length) useUIStore.getState().patch?.(uiPatch);
      if (Object.keys(libraryPatch).length) useLibraryStore.getState().patch?.(libraryPatch);
    },
  };
}