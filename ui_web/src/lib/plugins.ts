import { create } from "zustand";
import { getBridge } from "@/lib/client";

/** One row of the plugins panel, mirrored from what the core reported. */
export type SourceInfo = {
  id: string;
  name: string;
  description: string;
  /** "api" talks a site's own HTTP API; "yt-dlp" shells out to yt-dlp. */
  kind: string;
  enabled: boolean;
  /** YouTube is the default and the anchor every bare track id resolves to. */
  default: boolean;
};

type PluginsState = {
  open: boolean;
  sources: SourceInfo[];
  loading: boolean;
  /** The id whose toggle is in flight, so only that row shows a busy state
      instead of dimming the whole panel for one slow write. */
  pending: string | null;
  setOpen: (open: boolean) => void;
  load: () => Promise<void>;
  toggle: (id: string, next: boolean) => Promise<void>;
};

export const usePlugins = create<PluginsState>()((set, get) => ({
  open: false,
  sources: [],
  loading: false,
  pending: null,

  setOpen: (open) => {
    set({ open });
    // The list is small and the core already holds it, so it is refetched on
    // every open rather than cached — a source added or removed elsewhere is
    // reflected without a restart.
    if (open) void get().load();
  },

  load: async () => {
    const b = getBridge();
    if (!b || typeof b.listSources !== "function") {
      // Browser mode, or a core too old to answer. An empty panel that says
      // nothing would be worse than one that says why.
      set({ sources: [], loading: false });
      return;
    }
    set({ loading: true });
    try {
      const raw = await b.listSources();
      const parsed = raw ? JSON.parse(raw) : [];
      set({ sources: Array.isArray(parsed) ? parsed : [] });
    } catch (e) {
      console.error("[Melyar] listSources failed", e);
      set({ sources: [] });
    } finally {
      set({ loading: false });
    }
  },

  toggle: async (id, next) => {
    const b = getBridge();
    if (!b || typeof b.setSourceEnabled !== "function") return;

    set({ pending: id });
    try {
      // The core owns the truth. Its answer decides the row: a refusal (an
      // unknown id, or an attempt to switch YouTube off) leaves the switch
      // where it was rather than showing a state that is not real.
      const accepted = await b.setSourceEnabled(id, next);
      if (accepted === false) {
        await get().load();
        return;
      }
      set({
        sources: get().sources.map((s) => (s.id === id ? { ...s, enabled: next } : s)),
      });
    } catch (e) {
      console.error("[Melyar] setSourceEnabled failed", e);
      await get().load();
    } finally {
      set({ pending: null });
    }
  },
}));
