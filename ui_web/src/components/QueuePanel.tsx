import { useCallback, useEffect, useRef } from "react";
import { X, MoreHorizontal, ChevronUp, ListMusic } from "lucide-react";
import type { TrackMeta } from "@/lib/types";
import { cn } from "@/lib/cn";
import { formatTime } from "@/lib/lyrics";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { useLibraryStore } from "@/store";
import { usePlayer } from "@/store";
import { Artwork } from "@/components/Toolbar";
import { t, tCount, useLocale } from "@/lib/i18n";

export function QueuePanel() {
  useLocale();
  const open = useUIStore((s) => s.queueOpen);
  const setOpen = useUIStore((s) => s.setQueueOpen);
  const queue = useLibraryStore((s) => s.queue);
  const current = usePlaybackStore((s) => s.current);
  const { playQueueIndex, clearQueue } = usePlayer();
  const openMenu = useUIStore((s) => s.openMenu);

  const panelRef = useRef<HTMLDivElement>(null);
  const closeRef = useRef<HTMLButtonElement>(null);
  const isNarrow = typeof window !== "undefined" && window.innerWidth < 1200;

  const handleEscape = useCallback((e: KeyboardEvent) => {
    if (e.key === "Escape") setOpen(false);
  }, [setOpen]);

  useEffect(() => {
    if (!open || !isNarrow) return;
    window.addEventListener("keydown", handleEscape);
    return () => window.removeEventListener("keydown", handleEscape);
  }, [open, isNarrow, handleEscape]);

  useEffect(() => {
    if (open && panelRef.current) {
      const focusable = panelRef.current.querySelector<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      focusable?.focus();
    }
  }, [open]);

  if (!open) return null;

  const handleRowClick = (trackId: string, e: React.MouseEvent) => {
    const btn = e.currentTarget as HTMLButtonElement;
    const menuBtn = btn.querySelector('[data-menu-btn="true"]');
    if (menuBtn && menuBtn.contains(e.target as Node)) return;
    const idx = queue.findIndex((t) => t.id === trackId);
    if (idx >= 0) playQueueIndex(idx);
  };

  const handleMenuClick = (track: TrackMeta, rect: DOMRect, e: React.MouseEvent) => {
    e.stopPropagation();
    openMenu({ track, x: rect.left, y: rect.bottom + 6 });
  };

  return (
    <aside
      ref={panelRef}
      className={cn(
        "queue-panel glass-intense backdrop-blur-lg border-l border-glass-border shadow-2xl",
        isNarrow && "floating",
        open && "open",
      )}
      aria-hidden={!open}
      aria-label={t("queue.title")}
      role="region"
    >
      <div className="flex items-center justify-between px-5 pt-5 pb-3 border-b border-glass-border shrink-0">
        <div className="flex items-center gap-3">
          <div className="size-8 rounded-lg bg-panel flex items-center justify-center">
            <ListMusic className="size-5 text-accent-text" />
          </div>
          <div>
            <h2 className="text-[15px] font-bold tracking-tight text-text">{t("queue.title")}</h2>
            <p className="text-[11px] text-text-tertiary">{tCount("queue.count", queue.length)}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          {queue.length > 0 ? (
            <button
              className="btn-glass chip text-xs"
              onClick={(e) => {
                e.stopPropagation();
                clearQueue();
                // The queue is now empty, so this button is about to unmount
                // and would drop keyboard focus to the page body. Hand it to
                // the close button, which is the next thing a keyboard user
                // needs anyway. The undo affordance lives in the toast.
                closeRef.current?.focus();
              }}
              aria-label={t("queue.clear")}
            >
              {t("queue.clearButton")}
            </button>
          ) : null}
          <button
            ref={closeRef}
            className="btn-glass icon-btn text-text hover:bg-panel hover:text-text"
            aria-label={t("queue.close")}
            onClick={(e) => { e.stopPropagation(); setOpen(false); }}
          >
            <X className="size-4" />
          </button>
        </div>
      </div>

      <div className="scroll-thin flex-1 min-h-0 overflow-y-auto pr-2 pb-4">
        {queue.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <ListMusic className="size-12 text-text/10 mb-3" />
            <p className="text-sm text-text-tertiary">{t("queue.empty")}</p>
          </div>
        ) : (
          queue.map((track, i) => {
            const active = current?.id === track.id;
            return (
              <div key={`${track.id}-${i}`} className="relative group">
                <button
                  className={cn(
                    "track-row flex items-center gap-3 rounded-xl px-4 py-3 text-left transition-all duration-200",
                    active && "bg-accent/10 border border-accent/30",
                    !active && "hover:bg-white/5",
                  )}
                  onClick={(e) => handleRowClick(track.id, e)}
                  title={`${track.title} — ${track.artist}`}
                >
                  <div className="relative flex-shrink-0">
                    <Artwork src={track.thumbnail} label={track.title} className="size-12 rounded-lg" />
                    {active && (
                      <div className="absolute inset-0 rounded-lg flex items-center justify-center text-text">
                        <ListMusic className="size-5" />
                      </div>
                    )}
                    <span className="absolute -right-1 -top-1 z-10 flex size-5 items-center justify-center rounded-full bg-accent text-black font-bold text-[10px]">
                      {i + 1}
                    </span>
                  </div>
                  <div className="min-w-0 flex-1">
                    <span className={cn("block truncate text-[14px] font-medium", active && "text-accent-text")}>
                      {track.title}
                    </span>
                    <span className="block truncate text-[12px] text-text-secondary">{track.artist}</span>
                  </div>
                  <span className="text-[11px] tabular-nums text-text-tertiary whitespace-nowrap">
                    {formatTime(track.duration)}
                  </span>
                </button>
                <button
                  data-menu-btn="true"
                  className="absolute right-2 top-1/2 -translate-y-1/2 opacity-0 group-hover:opacity-100 transition-opacity duration-200 icon-btn text-text-tertiary hover:text-text"
                  aria-label={t("track.options")}
                  onClick={(e) => handleMenuClick(track, (e.currentTarget as HTMLElement).getBoundingClientRect(), e)}
                >
                  <MoreHorizontal className="size-4" />
                </button>
              </div>
            );
          })
        )}
      </div>
    </aside>
  );
}