import { create } from "zustand";
import { getBridge } from "@/lib/client";
import { useUIStore } from "@/store";
import { t, tCount } from "@/lib/i18n";

export type ImportProgress = {
  current: number;
  total: number;
  matched: number;
  skipped: number;
  done: boolean;
};

const EMPTY: ImportProgress = { current: 0, total: 0, matched: 0, skipped: 0, done: false };

/** The native dialog is one at a time; a second click must not arm a second
 * pick, or two answers would arrive for one question. */
let pickInFlight = false;

type ImportState = {
  progress: ImportProgress;
  running: boolean;
  /** Ask the user for an export file and start the import. */
  start: () => Promise<void>;
  cancel: () => void;
  reset: () => void;
};

export const useLibraryImport = create<ImportState>()((set, get) => ({
  progress: EMPTY,
  running: false,

  start: async () => {
    const b = getBridge();
    if (!b) return;
    if (pickInFlight) return;

    // The native file dialog is opened by the QML side, because the web
    // layer cannot reach the file system. The bridge asks for it and
    // answers with the picked path.
    if (typeof b.requestFilePick !== "function") return;

    pickInFlight = true;
    try {
      const path = await new Promise<string>((resolve) => {
        let settled = false;
        let timer: ReturnType<typeof setTimeout> | undefined;

        // The handler is detached however the promise ends. Leaving it attached
        // meant a cancelled dialog kept it alive, so the next pick woke both the
        // stale and the fresh one and the file was imported twice.
        const done = (value: string) => {
          if (settled) return;
          settled = true;
          if (timer !== undefined) clearTimeout(timer);
          b.filePicked?.disconnect?.(handler);
          resolve(value);
        };
        const handler = (picked: string) => done(String(picked || ""));

        timer = setTimeout(() => done(""), 5 * 60 * 1000);
        b.filePicked?.connect?.(handler);
        b.requestFilePick();
      });

      if (!path) return;
      if (typeof b.importLibrary !== "function") return;

      b.importLibrary(path);
      set({ running: true, progress: { ...EMPTY } });
    } finally {
      pickInFlight = false;
    }
  },

  cancel: () => {
    const b = getBridge();
    if (b && typeof b.cancelImport === "function") b.cancelImport();
    set({ running: false });
  },

  reset: () => set({ progress: EMPTY, running: false }),
}));

/**
 * Wire the bridge's progress signal to the store. Called once, after the
 * bridge connects.
 */
export function bindImportProgress() {
  const b = getBridge();
  if (!b) return;

  b.importProgressChanged?.connect(() => {
    const raw = b.importProgress || {};
    const progress: ImportProgress = {
      current: Number(raw.current) || 0,
      total: Number(raw.total) || 0,
      matched: Number(raw.matched) || 0,
      skipped: Number(raw.skipped) || 0,
      done: Boolean(raw.done),
    };

    useLibraryImport.setState({ progress, running: !progress.done });

    if (progress.done) {
      // The core holds the imported tracks, so the library has to be reloaded.
      if (typeof b.loadFavorites === "function") b.loadFavorites();

      useUIStore.getState().toastNote(
        tCount("import.finished", progress.matched, {
          matched: progress.matched,
          skipped: progress.skipped,
        }),
      );
    }
  });
}
