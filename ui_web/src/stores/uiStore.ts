import { create } from "zustand";
import { useSettings } from "@/lib/settings";
import type { View, TrackMeta } from "@/lib/types";
import { t } from "@/lib/i18n";

type Menu = { track: TrackMeta; x: number; y: number } | null;

type UIState = {
  view: View;
  navStack: View[];
  navIndex: number;
  query: string;
  results: TrackMeta[];
  searching: boolean;
  queueOpen: boolean; // Right-side Up Next queue visibility
  lyricsOpen: boolean; // Central Lyrics view visibility
  lyricsAutoFollow: boolean;
  lyricsUserScrolling: boolean;
  lyricsRequestId: number | null;
  lyricsRaw: string | null;
  menu: Menu;
  /** The track the share sheet is open for, if any. */
  shareTrack: TrackMeta | null;
  toast: string | null;
  toastAction: { label: string; run: () => void } | null;
  booted: boolean;
  bridgeError: string | null;
  patch: (partial: Partial<UIState>) => void;
  downloads: TrackMeta[];
  setDownloads: (tracks: TrackMeta[]) => void;
};

type UIActions = {
  navigate: (view: View) => void;
  goBack: () => void;
  goForward: () => void;
  setQuery: (q: string) => void;
  setResults: (results: TrackMeta[]) => void;
  setSearching: (searching: boolean) => void;
  setQueueOpen: (open: boolean) => void;
  setLyricsOpen: (open: boolean) => void;
  setLyricsAutoFollow: (v: boolean) => void;
  setLyricsUserScrolling: (v: boolean) => void;
  setLyricsRequestId: (id: number | null) => void;
  setLyricsRaw: (raw: string | null) => void;
  openMenu: (menu: Menu) => void;
  closeMenu: () => void;
  setShareTrack: (track: TrackMeta | null) => void;
  toastNote: (text: string) => void;
  /** A note that carries an Undo-style button and stays up long enough to use it. */
  toastWithAction: (text: string, action: { label: string; run: () => void }, ms?: number) => void;
  setBooted: (booted: boolean) => void;
  setBridgeError: (error: string | null) => void;
  setDownloads: (tracks: TrackMeta[]) => void;
  addDownload: (track: TrackMeta) => void;
  removeDownload: (trackId: string) => void;
};

let toastTimer: ReturnType<typeof setTimeout> | undefined;

function viewKey(v: View): string {
  return v.name === "album" || v.name === "artist" ? `${v.name}:${v.key}` : v.name;
}

export const useUIStore = create<UIState & UIActions>()((set, get) => ({
  view: { name: "home" },
  navStack: [{ name: "home" }],
  navIndex: 0,
  query: "",
  results: [],
  searching: false,
  queueOpen: false,
  lyricsOpen: false,
  lyricsAutoFollow: true,
  lyricsUserScrolling: false,
  lyricsRequestId: null,
  lyricsRaw: null,
  menu: null,
  shareTrack: null,
  toast: null,
  toastAction: null,
  booted: false,
  bridgeError: null,
  downloads: [],

  navigate: (view) => {
    const { navStack, navIndex } = get();
    if (viewKey(navStack[navIndex] ?? { name: "home" }) === viewKey(view)) {
      return set({ view, menu: null });
    }
    const stack = navStack.slice(0, navIndex + 1);
    stack.push(view);
    const trimmed = stack.slice(-40);
    set({ view, menu: null, navStack: trimmed, navIndex: trimmed.length - 1 });
  },
  goBack: () => {
    const { navStack, navIndex } = get();
    if (navIndex <= 0) return;
    const i = navIndex - 1;
    set({ view: navStack[i], navIndex: i, menu: null });
  },
  goForward: () => {
    const { navStack, navIndex } = get();
    if (navIndex >= navStack.length - 1) return;
    const i = navIndex + 1;
    set({ view: navStack[i], navIndex: i, menu: null });
  },

  setQuery: (query) => set({ query }),
  setResults: (results) => set({ results }),
  setSearching: (searching) => set({ searching }),

  setQueueOpen: (queueOpen) => set({ queueOpen }),
  setLyricsOpen: (lyricsOpen) => set({ lyricsOpen, menu: null }),
  setLyricsAutoFollow: (v) => set({ lyricsAutoFollow: Boolean(v), lyricsUserScrolling: false }),
  setLyricsUserScrolling: (v) => set({ lyricsUserScrolling: Boolean(v) }),
  setLyricsRequestId: (id) => set({ lyricsRequestId: id }),
  setLyricsRaw: (raw) => set({ lyricsRaw: raw }),

  openMenu: (menu) => set({ menu }),
  closeMenu: () => set({ menu: null }),
  setShareTrack: (shareTrack) => set({ shareTrack }),

  toastNote: (text) => {
    set({ toast: text, toastAction: null });
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => set({ toast: null }), 1700);
  },

  toastWithAction: (text, action, ms = 5000) => {
    set({ toast: text, toastAction: action });
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => set({ toast: null, toastAction: null }), ms);
  },

  setBooted: (booted) => set({ booted }),
  setBridgeError: (bridgeError) => set({ bridgeError }),
  setDownloads: (tracks: TrackMeta[]) => set({ downloads: tracks }),
  addDownload: (track: TrackMeta) => set((state) => {
    const already = state.downloads.some((t) => t.id === track.id);
    if (already) return state;
    set({ downloads: [...state.downloads, track] });
  }),
  removeDownload: (trackId: string) => set((state) => ({
    downloads: state.downloads.filter((t) => t.id !== trackId),
    shareTrack: state.shareTrack?.id === trackId ? null : state.shareTrack,
  })),
  patch: (partial) => set((state) => ({ ...state, ...partial })),
}));
