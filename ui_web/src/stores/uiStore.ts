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
  queueOpen: boolean;
  expandedPlayerOpen: boolean;
  lyricsOpen: boolean;
  lyricsAutoFollow: boolean;
  lyricsUserScrolling: boolean;
  lyricsRequestId: number | null;
  lyricsRaw: string | null;
  menu: Menu;
  toast: string | null;
  toastAction: { label: string; run: () => void } | null;
  booted: boolean;
  bridgeError: string | null;
  patch: (partial: Partial<UIState>) => void;
};

type UIActions = {
  navigate: (view: View) => void;
  goBack: () => void;
  goForward: () => void;
  setQuery: (q: string) => void;
  setResults: (results: TrackMeta[]) => void;
  setSearching: (searching: boolean) => void;
  setQueueOpen: (open: boolean) => void;
  setExpandedPlayerOpen: (open: boolean) => void;
  setLyricsOpen: (open: boolean) => void;
  setLyricsAutoFollow: (v: boolean) => void;
  setLyricsUserScrolling: (v: boolean) => void;
  setLyricsRequestId: (id: number | null) => void;
  setLyricsRaw: (raw: string | null) => void;
  openMenu: (menu: Menu) => void;
  closeMenu: () => void;
  toastNote: (text: string) => void;
  /** A note that carries an Undo-style button and stays up long enough to use it. */
  toastWithAction: (text: string, action: { label: string; run: () => void }, ms?: number) => void;
  setBooted: (booted: boolean) => void;
  setBridgeError: (error: string | null) => void;
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
  expandedPlayerOpen: false,
  lyricsOpen: false,
  lyricsAutoFollow: true,
  lyricsUserScrolling: false,
  lyricsRequestId: null,
  lyricsRaw: null,
  menu: null,
  toast: null,
  toastAction: null,
  booted: false,
  bridgeError: null,

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
  setExpandedPlayerOpen: (expandedPlayerOpen) => set({ expandedPlayerOpen }),
  setLyricsOpen: (lyricsOpen) => set({ lyricsOpen, menu: null }),
  setLyricsAutoFollow: (v) => set({ lyricsAutoFollow: Boolean(v), lyricsUserScrolling: false }),
  setLyricsUserScrolling: (v) => set({ lyricsUserScrolling: Boolean(v) }),
  setLyricsRequestId: (id) => set({ lyricsRequestId: id }),
  setLyricsRaw: (raw) => set({ lyricsRaw: raw }),

  openMenu: (menu) => set({ menu }),
  closeMenu: () => set({ menu: null }),

  toastNote: (text) => {
    set({ toast: text, toastAction: null });
    if (toastTimer) clearTimeout(toastTimer);
    toastTimer = setTimeout(() => set({ toast: null }), 1700);
  },

  toastWithAction: (text, action, ms = 5000) => {
    set({ toast: text, toastAction: action });
    if (toastTimer) clearTimeout(toastTimer);
    // An undo affordance needs more than the 1.7s a plain note gets: the
    // window has to be long enough to notice and aim at the button.
    toastTimer = setTimeout(() => set({ toast: null, toastAction: null }), ms);
  },

  setBooted: (booted) => set({ booted }),
  setBridgeError: (bridgeError) => set({ bridgeError }),
  patch: (partial) => set((state) => ({ ...state, ...partial })),
}));