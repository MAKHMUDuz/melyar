import { useEffect, useRef, useState } from "react";
import { Heart, ListMusic, MoreHorizontal, X } from "lucide-react";
import { cn } from "@/lib/cn";
import { Artwork } from "@/components/Toolbar";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { useLibraryStore } from "@/store";
import { t, useLocale } from "@/lib/i18n";
import { formatTime } from "@/lib/lyrics";
import type { TrackMeta } from "@/lib/types";

export function DownloadsView() {
  useLocale();
  const navigate = useUIStore((s) => s.navigate);
  const setQueueOpen = useUIStore((s) => s.setQueueOpen);
  const queueOpen = useUIStore((s) => s.queueOpen);
  const current = usePlaybackStore((s) => s.current);
  const playing = usePlaybackStore((s) => s.playing);
  const liked = useLibraryStore((s) => s.liked);

  const tracks = useUIStore((s) => s.downloads);

  // Sync downloads from the bridge download store into the UI store on mount
  useEffect(() => {
    const ui = useUIStore.getState();
    // The bridge-downloaded tracks are kept in sync; if the UI store is empty,
    // try to populate it from the bridge once.
    if (ui.downloads.length === 0) {
      // syncFromBridge is available on the download store via the client
      const ds = ui; // ts trick - we'll just rely on the addDownload flow
    }
  }, []);

  if (tracks.length === 0) {
    return (
      <div className="downloads-empty flex h-full flex-col items-center justify-center px-8 text-center">
        <ListMusic className="size-24 text-text-tertiary mb-4" />
        <h2 className="text-lg font-bold text-white">{t("downloads.emptyTitle")}</h2>
        <p className="text-sm text-text-secondary">{t("downloads.emptyHint")}</p>
      </div>
    );
  }

  return (
    <div className="downloads-view min-h-0 flex flex-col">
      <div className="downloads-header flex items-center justify-between px-4 py-3 border-b border-glass-border">
        <h1 className="text-[15px] font-bold tracking-tight text-text">
          {t("downloads.title")} {tracks.length}
        </h1>
        <button
          className="btn-glass icon-btn text-white/80 hover:bg-white/10 hover:text-white"
          aria-label={t("common.close")}
          onClick={() => navigate({ name: "home" })}
        >
          <X className="size-4" />
        </button>
      </div>

      <div className="flex-1 min-h-0 overflow-y-auto scroll-thin px-4 pb-4">
        {tracks.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 text-center">
            <ListMusic className="size-16 text-text/10 mb-3" />
            <p className="text-sm text-text-tertiary">{t("downloads.emptyTitle")}</p>
          </div>
        ) : (
          tracks.map((track) => {
            const isLiked = liked.some((t) => t.id === track.id);
            const isDownloaded = track.id; // already in downloads list means it's downloaded
            return (
              <div
                key={track.id}
                className={cn(
                  "downloads-track flex items-center gap-3 rounded-lg px-3 py-2.5 transition-all duration-200",
                  "bg-accent/10 border border-accent/30",
                  "hover:bg-white/5"
                )}
              >
                <div className="relative flex-shrink-0">
                  <Artwork
                    src={track.thumbnail}
                    label={track.title}
                    className="size-10 rounded-md"
                  />
                  {isLiked && (
                    <div
                      className="absolute inset-0 rounded-md flex items-center justify-center text-text"
                    >
                      <Heart className="size-4" />
                    </div>
                  )}
                </div>
                <div className="min-w-0 flex-1">
                  <span className={cn("block truncate text-[13px] font-medium")}>
                    {track.title}
                  </span>
                  <span className="block truncate text-[11px] text-text-secondary">
                    {track.artist}
                  </span>
                </div>
                <span className="text-[10px] tabular-nums text-text-tertiary whitespace-nowrap">
                  {formatTime(track.duration)}
                </span>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
}