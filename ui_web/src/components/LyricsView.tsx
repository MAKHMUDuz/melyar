import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ChevronLeft, Heart, ListMusic } from "lucide-react";
import { cn } from "@/lib/cn";
import { activeLineIndex, parseLyrics } from "@/lib/lyrics";
import { useLyricsScroll } from "@/lib/useLyricsScroll";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { useLibraryStore } from "@/store";
import { usePlayer } from "@/store";
import { Artwork } from "@/components/Toolbar";
import { t, useLocale } from "@/lib/i18n";
export function LyricsView() {
  useLocale();
  const current = usePlaybackStore((s) => s.current);
  const lyricsRaw = useUIStore((s) => s.lyricsRaw);
  const position = usePlaybackStore((s) => s.position);
  const duration = usePlaybackStore((s) => s.duration);
  const playing = usePlaybackStore((s) => s.playing);
  const toggle = usePlaybackStore((s) => s.toggle);
  const seekSec = usePlaybackStore((s) => s.seekSec);
  const liked = useLibraryStore((s) => s.liked);
  const toggleLike = useLibraryStore((s) => s.toggleLike);
  const setQueueOpen = useUIStore((s) => s.setQueueOpen);
  const queueOpen = useUIStore((s) => s.queueOpen);
  const queue = useLibraryStore((s) => s.queue);
  const { playQueueIndex } = usePlayer();

  const parsed = useMemo(() => parseLyrics(lyricsRaw), [lyricsRaw]);
  const active = parsed.synced ? activeLineIndex(parsed.lines, position) : -1;
  const isLiked = current ? liked.some((t) => t.id === current.id) : false;

  const scrollerRef = useRef<HTMLDivElement>(null);
  const controlsTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [showControls, setShowControls] = useState(true);

  const resetControlsTimer = useCallback(() => {
    if (controlsTimerRef.current) clearTimeout(controlsTimerRef.current);
    setShowControls(true);
    controlsTimerRef.current = setTimeout(() => setShowControls(false), 4000);
  }, []);

  useEffect(() => {
    resetControlsTimer();
    const onPointer = () => resetControlsTimer();
    window.addEventListener("pointermove", onPointer);
    window.addEventListener("scroll", onPointer, { passive: true });
    return () => {
      window.removeEventListener("pointermove", onPointer);
      window.removeEventListener("scroll", onPointer);
      if (controlsTimerRef.current) clearTimeout(controlsTimerRef.current);
    };
  }, [resetControlsTimer]);

  const { ref: lyricsScrollerRef, scrollToActive } = useLyricsScroll(
    parsed.synced,
    active,
  );

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

  const ratio = duration > 0 ? Math.min(1, position / duration) : 0;

  if (!current) {
    return (
      <div className="lyrics-empty flex h-full flex-col items-center justify-center px-8 text-center">
        <ListMusic className="size-24 text-text-tertiary mb-4" />
        <h2 className="text-lg font-bold text-white">{t("lyrics.title")}</h2>
        <p className="text-sm text-text-secondary">{t("toolbar.noTrack")}</p>
      </div>
    );
  }

  return (
    <div className="lyrics-page flex h-full flex-col min-w-0">
      {/* Compact page heading */}
      <header className="lyrics-header flex items-center justify-between px-6 pt-5 pb-3 border-b border-glass-border shrink-0 bg-black/20 backdrop-blur-sm">
        <button
          className="btn-glass text-white/90 hover:bg-white/10 hover:text-white"
          aria-label={t("toolbar.back")}
          onClick={() => {
            const ui = useUIStore.getState();
            ui.goBack();
          }}
        >
          <ChevronLeft className="size-5" />
        </button>
        <div className="min-w-0 flex-1 mx-4 text-center">
          <p className="truncate text-[18px] font-bold text-white">{current.title}</p>
          <p className="truncate text-[14px] text-white/70">{current.artist}</p>
        </div>
        {current ? (
          <button
            className="btn-glass text-white/90 hover:bg-white/10 hover:text-white"
            aria-label={isLiked ? t("track.unlike") : t("track.like")}
            onClick={() => toggleLike(current)}
          >
            <Heart className={cn("size-5", isLiked && "fill-accent text-accent")} />
          </button>
        ) : null}
      </header>

      <div className="flex-1 min-h-0 overflow-hidden flex flex-col">
        {/* Artwork backdrop + lyrics body */}
        <div className="lyrics-body flex-1 min-h-0 relative">
          {current?.thumbnail && (
            <div
              className="absolute inset-0 z-0"
              style={{
                backgroundImage: `url(${current.thumbnail})`,
                backgroundSize: "cover",
                backgroundPosition: "center",
                filter: "blur(46px) brightness(0.32) saturate(1.25)",
                transform: "scale(1.05)",
              }}
            />
          )}
          <div className="absolute inset-0 z-10 bg-gradient-to-t from-black via-black/60 to-transparent" />

          {/* Lyrics scroll area */}
          <div ref={lyricsScrollerRef} className="relative z-20 flex-1 min-h-0 overflow-y-auto scroll-thin px-8 py-12">
            <div className="mx-auto max-w-3xl pt-[30vh] pb-[30vh]">
              {parsed.synced ? (
                parsed.lines.map((line, i) => {
                  const isActive = i === active;
                  const isPast = active >= 0 && i < active;
                  return (
                    <p
                      key={`${line.time}-${i}`}
                      data-active={isActive}
                      className={cn(
                        "lyric-line relative text-center px-4 transition-all duration-300 ease-out",
                        isActive ? "font-bold" : "",
                      )}
                      style={{
                        fontSize: isActive ? "30px" : "22px",
                        transform: isActive ? "scale(1.05)" : "scale(1)",
                        opacity: isActive ? 1 : isPast ? 0.45 : 0.75,
                      }}
                      onClick={() => seekSec(line.time)}
                    >
                      <span className="relative inline-block">
                        <span
                          className={cn(
                            isActive ? "text-white/50" : isPast ? "text-white/45" : "text-white/65",
                          )}
                        >
                          {line.text}
                        </span>
                        {isActive && karaokeWidth > 0 && (
                          <span
                            aria-hidden
                            className="absolute inset-0 text-white pointer-events-none"
                            style={{ WebkitMaskImage: karaokeWipe, maskImage: karaokeWipe }}
                          >
                            {line.text}
                          </span>
                        )}
                      </span>
                    </p>
                  );
                })
              ) : parsed.plain.length > 0 ? (
                parsed.plain.map((line, i) => (
                  <p key={i} className="py-2 text-[20px] font-medium leading-relaxed text-white/85 text-center">
                    {line}
                  </p>
                ))
              ) : (
                <div className="grid place-items-center gap-3 text-center py-20">
                  <Artwork src={current?.thumbnail} label={current?.title} className="size-24" />
                  <p className="text-lg font-semibold text-white/80">{t("lyrics.notFound")}</p>
                  <p className="text-sm text-white/50">{t("lyrics.notFoundHint")}</p>
                </div>
              )}
            </div>
          </div>
        </div>

        {/* Bottom controls — only lyrics toggle and queue toggle, consistent with bottom player */}
        <div className={cn(
          "relative z-20 flex items-center justify-center gap-6 px-6 py-6 shrink-0 border-t border-glass-border bg-black/20 backdrop-blur-sm",
          showControls ? "opacity-100 transition-opacity duration-300" : "opacity-0 pointer-events-none transition-opacity duration-300",
        )}>
          <button
            className="btn-glass text-white/80 hover:bg-white/10 hover:text-white"
            aria-label={queueOpen ? t("lyrics.queue.close") : t("lyrics.queue.open")}
            onClick={() => setQueueOpen(!queueOpen)}
          >
            <ListMusic className={cn("size-5", queueOpen && "text-accent")} />
          </button>
        </div>

        {/* Progress bar at bottom */}
        <div className="relative z-20 h-3 bg-white/10 rounded-full overflow-hidden mb-6 px-6 shrink-0">
          <div className="h-full bg-gradient-to-r from-accent to-[#d61d33] shadow-[0_0_12px_rgba(250,35,59,0.6)]"
            style={{ width: `${ratio * 100}%` }}
          />
        </div>
      </div>
    </div>
  );
}
