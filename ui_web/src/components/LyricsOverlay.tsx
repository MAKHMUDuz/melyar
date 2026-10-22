import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { ChevronDown, Heart, ListMusic, SkipBack, SkipForward } from "lucide-react";
import { cn } from "@/lib/cn";
import { activeLineIndex, parseLyrics } from "@/lib/lyrics";
import { useLyricsScroll } from "@/lib/useLyricsScroll";
import { usePlayer } from "@/store";
import { Artwork } from "@/components/Toolbar";
import { PlayPauseMorph } from "@/components/GlassComponents";
import { t, useLocale } from "@/lib/i18n";
import { getMotionTokens } from "@/lib/motion";

export function LyricsOverlay() {
  useLocale();
  const open = usePlayer((s) => s.lyricsOpen);
  const setOpen = usePlayer((s) => s.setLyricsOpen);
  const current = usePlayer((s) => s.current);
  const lyricsRaw = usePlayer((s) => s.lyricsRaw);
  const position = usePlayer((s) => s.position);
  const duration = usePlayer((s) => s.duration);
  const playing = usePlayer((s) => s.playing);
  const toggle = usePlayer((s) => s.toggle);
  const prev = usePlayer((s) => s.prev);
  const next = usePlayer((s) => s.next);
  const seekSec = usePlayer((s) => s.seekSec);
  const liked = usePlayer((s) => s.liked);
  const toggleLike = usePlayer((s) => s.toggleLike);
  const setQueueOpen = usePlayer((s) => s.setQueueOpen);
  const queueOpen = usePlayer((s) => s.queueOpen);
  const queue = usePlayer((s) => s.queue);
  const playQueueIndex = usePlayer((s) => s.playQueueIndex);

  const parsed = useMemo(() => parseLyrics(lyricsRaw), [lyricsRaw]);
  const active = parsed.synced ? activeLineIndex(parsed.lines, position) : -1;
  const isLiked = current ? liked.some((t) => t.id === current.id) : false;

  const scrollerRef = useRef<HTMLDivElement>(null);
  const [showControls, setShowControls] = useState(true);
  // The timer lives in a ref, not in state. In state it fed itself: every reset
  // produced a new callback identity, the effect listed that identity as a
  // dependency, so it re-ran on each timer change - re-adding the window
  // listeners and re-arming the hide timer forever. The controls therefore
  // never hid, and the overlay redrew without pause for as long as it was open.
  const controlsTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Auto-hide controls after 4s of inactivity
  const resetControlsTimer = useCallback(() => {
    if (controlsTimerRef.current) clearTimeout(controlsTimerRef.current);
    setShowControls(true);
    controlsTimerRef.current = setTimeout(() => setShowControls(false), 4000);
  }, []);

  useEffect(() => {
    if (!open) return;
    resetControlsTimer();
    const onPointer = () => resetControlsTimer();
    window.addEventListener("pointermove", onPointer);
    window.addEventListener("scroll", onPointer, { passive: true });
    return () => {
      window.removeEventListener("pointermove", onPointer);
      window.removeEventListener("scroll", onPointer);
      if (controlsTimerRef.current) clearTimeout(controlsTimerRef.current);
    };
  }, [open, resetControlsTimer]);

  const { ref: lyricsScrollerRef, scrollToActive } = useLyricsScroll(
    open && parsed.synced,
    active,
  );

  // Close on Escape
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    if (open) {
      window.addEventListener("keydown", onKey);
      document.body.style.overflow = "hidden";
    }
    return () => {
      window.removeEventListener("keydown", onKey);
      document.body.style.overflow = "";
    };
  }, [open, setOpen]);

  if (!open) return null;
  const motion = getMotionTokens();
  const ratio = duration > 0 ? Math.min(1, position / duration) : 0;

  // Karaoke fill progress for the active line
  let karaokeWidth = 0;
  if (parsed.synced && active >= 0) {
    const line = parsed.lines[active];
    const nextLine = parsed.lines[active + 1];
    const lineEnd = nextLine ? nextLine.time : line.time + 5;
    const lineDuration = lineEnd - line.time;
    karaokeWidth = lineDuration > 0 ? Math.min(1, Math.max(0, (position - line.time) / lineDuration)) : 0;
  }
  // A soft leading edge instead of a hard cut, so the fill reads as a wipe
  // travelling across the line rather than a block snapping into place.
  const wipeEdge = 8;
  const wipeStop = karaokeWidth * 100;
  const karaokeWipe = `linear-gradient(to right, black ${Math.max(0, wipeStop - wipeEdge)}%, transparent ${Math.min(100, wipeStop + wipeEdge)}%)`;

  return (
    <div
      className={cn(
        "lyrics-overlay fixed inset-0 z-[70] flex flex-col bg-black",
        !showControls ? "pointer-events-none" : "",
      )}
      role="dialog"
      aria-modal="true"
      aria-label={t("lyrics.aria")}
      onClick={() => resetControlsTimer()}
    >
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

      {/* Top bar */}
      <div className="relative z-20 flex items-center justify-between px-5 pt-4 pb-2">
        <button
          className="btn-glass text-white/90 hover:bg-white/10 hover:text-white"
          aria-label={t("lyrics.close")}
          onClick={() => setOpen(false)}
        >
          <ChevronDown className="size-6" />
        </button>
        <div className="min-w-0 flex-1 mx-4 text-center">
          <p className="truncate text-[18px] font-bold text-white">{current?.title ?? t("lyrics.title")}</p>
          <p className="truncate text-[14px] text-white/70">{current?.artist ?? ""}</p>
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
      </div>

      {/* Lyrics body */}
      <div ref={lyricsScrollerRef} className="relative z-20 flex-1 min-h-0 overflow-y-auto px-8 py-12">
        <div className="mx-auto max-w-3xl pt-[40vh] pb-[40vh]">
          {parsed.synced ? (
            parsed.lines.map((line, i) => {
              const isActive = i === active;
              const isPast = active >= 0 && i < active;
              return (
                <p
                  key={`${line.time}-${i}`}
                  data-active={isActive}
                  className={cn(
                    "lyric-line relative text-center px-4",
                    isActive ? "font-bold" : "",
                  )}
                  style={{
                    fontSize: isActive ? "34px" : "24px",
                    transform: isActive ? "scale(1.05)" : "scale(1)",
                    opacity: isActive ? 1 : isPast ? 0.45 : 0.75,
                    transition: motion.transition.overlay,
                  }}
                  onClick={() => seekSec(line.time)}
                >
                  {/* The glyphs sit in one box sized to themselves, and the
                      karaoke copy is painted over exactly that box. Without
                      this wrapper the overlay's `inset-0` resolved against
                      the scroller instead of the line, so the active line's
                      text was swept across the whole pane as a white bar —
                      and because the base copy was conditional on the line
                      not being active, the active line itself rendered
                      blank. The line is now always readable, and the fill
                      brightens what has already been sung. */}
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

      {/* Bottom controls */}
      <div
        className={cn("relative z-20 flex items-center justify-center gap-6 px-5 py-6", showControls ? "opacity-100" : "opacity-0 pointer-events-none")}
        style={{ transition: motion.transition.overlay }}
      >
        <button
          className="btn-glass text-white/90 hover:bg-white/10 hover:text-white"
          aria-label={t("lyrics.prev")}
          onClick={prev}
        >
          <SkipBack className="size-7 fill-current" />
        </button>
        <PlayPauseMorph
          size="expanded"
          playing={playing}
          onToggle={toggle}
          className="!size-20"
        />
        <button
          className="btn-glass text-white/90 hover:bg-white/10 hover:text-white"
          aria-label={t("toolbar.next")}
          onClick={next}
        >
          <SkipForward className="size-7 fill-current" />
        </button>
        <button
          className="btn-glass text-white/80 hover:bg-white/10 hover:text-white"
          aria-label={queueOpen ? t("lyrics.queue.close") : t("lyrics.queue.open")}
          onClick={() => setQueueOpen(!queueOpen)}
        >
          <ListMusic className={cn("size-5", queueOpen && "text-accent")} />
        </button>
      </div>

      {/* Progress bar at bottom */}
      <div className="relative z-20 h-3 bg-white/10 rounded-full overflow-hidden mb-6 px-5">
        <div className="h-full bg-gradient-to-r from-accent to-[#d61d33] shadow-[0_0_12px_rgba(250,35,59,0.6)]"
          style={{ width: `${ratio * 100}%` }}
        />
      </div>
    </div>
  );
}
