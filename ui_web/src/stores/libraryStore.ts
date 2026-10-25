import { create } from "zustand";
import type { TrackMeta } from "@/lib/types";
import { t } from "@/lib/i18n";
import { usePlaybackStore } from "./playbackStore";
import { useUIStore } from "./uiStore";

type LibraryState = {
  liked: TrackMeta[];
  history: TrackMeta[];
  queue: TrackMeta[];
  patch: (partial: Partial<LibraryState>) => void;
};

type LibraryActions = {
  setLiked: (liked: TrackMeta[]) => void;
  setHistory: (history: TrackMeta[]) => void;
  setQueue: (queue: TrackMeta[]) => void;
  addToQueue: (track: TrackMeta) => void;
  removeFromQueue: (track: TrackMeta) => void;
  playNextInQueue: (track: TrackMeta) => void;
  clearQueue: () => void;
  toggleLike: (track: TrackMeta) => void;
  loadFavorites: () => void;
};


function bridge(): any | null {
  return (typeof window !== "undefined" && window.bridge) || null;
}

export const useLibraryStore = create<LibraryState & LibraryActions>()((set, get) => ({
  liked: [],
  history: [],
  queue: [],

  setLiked: (liked) => set({ liked }),
  setHistory: (history) => set({ history }),
  setQueue: (queue) => set({ queue }),

  addToQueue: (track) => {
    const b = bridge();
    if (!b) return;
    b.addToQueue(track.id);
    const ui = useUIStore.getState();
    ui.patch({ menu: null });
    ui.toastNote(t("toast.queued"));
  },

  removeFromQueue: (track) => {
    const b = bridge();
    if (!b) return;
    const index = get().queue.findIndex((q) => q.id === track.id);
    if (index < 0) return;
    b.removeFromQueue(index);
    // No toast: the row leaves the panel in the same frame, which is the
    // confirmation. The menu that asked for it does have to close, though.
    useUIStore.getState().patch({ menu: null });
  },

  playNextInQueue: (track) => {
    const b = bridge();
    if (!b) return;
    // The core inserts after the track that is playing, so the wording has
    // to say "next" rather than "queued" - adding to the end would promise
    // something the engine does not do.
    b.playNextInQueue(track.id);
    const ui = useUIStore.getState();
    ui.patch({ menu: null });
    ui.toastNote(t("toast.playNext"));
  },

  clearQueue: () => {
    const b = bridge();
    if (!b) return;
    const queue = get().queue;
    if (queue.length === 0) return;

    // Snapshot before the bridge clears its mirror, so Undo can restore the
    // exact list and the position within it. replaceQueue only swaps the
    // list — it never starts playback — so the undo is non-destructive.
    const snapshot = [...queue];
    const start = Math.max(
      0,
      snapshot.findIndex((track) => track.id === usePlaybackStore.getState().current?.id),
    );
    b.clearQueue();
    set({ queue: [] });

    useUIStore.getState().toastWithAction(t("toast.queueCleared"), {
      label: t("toast.undo"),
      run: () => {
        b.replaceQueue(
          snapshot.map((track) => ({
            id: track.id,
            title: track.title,
            artist: track.artist,
            album: track.album,
            duration: track.duration,
            thumbnail: track.thumbnail,
          })),
          start,
        );
        useUIStore.getState().patch({ toast: null, toastAction: null });
      },
    });
  },

  toggleLike: (track) => {
    const b = bridge();
    if (!b) return;
    const liked = get().liked;
    const isLiked = liked.some((t) => t.id === track.id);
    try {
      b.toggleFavorite(track.id, track.title, track.artist, track.album, track.duration, track.thumbnail);
    } catch (e) {
      console.error("toggleFavorite failed", e);
    }
    set({
      liked: isLiked ? liked.filter((t) => t.id !== track.id) : [track, ...liked],
    });
    const ui = useUIStore.getState();
    ui.patch({ menu: null });
    ui.toastNote(t(isLiked ? "toast.unliked" : "toast.liked"));
  },

  loadFavorites: () => {
    const b = bridge();
    if (b && typeof b.loadFavorites === "function") b.loadFavorites();
  },
  patch: (partial) => set((state) => ({ ...state, ...partial })),
}));
