import { create } from "zustand";
import { register } from "@/lib/registry";
import { useLibraryStore } from "@/store";
import type { TrackMeta } from "@/lib/types";

export type ImportCandidate = {
  trackId: string;
  title: string;
  artist: string;
  album: string;
  duration: number;
  thumbnail: string;
  source: string;
  availability: string;
};

export type ImportPreviewItem = {
  itemId: string;
  source: string;
  sourceId: string;
  trackId: string;
  title: string;
  artist: string;
  album: string;
  duration: number;
  thumbnail: string;
  trackNo: number;
  discNo: number;
  year: string;
  isrc: string;
  filePath: string;
  availability: string;
  matchState: string;
  confidence: number;
  duplicate: boolean;
  playable: boolean;
  candidates: ImportCandidate[];
};

export type ImportPreview = {
  id: string;
  kind: string;
  provider: string;
  format: string;
  name: string;
  path: string;
  mode: string;
  copyTarget: string;
  counts: {
    tracks: number;
    albums: number;
    artists: number;
    playlists: number;
    duplicates: number;
    unreadable: number;
    matched: number;
    review: number;
    unavailable: number;
  };
  items: ImportPreviewItem[];
  truncated: boolean;
  playlists: { name: string; tracks: number }[];
  skipped: number;
  warnings: string[];
};

export type ImportSummary = {
  imported: number;
  skipped: number;
  failed: number;
  duplicates: number;
  copied: number;
  total: number;
  playlists: number;
  provider: string;
};

export type ImportFolder = {
  path: string;
  mode: string;
  autoScan: boolean;
  lastScan: number;
  tracks: number;
  albums: number;
  artists: number;
  accessible: boolean;
  status: string;
};

export type ImportPlaylistInfo = {
  id: string;
  source: string;
  name: string;
  thumbnail: string;
  tracks: number;
  importedAt?: number;
};

export type SpotifyStatus = {
  status: string;
  configured: boolean;
  connected: boolean;
  account: string;
  redirectUri: string;
  scopes: string[];
  reason?: string | null;
};

export type ProviderState = {
  status: string;
  reason?: string | null;
  configured?: boolean;
  connected?: boolean;
  account?: string;
  redirectUri?: string;
  scopes?: string[];
  folders?: number;
  tracks?: number;
  formats?: string[];
};

export type ImportSnapshot = {
  folders: ImportFolder[];
  playlists: ImportPlaylistInfo[];
  stats: { items: number; localItems: number; playable: number; metadataOnly: number };
  providers: Record<string, ProviderState>;
};

type ImportError = { code: string; message: string };

export type WizardStep = 1 | 2 | 3 | 4 | 5 | 6 | 7;

type Progress = {
  phase: string;
  current: number;
  total: number;
  detail?: Record<string, unknown>;
};

type ImportCenterState = {
  snapshot: ImportSnapshot | null;
  loading: boolean;
  error: ImportError | null;
  itemErrors: string[];
  preview: ImportPreview | null;
  wizardOpen: boolean;
  step: WizardStep;
  selected: string[];
  duplicatePolicy: "skip" | "merge" | "keep";
  importPlaylists: boolean;
  progress: Progress | null;
  running: boolean;
  validating: boolean;
  completion: ImportSummary | null;
  spotifyOptions: { tracks: boolean; albums: boolean; playlists: boolean };
  spotifyConfigOpen: boolean;
  spotifyBusy: boolean;
  spotifyAuthPending: boolean;
  pendingManagedCopy: boolean;
  copyTarget: string;
  changingFolder: string | null;

  refresh: () => Promise<void>;
  loadLibrary: () => Promise<void>;
  openFolderPicker: () => void;
  openWizard: () => void;
  startFolderScan: (path: string, mode: "index" | "copy", copyTarget?: string) => void;
  openExportFilePicker: () => void;
  startFilePreview: (path: string) => Promise<void>;
  openManagedFolderPicker: () => void;
  handleDirectoryPicked: (path: string) => void;
  handleFilePicked: (path: string, kind: string) => void;
  setStep: (step: WizardStep) => void;
  toggleItem: (itemId: string) => void;
  selectAll: () => void;
  clearSelection: () => void;
  pickCandidate: (itemId: string, candidateId: string) => void;
  setDuplicatePolicy: (policy: "skip" | "merge" | "keep") => void;
  setImportPlaylists: (value: boolean) => void;
  commit: () => void;
  cancel: () => void;
  closeWizard: () => void;
  clearError: () => void;
  rescanFolder: (path: string) => void;
  removeFolder: (path: string) => void;
  setAutoScan: (path: string, enabled: boolean) => void;
  changeFolder: (path: string) => void;
  setSpotifyOption: (key: "tracks" | "albums" | "playlists", value: boolean) => void;
  setSpotifyConfigOpen: (open: boolean) => void;
  saveSpotifyClientId: (clientId: string) => void;
  connectSpotify: () => void;
  disconnectSpotify: () => void;
  previewSpotify: () => void;
  openExternal: (url: string) => void;
  handleEvent: (json: string) => void;
};

let bridge: any = null;

function b(): any {
  return bridge ?? (typeof window !== "undefined" ? (window as any).bridge : null);
}

function parse<T>(raw: string, fallback: T): T {
  try {
    return JSON.parse(raw) as T;
  } catch {
    return fallback;
  }
}

function trackFromImported(item: any): TrackMeta {
  const playable = Boolean(item.playable) && Boolean(item.trackId);
  const id = playable ? String(item.trackId) : String(item.itemId);
  return register({
    id,
    title: String(item.title || ""),
    artist: String(item.artist || ""),
    album: String(item.album || ""),
    duration: Number(item.duration) || 0,
    thumbnail: String(item.thumbnail || ""),
    playable,
    source: String(item.source || ""),
    availability: item.availability as TrackMeta["availability"],
  });
}

function dedupeTracks(tracks: TrackMeta[]): TrackMeta[] {
  const seen = new Set<string>();
  return tracks.filter((track) => {
    if (seen.has(track.id)) return false;
    seen.add(track.id);
    return true;
  });
}

export const useImportCenter = create<ImportCenterState>()((set, get) => ({
  snapshot: null,
  loading: false,
  error: null,
  itemErrors: [],
  preview: null,
  wizardOpen: false,
  step: 1,
  selected: [],
  duplicatePolicy: "skip",
  importPlaylists: true,
  progress: null,
  running: false,
  validating: false,
  completion: null,
  spotifyOptions: { tracks: true, albums: true, playlists: true },
  spotifyConfigOpen: false,
  spotifyBusy: false,
  spotifyAuthPending: false,
  pendingManagedCopy: false,
  copyTarget: "",
  changingFolder: null,

  refresh: async () => {
    const api = b();
    if (!api || typeof api.importCenterState !== "function") return;
    set({ loading: true });
    try {
      const snapshot = parse<ImportSnapshot>(api.importCenterState(), {
        folders: [],
        playlists: [],
        stats: { items: 0, localItems: 0, playable: 0, metadataOnly: 0 },
        providers: {},
      });
      set({ snapshot });
    } finally {
      set({ loading: false });
    }
  },

  loadLibrary: async () => {
    const api = b();
    if (!api || typeof api.importListItems !== "function") return;
    const raw = parse<any[]>(api.importListItems(), []);
    const imported = dedupeTracks(raw.map(trackFromImported));
    useLibraryStore.setState({ imported });
  },

  openFolderPicker: () => {
    const api = b();
    if (!api || typeof api.requestDirectoryPick !== "function") return;
    set({ pendingManagedCopy: false, copyTarget: "", changingFolder: null });
    api.requestDirectoryPick();
  },

  openWizard: () => {
    set({
      wizardOpen: true,
      step: 1,
      preview: null,
      selected: [],
      completion: null,
      error: null,
      itemErrors: [],
      progress: null,
      validating: false,
      running: false,
    });
  },

  startFolderScan: (path, mode, copyTarget) => {
    const api = b();
    if (!api || typeof api.importScanFolder !== "function") return;
    const options = {
      mode,
      copyTarget: copyTarget || "",
      autoCommit: false,
    };
    set({
      error: null,
      itemErrors: [],
      completion: null,
      progress: { phase: "scanning", current: 0, total: 0 },
      running: true,
      validating: true,
      preview: null,
      wizardOpen: true,
      step: 2,
    });
    api.importScanFolder(path, JSON.stringify(options));
  },

  openExportFilePicker: () => {
    const api = b();
    if (!api || typeof api.requestImportFilePick !== "function") return;
    api.requestImportFilePick("file");
  },

  startFilePreview: async (path) => {
    const api = b();
    if (!api || typeof api.importPreviewFile !== "function") return;
    set({
      error: null,
      itemErrors: [],
      completion: null,
      preview: null,
      wizardOpen: true,
      step: 2,
      validating: true,
      running: false,
      progress: null,
    });
    // Let the validating state paint before the synchronous parse.
    await new Promise((resolve) => setTimeout(resolve, 30));
    let result: any = null;
    try {
      result = parse<any>(api.importPreviewFile(path), null);
    } catch (err) {
      set({
        validating: false,
        error: { code: "read_failed", message: String(err) },
      });
      return;
    }
    if (!result) {
      set({
        validating: false,
        error: { code: "invalid_export", message: "The file could not be read." },
      });
      return;
    }
    if (result.error) {
      set({ validating: false, error: result.error });
      return;
    }
    get().handleEvent(JSON.stringify({ type: "preview", preview: result }));
  },

  openManagedFolderPicker: () => {
    const api = b();
    if (!api || typeof api.requestImportFilePick !== "function") return;
    set({ pendingManagedCopy: true, copyTarget: "", changingFolder: null });
    api.requestImportFilePick("managed-folder");
  },

  handleDirectoryPicked: (path) => {
    if (!path) {
      set({ changingFolder: null, pendingManagedCopy: false });
      return;
    }
    const state = get();
    if (state.changingFolder) {
      const old = state.changingFolder;
      const api = b();
      api?.importRemoveFolder?.(old);
      set({ changingFolder: null });
      state.startFolderScan(path, "index");
      return;
    }
    if (state.pendingManagedCopy) {
      const target = state.copyTarget;
      set({ pendingManagedCopy: false });
      if (target) state.startFolderScan(path, "copy", target);
      return;
    }
    state.startFolderScan(path, "index");
  },

  handleFilePicked: (path, kind) => {
    if (!path) {
      set({ pendingManagedCopy: false, copyTarget: "" });
      return;
    }
    if (kind === "managed-folder") {
      // Step one of the copy flow: remember where the copies go, then ask
      // for the source folder. The source is only read, never moved.
      set({ copyTarget: path, pendingManagedCopy: true });
      b()?.requestDirectoryPick?.();
      return;
    }
    void get().startFilePreview(path);
  },

  setStep: (step) => set({ step }),

  toggleItem: (itemId) =>
    set((state) => ({
      selected: state.selected.includes(itemId)
        ? state.selected.filter((id) => id !== itemId)
        : [...state.selected, itemId],
    })),

  selectAll: () =>
    set((state) => ({
      selected: state.preview ? state.preview.items.map((item) => item.itemId) : [],
    })),

  clearSelection: () => set({ selected: [] }),

  pickCandidate: (itemId, candidateId) => {
    const state = get();
    const api = b();
    if (!state.preview) return;
    const ok = api?.importPreviewPick?.(state.preview.id, itemId, candidateId);
    if (!ok) return;
    set({
      preview: {
        ...state.preview,
        items: state.preview.items.map((item) => {
          if (item.itemId !== itemId) return item;
          const candidate = item.candidates.find((c) => c.trackId === candidateId);
          if (!candidate) return item;
          return {
            ...item,
            trackId: candidate.trackId,
            matchState: "manual",
            confidence: 100,
            playable: true,
            availability: candidate.availability,
          };
        }),
      },
    });
  },

  setDuplicatePolicy: (duplicatePolicy) =>
    set((state) => {
      if (!state.preview) return { duplicatePolicy };
      const duplicateIds = state.preview.items
        .filter((item) => item.duplicate)
        .map((item) => item.itemId);
      if (duplicatePolicy === "skip") {
        const duplicates = new Set(duplicateIds);
        return {
          duplicatePolicy,
          selected: state.selected.filter((id) => !duplicates.has(id)),
        };
      }
      if (duplicatePolicy === "keep") {
        return {
          duplicatePolicy,
          selected: Array.from(new Set([...state.selected, ...duplicateIds])),
        };
      }
      return { duplicatePolicy };
    }),
  setImportPlaylists: (importPlaylists) => set({ importPlaylists }),

  commit: () => {
    const state = get();
    const api = b();
    if (!state.preview || !api || typeof api.importCommit !== "function") return;
    api.importCommit(
      JSON.stringify({
        previewId: state.preview.id,
        itemIds: state.selected,
        skipDuplicates: state.duplicatePolicy === "skip",
        duplicatePolicy: state.duplicatePolicy,
        importPlaylists: state.importPlaylists,
      }),
    );
    set({
      running: true,
      step: 7,
      completion: null,
      itemErrors: [],
      progress: {
        phase: "importing",
        current: 0,
        total: state.selected.length,
      },
    });
  },

  cancel: () => {
    b()?.importCancel?.();
  },

  closeWizard: () => {
    if (get().running) return;
    set({ wizardOpen: false, step: 1, preview: null, selected: [], completion: null });
  },

  clearError: () => set({ error: null }),

  rescanFolder: (path) => get().startFolderScan(path, "index"),

  removeFolder: (path) => {
    b()?.importRemoveFolder?.(path);
  },

  setAutoScan: (path, enabled) => {
    b()?.importSetFolderAutoScan?.(path, enabled);
    void get().refresh();
  },

  changeFolder: (path) => {
    set({ changingFolder: path });
    b()?.requestDirectoryPick?.();
  },

  setSpotifyOption: (key, value) =>
    set((state) => ({ spotifyOptions: { ...state.spotifyOptions, [key]: value } })),

  setSpotifyConfigOpen: (spotifyConfigOpen) => set({ spotifyConfigOpen }),

  saveSpotifyClientId: (clientId) => {
    const api = b();
    if (!api || typeof api.importSpotifySetClientId !== "function") return;
    const ok = api.importSpotifySetClientId(clientId);
    if (ok) {
      set({ spotifyConfigOpen: false, error: null });
      void get().refresh();
    }
  },

  connectSpotify: () => {
    const api = b();
    if (!api || typeof api.importSpotifyConnect !== "function") return;
    set({ spotifyBusy: true, spotifyAuthPending: true, error: null });
    api.importSpotifyConnect();
  },

  disconnectSpotify: () => {
    const api = b();
    if (!api || typeof api.importSpotifyDisconnect !== "function") return;
    set({ spotifyBusy: true });
    api.importSpotifyDisconnect();
  },

  previewSpotify: () => {
    const api = b();
    if (!api || typeof api.importSpotifyPreview !== "function") return;
    const state = get();
    set({
      error: null,
      itemErrors: [],
      completion: null,
      preview: null,
      wizardOpen: true,
      step: 2,
      validating: true,
      running: true,
      progress: { phase: "spotify_library", current: 0, total: 0 },
    });
    api.importSpotifyPreview(JSON.stringify(state.spotifyOptions));
  },

  openExternal: (url) => {
    b()?.openExternalUrl?.(url);
  },

  handleEvent: (json) => {
    const event = parse<any>(json, null);
    if (!event || !event.type) return;
    const state = get();
    switch (event.type) {
      case "progress":
        set({
          progress: {
            phase: String(event.phase || ""),
            current: Number(event.current) || 0,
            total: Number(event.total) || 0,
            detail: event.detail || {},
          },
        });
        break;
      case "preview": {
        const preview = event.preview as ImportPreview;
        if (!preview || !preview.id) return;
        set({
          preview,
          wizardOpen: true,
          step: 3,
          validating: false,
          running: false,
          progress: null,
          selected: preview.items.filter((item) => !item.duplicate || state.duplicatePolicy !== "skip").map((item) => item.itemId),
        });
        break;
      }
      case "complete": {
        const summary = event.summary as ImportSummary;
        set({
          running: false,
          validating: false,
          completion: summary,
          step: 7,
          progress: null,
        });
        void get().loadLibrary();
        void get().refresh();
        break;
      }
      case "error":
        set({
          running: false,
          validating: false,
          spotifyBusy: false,
          spotifyAuthPending: false,
          error: {
            code: String(event.code || "unknown"),
            message: String(event.message || ""),
          },
        });
        break;
      case "cancelled":
        set({
          running: false,
          validating: false,
          progress: null,
          error: {
            code: "cancelled",
            message: String(event.summary ? JSON.stringify(event.summary) : ""),
          },
        });
        break;
      case "item_error":
        set((current) => ({
          itemErrors: [...current.itemErrors, String(event.message || "")].slice(-20),
        }));
        break;
      case "spotify_auth_url":
        set({ spotifyBusy: false, spotifyAuthPending: true });
        if (event.url) b()?.openExternalUrl?.(String(event.url));
        break;
      case "provider_connected":
      case "provider_state":
      case "provider_disconnected":
        set({ spotifyBusy: false, spotifyAuthPending: false });
        void get().refresh();
        break;
      case "folder_removed":
        void get().refresh();
        void get().loadLibrary();
        break;
      default:
        break;
    }
  },
}));

/** Wire the bridge signals once, at boot. */
export function attachImportCenter(api: any) {
  bridge = api;
  if (!api) return;
  useImportCenter.setState({});
  api.importEvent?.connect?.((json: string) => {
    useImportCenter.getState().handleEvent(json);
  });
  api.directoryPicked?.connect?.((path: string) => {
    useImportCenter.getState().handleDirectoryPicked(path);
  });
  api.importFilePicked?.connect?.((path: string, kind: string) => {
    useImportCenter.getState().handleFilePicked(path, kind);
  });
  void useImportCenter.getState().refresh();
  void useImportCenter.getState().loadLibrary();
}
