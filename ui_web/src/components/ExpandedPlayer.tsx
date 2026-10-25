import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ChevronDown,
  Heart,
  ListMusic,
  Mic2,
  MoreHorizontal,
  Pause,
  Play,
  Repeat,
  Repeat1,
  Share2,
  Shuffle,
  SkipBack,
  SkipForward,
  Volume1,
  Volume2,
  VolumeX,
  ExternalLink,
  Settings,
} from "lucide-react";
import { cn } from "@/lib/cn";
import { formatTime } from "@/lib/lyrics";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { useLibraryStore } from "@/store";
import { usePlayer } from "@/store";
import { Artwork } from "@/components/Toolbar";
import { activeLineIndex, parseLyrics } from "@/lib/lyrics";
import { GlassSlider, PlayPauseMorph } from "@/components/GlassComponents";
import type { TrackMeta } from "@/lib/types";
import { t, useLocale } from "@/lib/i18n";
import { useSettings } from "@/lib/settings";

const VOLUME_BAR_H = 4;

function LyricLine({ text, isActive, isPast, onClick }: { text: string; isActive: boolean; isPast: boolean; onClick: () => void }) {
  return (
    <p
      className={cn(
        "text-center px-4 py-2 cursor-pointer select-text transition-all duration-300 ease-out",
        isActive ? "text-white font-bold text-[20px] md:text-[24px] font-semibold scale-105 opacity-100" : "text-white/50 text-[16px] md:text-[18px] opacity-100",
        isPast ? "text-white/55" : "",
      )}
      onClick={onClick}
      style={{ transform: isActive ? "scale(1.02)" : "scale(1)" }}
    >
      {text}
    </p>
  );
}

export function ExpandedPlayer() {
  useLocale();
  const open = useUIStore((s) => s.expandedPlayerOpen);
  const setOpen = useUIStore((s) => s.setExpandedPlayerOpen);
  const current = usePlaybackStore((s) => s.current);
  const playing = usePlaybackStore((s) => s.playing);
  const position = usePlaybackStore((s) => s.position);
  const duration = usePlaybackStore((s) => s.duration);
  const volume = usePlaybackStore((s) => s.volume);
  const muted = usePlaybackStore((s) => s.muted);
  const shuffle = usePlaybackStore((s) => s.shuffle);
  const repeat = usePlaybackStore((s) => s.repeat);
  const lyricsRaw = useUIStore((s) => s.lyricsRaw);
  const lyricsOpen = useUIStore((s) => s.lyricsOpen);
  const queueOpen = useUIStore((s) => s.queueOpen);
  const queue = useLibraryStore((s) => s.queue);
  const liked = useLibraryStore((s) => s.liked);
  const setSettingsOpen = useSettings((s) => s.setOpen);

  const toggle = usePlaybackStore((s) => s.toggle);
  const next = usePlaybackStore((s) => s.next);
  const prev = usePlaybackStore((s) => s.prev);
  const setVolume = usePlaybackStore((s) => s.setVolume);
  const toggleMute = usePlaybackStore((s) => s.toggleMute);
  const toggleShuffle = usePlaybackStore((s) => s.toggleShuffle);
  const cycleRepeat = usePlaybackStore((s) => s.cycleRepeat);
  const toggleLike = useLibraryStore((s) => s.toggleLike);
  const seekSec = usePlaybackStore((s) => s.seekSec);
  const seekRatio = usePlaybackStore((s) => s.seekRatio);
  const setLyricsOpen = useUIStore((s) => s.setLyricsOpen);
  const setQueueOpen = useUIStore((s) => s.setQueueOpen);
  const { playQueueIndex } = usePlayer();
  const openMenu = useUIStore((s) => s.openMenu);
  const toastNote = useUIStore((s) => s.toastNote);

  const moreBtnRef = useRef<HTMLButtonElement>(null);
  const timelineRef = useRef<HTMLDivElement>(null);
  const dragRef = useRef(false);
  const [hoverTime, setHoverTime] = useState<number | null>(null);
  const [hoverX, setHoverX] = useState<number | null>(null);
  const [showControls, setShowControls] = useState(true);
  // Held in a ref for the same reason as LyricsOverlay: a timer in state gave
  // the effect a new dependency on every reset, so it re-ran forever and the
  // controls never actually hid.
  const controlsTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const parsed = useMemo(() => parseLyrics(lyricsRaw), [lyricsRaw]);
  const active = parsed.synced ? activeLineIndex(parsed.lines, position) : -1;
  const isLiked = current ? liked.some((t) => t.id === current.id) : false;
  const ratio = duration > 0 ? Math.min(1, position / duration) : 0;

  // Auto-hide controls after 4s of inactivity (lyrics-only style)
  const resetControlsTimer = useCallback(() => {
    if (controlsTimerRef.current) clearTimeout(controlsTimerRef.current);
    setShowControls(true);
    controlsTimerRef.current = setTimeout(() => setShowControls(false), 4000);
  }, []);

  useEffect(() => {
    resetControlsTimer();
    return () => {
      if (controlsTimerRef.current) clearTimeout(controlsTimerRef.current);
    };
  }, [resetControlsTimer]);

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

  const handleTimelinePointerDown = (e: React.PointerEvent) => {
    dragRef.current = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const r = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
    seekRatio(r);
  };

  const handleTimelinePointerMove = (e: React.PointerEvent) => {
    if (!dragRef.current) return;
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const r = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
    seekRatio(r);
  };

  const handleTimelinePointerUp = () => {
    dragRef.current = false;
  };

  const handleTimelineMouseMove = (e: React.MouseEvent) => {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const r = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
    setHoverTime(r * duration);
    setHoverX(e.clientX - rect.left);
  };

  const handleTimelineMouseLeave = () => {
    setHoverTime(null);
    setHoverX(null);
  };

  const handleVolumeChange = (value: number) => {
    setVolume(value);
  };

  const handleMoreClick = () => {
    if (!current) return;
    const r = moreBtnRef.current?.getBoundingClientRect();
    if (!r) return;
    openMenu({ track: current, x: r.left, y: r.bottom + 6 });
  };

  const handleShareClick = () => {
    if (!current) return;
    const text = `${current.title} - ${current.artist}`;
    const share = (navigator as {
      share?: (data: { title?: string; text?: string; url?: string }) => Promise<void>;
    }).share;
    if (share) {
      share({ title: current.title, text, url: `melyar://track/${current.id}` }).catch(() => {});
    }
    navigator.clipboard.writeText(text).then(() => toastNote(t("expanded.linkCopied")));
  };

  const VolIcon = muted || volume === 0 ? VolumeX : volume < 0.5 ? Volume1 : Volume2;

  // Ambient shadow color from artwork (placeholder — dominant color extraction not available)
  const ambientColor = "radial-gradient(circle at 30% 30%, rgba(250, 35, 59, 0.35), transparent 60%)";

  return (
    <div
      className="expanded-player-overlay"
      onClick={() => setOpen(false)}
      role="dialog"
      aria-modal="true"
      aria-label={t("expanded.aria")}
    >
      {/* Ambient shadow from artwork */}
      {current?.thumbnail && (
        <div className="expanded-player-ambient" style={{ background: ambientColor }} />
      )}
      {current?.thumbnail && (
        <img src={current.thumbnail} alt="" className="expanded-player-backdrop" />
      )}
      <div className="expanded-player-veil" />

      <div
        className="expanded-player-panel flex h-full flex-col glass-strong backdrop-blur-xl border border-white/10 shadow-2xl"
        tabIndex={-1}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header — top bar */}
        <header className="flex items-center justify-between px-5 pt-4 pb-3 border-b border-white/10 bg-black/20 backdrop-blur-md z-20">
          <button
            className="btn-glass text-white/90 hover:bg-white/10 hover:text-white"
            aria-label={t("expanded.minimize")}
            onClick={() => setOpen(false)}
          >
            <ChevronDown className="size-6" />
          </button>
          <div className="min-w-0 flex-1 mx-4">
            <p className="truncate text-[12px] font-medium text-white/60 uppercase tracking-wider">
              {t("expanded.nowPlaying")}
            </p>
            <p className="truncate text-[18px] font-bold text-white">
              {current?.title ?? "Melyar"}
            </p>
            <p className="truncate text-[14px] text-white/70">
              {current?.artist ?? t("expanded.unknownArtist")}
            </p>
          </div>
          <div className="flex items-center gap-2">
            <button
              className="btn-glass text-white/90 hover:bg-white/10 hover:text-white"
              aria-label={t("settings.title")}
              onClick={() => setSettingsOpen(true)}
            >
              <Settings className="size-6" />
            </button>
            <button
              ref={moreBtnRef}
              className="btn-glass text-white/90 hover:bg-white/10 hover:text-white"
              aria-label={t("expanded.moreActions")}
              onClick={handleMoreClick}
            >
              <MoreHorizontal className="size-6" />
            </button>
          </div>
        </header>

        {/* Main content — scrollable */}
        <div className="flex-1 overflow-y-auto px-5 py-6">
          {/* Cover artwork */}
          <div className="flex justify-center mb-5 relative group">
            <div
              className="cover-expanded relative rounded-[20px] overflow-hidden shadow-2xl shadow-accent-glow/40 bg-panel glass-strong border border-white/10"
              style={{ width: "min(260px, 28vh)", height: "min(260px, 28vh)" }}
            >
              {current?.thumbnail ? (
                <img src={current.thumbnail} alt="" className="h-full w-full object-cover" />
              ) : (
                <div className="cover-fallback h-full w-full flex items-center justify-center">
                  <svg className="size-[96px] opacity-70" viewBox="0 0 64 64">
                    <path d="M20 44V20l14 8 14-8v24" fill="none" stroke="currentColor" strokeWidth="6" strokeLinecap="round" strokeLinejoin="round" />
                  </svg>
                </div>
              )}
              {/* Volume column beside the cover. It is a vertical slider in its
                  own right: rotating a horizontal one mapped the pointer across
                  a rotated bounding box eight pixels wide, so a millimetre of
                  jitter threw the volume from one end to the other, and the bar
                  itself was squeezed into a container with no room to draw. */}
              <div className="absolute top-1/2 right-[-14px] -translate-y-1/2 h-24 w-3 flex items-center justify-center rounded-full bg-white/10 backdrop-blur-sm border border-white/20">
                <GlassSlider
                  orientation="vertical"
                  value={muted ? 0 : volume}
                  onChange={handleVolumeChange}
                  min={0}
                  max={1}
                  step={0.01}
                  trackHeight={VOLUME_BAR_H}
                  thumbSize={14}
                  className="h-full"
                  aria-label={t("toolbar.volume")}
                />
              </div>
            </div>
          </div>

          {/* Song info */}
          <div className="text-center mb-6">
            {current?.album && (
              <p className="text-xs font-semibold uppercase tracking-widest text-text-tertiary mb-2">
                {current.album}
              </p>
            )}
            <h1 className="text-[22px] md:text-[28px] font-bold tracking-tight text-white mb-2">
              {current?.title ?? "Melyar"}
            </h1>
            <p className="text-[15px] font-medium text-white/80 mb-5">
              {current?.artist ?? t("expanded.unknownArtist")}
            </p>

            <div className="flex items-center justify-center gap-4">
              <button
                className="btn-glass text-white/90 hover:bg-white/10 hover:text-white transition-all"
                aria-label={isLiked ? t("menu.unlike") : t("menu.like")}
                onClick={() => current && toggleLike(current)}
              >
                <Heart className={cn("size-6", isLiked && "fill-accent text-accent scale-110 transition-transform")} />
              </button>
              <button
                className="btn-glass text-white/90 hover:bg-white/10 hover:text-white transition-all"
                aria-label={t("expanded.share")}
                onClick={handleShareClick}
              >
                <Share2 className="size-6" />
              </button>
            </div>
          </div>

          {/* Progress bar */}
          <div className="mb-6">
            <div
              ref={timelineRef}
              className="relative h-6 bg-white/10 rounded-full cursor-pointer overflow-hidden"
              onPointerDown={handleTimelinePointerDown}
              onPointerMove={handleTimelinePointerMove}
              onPointerUp={handleTimelinePointerUp}
              onPointerLeave={handleTimelinePointerUp}
              onMouseMove={handleTimelineMouseMove}
              onMouseLeave={handleTimelineMouseLeave}
              role="slider"
              aria-label={t("toolbar.seek")}
              aria-valuemin={0}
              aria-valuemax={Math.floor(duration)}
              aria-valuenow={Math.floor(position)}
              tabIndex={0}
            >
              <div className="absolute left-0 top-0 bottom-0 rounded-full bg-gradient-to-r from-accent to-[#d61d33] shadow-[0_0_12px_rgba(250,35,59,0.6)] transition-all duration-100"
                style={{ width: `${ratio * 100}%` }}
              />
              <div
                className="absolute left-[calc(var(--ratio,0)*100%)] top-1/2 -translate-y-1/2 -translate-x-1/2 size-6 bg-white border border-white/20 rounded-full shadow-lg transition-all duration-100"
                style={{ left: `${ratio * 100}%` }}
              />
            </div>
            <div className="flex justify-between text-[11px] font-medium text-white/60 mt-2 tabular-nums">
              <span>{formatTime(position)}</span>
              <span>{formatTime(duration)}</span>
            </div>
            {hoverTime !== null && hoverX !== null && (
              <div
                className="absolute -bottom-10 transform -translate-x-1/2"
                style={{ left: `${hoverX}px` }}
              >
                <span className="px-2.5 py-1 text-[11px] font-medium bg-white text-black rounded-full shadow-lg whitespace-nowrap">
                  {formatTime(hoverTime)}
                </span>
              </div>
            )}
          </div>

          {/* Lyrics section — BloomeeTunes style: current line prominent, context lines around it */}
          {parsed.synced && parsed.lines.length > 0 && active >= 0 && (
            <div className="mb-8 flex flex-col items-center gap-2">
              {active > 0 && (
                <LyricLine
                  text={parsed.lines[active - 1].text}
                  isActive={false}
                  isPast={true}
                  onClick={() => seekSec(parsed.lines[active - 1].time)}
                />
              )}
              <LyricLine
                text={parsed.lines[active].text}
                isActive={true}
                isPast={false}
                onClick={() => seekSec(parsed.lines[active].time)}
              />
              {active < parsed.lines.length - 1 && (
                <LyricLine
                  text={parsed.lines[active + 1].text}
                  isActive={false}
                  isPast={false}
                  onClick={() => seekSec(parsed.lines[active + 1].time)}
                />
              )}
            </div>
          )}
          {parsed.synced && parsed.lines.length > 0 && active < 0 && (
            <p className="text-center text-white/40 text-sm mt-4">{t("expanded.lyricsLoading")}</p>
          )}

          {/* Controls row — one centered group, in the order every
              reference player uses: shuffle, previous, play, next, repeat. */}
          <div className="flex items-center justify-center gap-5 px-2">
            <button
              className="transport-btn-large"
              data-active={shuffle}
              aria-label={t("toolbar.shuffle")}
              onClick={toggleShuffle}
            >
              <Shuffle className={cn("size-6", shuffle && "text-accent")} />
            </button>
            <button
              className="transport-btn-large"
              aria-label={t("toolbar.prev")}
              onClick={prev}
            >
              <SkipBack className="size-7 fill-current" />
            </button>
            <button
              className="play-pause-main"
              aria-label={playing ? t("toolbar.pause") : t("toolbar.play")}
              onClick={toggle}
            >
              {playing ? (
                <Pause className="size-8 fill-current ml-0.5" />
              ) : (
                <Play className="size-8 fill-current ml-1" />
              )}
            </button>
            <button
              className="transport-btn-large"
              aria-label={t("toolbar.next")}
              onClick={next}
            >
              <SkipForward className="size-7 fill-current" />
            </button>
            <button
              className="transport-btn-large"
              data-active={repeat !== 0}
              aria-label={t("toolbar.repeat")}
              onClick={cycleRepeat}
            >
              {repeat === 2 ? (
                <Repeat1 className="size-6 text-accent" />
              ) : (
                <Repeat className={cn("size-6", repeat === 1 && "text-accent")} />
              )}
            </button>
          </div>

          {/* Bottom row: lyrics + queue + external link */}
          <div className="mt-6 flex items-center justify-center gap-4">
            <button
              className="btn-glass text-white/80 hover:bg-white/10 hover:text-white"
              aria-label={lyricsOpen ? t("toolbar.lyrics.close") : t("toolbar.lyrics.open")}
              onClick={() => setLyricsOpen(!lyricsOpen)}
            >
              <Mic2 className={cn("size-5", lyricsOpen && "text-accent")} />
            </button>
            <button
              className="btn-glass text-white/80 hover:bg-white/10 hover:text-white"
              aria-label={queueOpen ? t("lyrics.queue.close") : t("lyrics.queue.open")}
              onClick={() => setQueueOpen(!queueOpen)}
            >
              <ListMusic className={cn("size-5", queueOpen && "text-accent")} />
            </button>
            <button
              className="btn-glass text-white/80 hover:bg-white/10 hover:text-white"
              aria-label={t("expanded.externalLink")}
              onClick={handleShareClick}
            >
              <ExternalLink className="size-5" />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}