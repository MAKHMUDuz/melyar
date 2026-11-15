import { useMemo, useRef, useState } from "react";
import { ChevronLeft, Heart, Mic2, MoreHorizontal, Settings, Share2 } from "lucide-react";
import { cn } from "@/lib/cn";
import { formatTime } from "@/lib/lyrics";
import { activeLineIndex, parseLyrics } from "@/lib/lyrics";
import { useLyricsScroll } from "@/lib/useLyricsScroll";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { useLibraryStore } from "@/store";
import { usePlayer } from "@/store";
import { Artwork } from "@/components/Toolbar";
import type { TrackMeta } from "@/lib/types";
import { t, useLocale } from "@/lib/i18n";
import { useSettings } from "@/lib/settings";

function LyricLine({ text, isActive, isPast, onClick }: { text: string; isActive: boolean; isPast: boolean; onClick: () => void }) {
  return (
    <p
      data-active={isActive}
      className={cn(
        "lyric-line text-center px-3 py-1 cursor-pointer select-text transition-all duration-200 ease-out",
        isActive ? "text-white font-semibold text-[22px] scale-105 opacity-100" : "text-white/60 text-[18px] opacity-100",
        isPast ? "text-white/45" : "",
      )}
      onClick={onClick}
      style={{ transform: isActive ? "scale(1.03)" : "scale(1)" }}
    >
      {text}
    </p>
  );
}

export function NowPlayingView() {
  useLocale();
  const setQueueOpen = useUIStore((s) => s.setQueueOpen);
  const queueOpen = useUIStore((s) => s.queueOpen);
  const current = usePlaybackStore((s) => s.current);
  const playing = usePlaybackStore((s) => s.playing);
  const position = usePlaybackStore((s) => s.position);
  const duration = usePlaybackStore((s) => s.duration);
  const liked = useLibraryStore((s) => s.liked);
  const setSettingsOpen = useSettings((s) => s.setOpen);
  const toggle = usePlaybackStore((s) => s.toggle);
  const toggleLike = useLibraryStore((s) => s.toggleLike);
  const seekSec = usePlaybackStore((s) => s.seekSec);
  const openMenu = useUIStore((s) => s.openMenu);
  const setShareTrack = useUIStore((s) => s.setShareTrack);

  const lyricsRaw = useUIStore((s) => s.lyricsRaw);

  const moreBtnRef = useRef<HTMLButtonElement>(null);
  const scrollerRef = useRef<HTMLDivElement>(null);
  const [returning, setReturning] = useState(false);

  const parsed = useMemo(() => parseLyrics(lyricsRaw), [lyricsRaw]);
  const active = parsed.synced ? activeLineIndex(parsed.lines, position) : -1;
  const isLiked = current ? liked.some((t) => t.id === current.id) : false;

  const lyricsScrollerRef = useLyricsScroll(parsed.synced, active);

  // Karaoke fill progress for the active line
  let karaokeWidth = 0;
  if (parsed.synced && active >= 0) {
    const line = parsed.lines[active];
    const nextLine = parsed.lines[active + 1];
    const lineEnd = nextLine ? nextLine.time : line.time + 5;
    const lineDuration = lineEnd - line.time;
    karaokeWidth = lineDuration > 0 ? Math.min(1, Math.max(0, (position - line.time) / lineDuration)) : 0;
  }
  const wipeEdge = 6;
  const wipeStop = karaokeWidth * 100;
  const karaokeWipe = `linear-gradient(to right, black ${Math.max(0, wipeStop - wipeEdge)}%, transparent ${Math.min(100, wipeStop + wipeEdge)}%)`;

  const handleMoreClick = () => {
    if (!current) return;
    const r = moreBtnRef.current?.getBoundingClientRect();
    if (!r) return;
    openMenu({ track: current, x: r.left, y: r.bottom + 6 });
  };

  const handleShareClick = () => {
    if (!current) return;
    // The share sheet carries the link and the download, so the button
    // opens it rather than writing to the clipboard behind the user's back.
    setShareTrack(current);
  };

  const handleReturnToActive = () => {
    setReturning(true);
    lyricsScrollerRef.scrollToActive?.();
    setTimeout(() => setReturning(false), 300);
  };

  if (!current) {
    return (
      <div className="now-playing-empty flex h-full flex-col items-center justify-center px-8 text-center">
        <Mic2 className="size-24 text-text-tertiary mb-4" />
        <h2 className="text-lg font-bold text-white">{t("expanded.nowPlaying")}</h2>
        <p className="text-sm text-text-secondary">{t("toolbar.noTrack")}</p>
      </div>
    );
  }

  return (
    <div className="now-playing-page flex h-full flex-col min-w-0">
      {/* Compact page heading — it lives inside the content width instead of
          being a full-bleed dark bar across the top of the window. */}
      <div className="now-playing-heading flex items-center gap-2">
        <button
          className="btn-glass icon-btn text-white/90 hover:bg-white/10 hover:text-white"
          aria-label={t("toolbar.back")}
          onClick={() => {
            const ui = useUIStore.getState();
            ui.goBack();
          }}
        >
          <ChevronLeft className="size-[18px]" />
        </button>
        <h1 className="text-[13px] font-semibold tracking-tight text-text-secondary">
          {t("expanded.nowPlaying")}
        </h1>
        <div className="ml-auto flex items-center gap-1">
          <button
            className="btn-glass icon-btn text-white/80 hover:bg-white/10 hover:text-white"
            aria-label={t("settings.title")}
            onClick={() => setSettingsOpen(true)}
          >
            <Settings className="size-4" />
          </button>
        </div>
      </div>

      <div className="flex-1 min-h-0 overflow-hidden">
        {/* Artwork + Metadata | Lyrics grid */}
        <div className="now-playing-grid flex-1 min-h-0 overflow-hidden">
          {/* Left: Artwork + Metadata */}
          <div className="now-playing-artwork-section flex flex-col items-center min-w-0">
            <div className="now-playing-artwork-wrap relative flex-shrink-0 w-full max-w-[380px] aspect-square">
              {current.thumbnail && (
                <div className="now-playing-artwork relative overflow-hidden rounded-2xl shadow-2xl shadow-accent-glow/20">
                  <img src={current.thumbnail} alt="" className="h-full w-full object-cover" />
                  <div className="absolute inset-0 bg-black/20" />
                </div>
              )}
              {!current.thumbnail && (
                <div className="now-playing-artwork cover-fallback h-full w-full flex items-center justify-center rounded-2xl bg-black/30">
                  <Mic2 className="size-32 opacity-60" />
                </div>
              )}
            </div>

            <div className="now-playing-meta mt-5 text-center">
              {current.album && (
                <p className="text-[13px] text-text-tertiary truncate mb-1.5">
                  {current.album}
                </p>
              )}
              <h2 className="now-playing-title text-[28px] font-bold tracking-tight text-white mb-1 truncate">
                {current.title}
              </h2>
              <p className="now-playing-artist text-[16px] font-medium text-white/80 truncate mb-4">
                {current.artist}
              </p>

              <div className="flex items-center justify-center gap-1.5">
                <button
                  className="btn-glass icon-btn text-white/90 hover:bg-white/10 hover:text-white transition-all"
                  aria-label={isLiked ? t("menu.unlike") : t("menu.like")}
                  onClick={() => toggleLike(current)}
                >
                  <Heart className={cn("size-[18px]", isLiked && "fill-accent text-accent")} />
                </button>
                <button
                  className="btn-glass icon-btn text-white/90 hover:bg-white/10 hover:text-white transition-all"
                  aria-label={t("expanded.share")}
                  onClick={handleShareClick}
                >
                  <Share2 className="size-[18px]" />
                </button>
                <button
                  ref={moreBtnRef}
                  className="btn-glass icon-btn text-white/90 hover:bg-white/10 hover:text-white transition-all"
                  aria-label={t("track.options")}
                  onClick={handleMoreClick}
                >
                  <MoreHorizontal className="size-[18px]" />
                </button>
              </div>
            </div>
          </div>

          {/* Right: Lyrics */}
          <div className="now-playing-lyrics-section flex-1 min-w-0 flex flex-col">
            {parsed.synced && parsed.lines.length > 0 ? (
              <div className="flex-1 min-h-0 overflow-y-auto scroll-thin px-4">
                <div className="flex flex-col items-center gap-1.5 pt-2">
                  {parsed.lines.map((line, i) => {
                    const isActive = i === active;
                    const isPast = active >= 0 && i < active;
                    return (
                      <LyricLine
                        key={`${line.time}-${i}`}
                        text={line.text}
                        isActive={isActive}
                        isPast={isPast}
                        onClick={() => seekSec(line.time)}
                      />
                    );
                  })}
                  {active >= 0 && (
                    <div className={`mt-4 flex items-center gap-2 transition-opacity duration-300 ${returning ? "opacity-50" : "opacity-100"}`}>
                      <button
                        className="btn-glass text-xs text-white/70 hover:text-white hover:bg-white/10"
                        onClick={handleReturnToActive}
                      >
                        {t("lyrics.returnToCurrent")}
                      </button>
                    </div>
                  )}
                </div>
              </div>
            ) : parsed.plain.length > 0 ? (
              <div className="flex-1 min-h-0 flex flex-col items-center justify-center px-8 py-12">
                {parsed.plain.map((line, i) => (
                  <p key={i} className="py-2 text-[17px] font-medium leading-relaxed text-white/85 text-center">
                    {line}
                  </p>
                ))}
              </div>
            ) : (
              <div className="flex-1 min-h-0 flex flex-col items-center justify-center px-8 py-12">
                <Artwork src={current?.thumbnail} label={current?.title} className="size-24" />
                <p className="mt-3 text-lg font-semibold text-white/80">{t("lyrics.notFound")}</p>
                <p className="text-sm text-white/50">{t("lyrics.notFoundHint")}</p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
