import { create } from "zustand";
import {
  downloadTrack as bridgeDownload,
  removeDownload as bridgeRemove,
  downloadedTracks as bridgeDownloaded,
} from "@/lib/client";
import { useUIStore } from "@/store";
import type { TrackMeta } from "@/lib/types";

type DownloadState = {
  /** Tracks that have a local copy on disk. */
  downloaded: Record<string, boolean>;
  /** Tracks whose download is in flight. */
  downloading: Record<string, boolean>;
  /** The last error message, if a download failed. */
  error: string | null;
  /** Called by the bridge when a download finishes. */
  onDownloadFinished: (trackId: string, success: boolean, error: string) => void;
  /** Read the list of downloaded tracks from the bridge. */
  syncFromBridge: () => void;
  /** Ask the core to fetch a track. */
  download: (track: TrackMeta) => void;
  /** Drop the local copy and the file. */
  remove: (track: TrackMeta) => void;
  /** Whether a track has a local copy. */
  isDownloaded: (trackId: string) => boolean;
};

export const useDownloadStore = create<DownloadState>((set, get) => ({
  downloaded: {},
  downloading: {},
  error: null,

  onDownloadFinished: (trackId, success, error) => {
    set((state) => {
      const downloading = { ...state.downloading };
      delete downloading[trackId];
      const downloaded = { ...state.downloaded };
      if (success) {
        downloaded[trackId] = true;
      } else {
        delete downloaded[trackId];
      }
      return { downloading, downloaded, error: success ? null : error || null };
    });

    if (success) {
      useUIStore.getState().toastNote("download.finished");
    } else {
      useUIStore.getState().toastNote("download.failed");
    }
  },

  syncFromBridge: () => {
    const list = bridgeDownloaded();
    const downloaded: Record<string, boolean> = {};
    list.forEach((t) => {
      downloaded[String(t.id)] = true;
    });
    set({ downloaded });
  },

  download: (track) => {
    const id = track.id;
    if (!id) return;
    if (get().downloading[id]) return;
    if (get().downloaded[id]) return;

    set((state) => ({
      downloading: { ...state.downloading, [id]: true },
      error: null,
    }));

    bridgeDownload(id);
  },

  remove: (track) => {
    const id = track.id;
    if (!id) return;
    bridgeRemove(id);
    set((state) => {
      const downloaded = { ...state.downloaded };
      delete downloaded[id];
      return { downloaded };
    });
  },

  isDownloaded: (trackId) => Boolean(get().downloaded[trackId]),
}));
