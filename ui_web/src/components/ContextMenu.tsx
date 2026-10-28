import { Heart, ListPlus, Mic2, Share2, X, Trash2, Play } from "lucide-react";
import { cn } from "@/lib/cn";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { useLibraryStore } from "@/store";
import { Artwork } from "@/components/Toolbar";
import { t, useLocale } from "@/lib/i18n";

export function ContextMenu() {
  useLocale();
  const menu = useUIStore((s) => s.menu);
  const closeMenu = useUIStore((s) => s.closeMenu);
  const liked = useLibraryStore((s) => s.liked);
  const toggleLike = useLibraryStore((s) => s.toggleLike);
  const playNextInQueue = useLibraryStore((s) => s.playNextInQueue);
  const addToQueue = useLibraryStore((s) => s.addToQueue);
  const removeFromQueue = useLibraryStore((s) => s.removeFromQueue);
  const queue = useLibraryStore((s) => s.queue);
  const navigate = useUIStore((s) => s.navigate);
  const setLyricsOpen = useUIStore((s) => s.setLyricsOpen);
  const toastNote = useUIStore((s) => s.toastNote);

  if (!menu) return null;
  const { track, x, y } = menu;
  const isLiked = liked.some((t) => t.id === track.id);
  const left = Math.max(10, Math.min(x, window.innerWidth - 320));
  const top = Math.max(10, Math.min(y, window.innerHeight - 420));

  return (
    <div
      className="fixed z-[90] w-full max-w-md"
      style={{ left, top }}
      onPointerDown={(e) => e.stopPropagation()}
    >
      {/* Backdrop barrier */}
      <div
        className="fixed inset-0 bg-black/72 backdrop-blur-sm z-[89]"
        onClick={closeMenu}
      />

      <div className="relative z-[90] mt-2 mb-4 mx-4 rounded-[24px] bg-panel border border-glass-border shadow-2xl shadow-glass-shadow-xl overflow-hidden flex flex-col">
        {/* Drag handle + close */}
        <div className="flex items-center justify-between px-5 pt-4 pb-3 border-b border-glass-border">
          <div className="w-8 h-1 bg-panel-strong rounded-full mx-4" />
          <button
            className="icon-btn text-text-secondary hover:bg-panel hover:text-text"
            onClick={closeMenu}
            aria-label={t("common.close")}
          >
            <X className="size-5" />
          </button>
        </div>

        {/* Song card */}
        <div className="px-5 py-4">
          <div className="flex items-center gap-3">
            <div className="relative">
              <Artwork src={track.thumbnail} label={track.title} className="size-12 rounded-lg" />
            </div>
            <div className="min-w-0 flex-1">
              <p className="truncate text-[15px] font-medium text-text">{track.title}</p>
              <p className="truncate text-[13px] text-text-secondary">{track.artist}</p>
            </div>
          </div>
        </div>

        {/* Action row: play next, add to queue */}
        <div className="flex gap-2 px-5 py-3 border-b border-glass-border/50">
          <button
            className="flex-1 flex items-center justify-center gap-2 bg-white/5 hover:bg-panel rounded-xl px-4 py-2.5 text-sm font-medium text-text transition-colors"
            onClick={(e) => { e.stopPropagation(); playNextInQueue(track); closeMenu(); }}
          >
            <Play className="size-4" />
            {t("menu.playNext")}
          </button>
          <button
            className="flex-1 flex items-center justify-center gap-2 bg-white/5 hover:bg-panel rounded-xl px-4 py-2.5 text-sm font-medium text-text transition-colors"
            onClick={(e) => { e.stopPropagation(); addToQueue(track); closeMenu(); }}
          >
            <ListPlus className="size-4" />
            {t("menu.addToQueue")}
          </button>
        </div>

        {/* Compact tiles */}
        <div className="py-2">
          <button
            className="compact-tile flex items-center justify-between gap-3 px-5 py-2.5 rounded-lg hover:bg-white/5 transition-colors"
            onClick={(e) => { e.stopPropagation(); toggleLike(track); closeMenu(); }}
          >
            <div className="flex items-center gap-3">
              <Heart className={cn("size-4", isLiked && "fill-accent text-accent-text")} />
              <span className="text-sm font-medium text-text">
                {isLiked ? t("menu.unlike") : t("menu.like")}
              </span>
            </div>
            <div className="size-6 rounded-full bg-white/5 flex items-center justify-center text-text-tertiary">
              <ChevronIcon className="size-3" />
            </div>
          </button>
          <button
            className="compact-tile flex items-center justify-between gap-3 px-5 py-2.5 rounded-lg hover:bg-white/5 transition-colors border-t border-glass-border/50"
            onClick={(e) => { e.stopPropagation(); setLyricsOpen(true); closeMenu(); }}
          >
            <div className="flex items-center gap-3">
              <Mic2 className="size-4 text-text-secondary" />
              <span className="text-sm font-medium text-text">{t("menu.goToLyrics")}</span>
            </div>
            <div className="size-6 rounded-full bg-white/5 flex items-center justify-center text-text-tertiary">
              <ChevronIcon className="size-3" />
            </div>
          </button>
          <button
            className="compact-tile flex items-center justify-between gap-3 px-5 py-2.5 rounded-lg hover:bg-white/5 transition-colors border-t border-glass-border/50"
            onClick={(e) => { e.stopPropagation(); navigator.clipboard.writeText(`${track.title} - ${track.artist}`).then(() => toastNote(t("toast.linkCopied"))); closeMenu(); }}
          >
            <div className="flex items-center gap-3">
              <Share2 className="size-4 text-text-secondary" />
              <span className="text-sm font-medium text-text">{t("menu.copyLink")}</span>
            </div>
            <div className="size-6 rounded-full bg-white/5 flex items-center justify-center text-text-tertiary">
              <ChevronIcon className="size-3" />
            </div>
          </button>
          <button
            className="compact-tile flex items-center justify-between gap-3 px-5 py-2.5 rounded-lg hover:bg-white/5 transition-colors border-t border-glass-border/50"
            onClick={(e) => { e.stopPropagation(); navigate({ name: "artist", key: track.artist.toLowerCase() }); closeMenu(); }}
          >
            <div className="flex items-center gap-3">
              <svg viewBox="0 0 24 24" className="size-4 text-text-secondary" fill="none" stroke="currentColor" strokeWidth="2">
                <circle cx="12" cy="8" r="4" />
                <path d="M4 20c1.5-4 4.5-6 8-6s6.5 2 8 6" />
              </svg>
              <span className="text-sm font-medium text-text">{t("menu.goToArtist")}</span>
            </div>
            <div className="size-6 rounded-full bg-white/5 flex items-center justify-center text-text-tertiary">
              <ChevronIcon className="size-3" />
            </div>
          </button>
          {/* Only offered for a track that is actually queued. The bridge used to
              clear this row off the list without touching what plays, so the
              song you removed came back at the end of the album - the row now
              goes through the core, which is the list that plays. */}
          {queue.some((q) => q.id === track.id) ? (
            <button
              className="compact-tile flex items-center justify-between gap-3 px-5 py-2.5 rounded-lg hover:bg-white/5 transition-colors border-t border-glass-border/50 text-red-400"
              onClick={(e) => { e.stopPropagation(); removeFromQueue(track); }}
            >
              <div className="flex items-center gap-3">
                <Trash2 className="size-4" />
                <span className="text-sm font-medium text-red-400">{t("menu.removeFromQueue")}</span>
              </div>
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}

function ChevronIcon({ className }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" className={cn("size-4", className)} fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M9 5l7 7-7 7" />
    </svg>
  );
}