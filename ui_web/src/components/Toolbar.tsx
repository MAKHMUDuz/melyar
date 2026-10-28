import { useEffect, useRef, useState } from "react";
import {
  FolderPlus,
  ChevronLeft,
  ChevronRight,
  Heart,
  Languages,
  Check,
  Mic2,
  Repeat,
  Repeat1,
  Settings,
  Shuffle,
  SkipBack,
  SkipForward,
  ListMusic,
  Volume1,
  Volume2,
  VolumeX,
} from "lucide-react";
import { cn } from "@/lib/cn";
import { formatTime } from "@/lib/lyrics";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { useLibraryStore } from "@/store";
import { PlayPauseMorph, GlassSlider } from "@/components/GlassComponents";
import { handleUri } from "@/lib/client";
import { useLibraryImport } from "@/lib/import";
import type { TrackMeta } from "@/lib/types";
import { t, useLocale, setLocale, LOCALES } from "@/lib/i18n";

/**
 * Cover art, with a designed tile when there is none.
 *
 * A grey box with a note in it reads as "nothing here". The fallback is the
 * first letters of the title on a tinted tile instead, which reads as a
 * record sleeve whose artwork has not loaded — which is what it is.
 */
export function Artwork({
  src,
  alt,
  className,
  label,
}: {
  src?: string;
  alt?: string;
  className?: string;
  label?: string;
}) {
  const text = (label ?? alt ?? "").trim();
  const initials = text
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((w) => w[0]?.toUpperCase() ?? "")
    .join("");
  return (
    <div className={cn("cover shrink-0", className)}>
      {src ? (
        <img src={src} alt={alt ?? ""} draggable={false} className="h-full w-full object-cover" loading="lazy" />
      ) : (
        <div className="cover-fallback" aria-hidden>
          {initials ? (
            <span className="cover-initials">{initials}</span>
          ) : (
            <svg viewBox="0 0 64 64" className="size-1/3 opacity-70">
              <path d="M20 44V20l14 8 14-8v24" fill="none" stroke="currentColor" strokeWidth="6" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
          )}
        </div>
      )}
    </div>
  );
}

function WaveBars() {
  const playing = usePlaybackStore((s) => s.playing);
  if (!playing) return null;
  return (
    <div className="eq-bars flex items-end gap-1 h-4" aria-hidden>
      {[1, 2, 3, 4].map((i) => (
        <i key={i} className="rounded-full bg-accent animate-[eqb_0.8s_ease-in-out_infinite_alternate]" style={{ height: `${30 + i * 15}%`, animationDelay: `${i * 0.15}s` }} />
      ))}
    </div>
  );
}

/** The seek bar. Thin at rest, thicker under the pointer, as every player does. */
export function MiniTimeline() {
  const position = usePlaybackStore((s) => s.position);
  const duration = usePlaybackStore((s) => s.duration);
  const seekRatio = usePlaybackStore((s) => s.seekRatio);
  const drag = useRef(false);
  const barRef = useRef<HTMLDivElement>(null);

  const ratio = duration > 0 ? Math.min(1, position / duration) : 0;

  const pointRatio = (clientX: number) => {
    const rect = barRef.current?.getBoundingClientRect();
    if (!rect || rect.width === 0) return 0;
    return Math.min(1, Math.max(0, (clientX - rect.left) / rect.width));
  };

  const handlePointerDown = (e: React.PointerEvent) => {
    drag.current = true;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    seekRatio(pointRatio(e.clientX));
  };

  const handlePointerMove = (e: React.PointerEvent) => {
    if (drag.current) seekRatio(pointRatio(e.clientX));
  };

  const handlePointerUp = () => {
    drag.current = false;
  };

  return (
    <div
      ref={barRef}
      className="pb-progress"
      onPointerDown={handlePointerDown}
      onPointerMove={handlePointerMove}
      onPointerUp={handlePointerUp}
      onPointerLeave={handlePointerUp}
      role="slider"
      aria-label={t("toolbar.seek")}
      aria-valuemin={0}
      aria-valuemax={Math.floor(duration)}
      aria-valuenow={Math.floor(position)}
      tabIndex={0}
    >
      <span className="pb-progress-fill" style={{ width: `${ratio * 100}%` }} />
    </div>
  );
}

/**
 * Five controls on one surface, not five floating circles.
 *
 * A row of equally weighted pills reads as widgets assembled from a parts
 * bin. A single capsule reads as one control with modes — which is what it
 * is — and leaves the play button as the only filled element, so the eye
 * lands where the hand wants to go.
 */
function TransportCapsule() {
  useLocale();
  const playing = usePlaybackStore((s) => s.playing);
  const shuffle = usePlaybackStore((s) => s.shuffle);
  const repeat = usePlaybackStore((s) => s.repeat);
  const toggle = usePlaybackStore((s) => s.toggle);
  const next = usePlaybackStore((s) => s.next);
  const prev = usePlaybackStore((s) => s.prev);
  const toggleShuffle = usePlaybackStore((s) => s.toggleShuffle);
  const cycleRepeat = usePlaybackStore((s) => s.cycleRepeat);

  return (
    <div className="transport-capsule">
      <button className="tc-btn" data-active={shuffle} aria-label={t("toolbar.shuffle")} onClick={toggleShuffle}>
        <Shuffle className={cn("size-[15px]", shuffle && "text-accent-text")} />
      </button>
      <button className="tc-btn" aria-label={t("toolbar.prev")} onClick={prev}>
        <SkipBack className="size-[17px] fill-current" />
      </button>
      <PlayPauseMorph size="compact" playing={playing} onToggle={toggle} />
      <button className="tc-btn" aria-label={t("toolbar.next")} onClick={next}>
        <SkipForward className="size-[17px] fill-current" />
      </button>
      <button className="tc-btn" data-active={repeat !== 0} aria-label={t("toolbar.repeat")} onClick={cycleRepeat}>
        {repeat === 2 ? <Repeat1 className="size-[15px] text-accent-text" /> : <Repeat className={cn("size-[15px]", repeat === 0 ? "text-text-secondary" : "text-accent-text")} />}
      </button>
    </div>
  );
}

/** Volume: an icon and a short track that widens under the pointer. */
function VolumeControl() {
  useLocale();
  const volume = usePlaybackStore((s) => s.volume);
  const muted = usePlaybackStore((s) => s.muted);
  const setVolume = usePlaybackStore((s) => s.setVolume);
  const toggleMute = usePlaybackStore((s) => s.toggleMute);

  const VolIcon = muted || volume === 0 ? VolumeX : volume < 0.5 ? Volume1 : Volume2;

  const onWheel = (e: React.WheelEvent) => {
    const step = 0.05;
    const next = Math.min(1, Math.max(0, (muted ? 0 : volume) + (e.deltaY < 0 ? step : -step)));
    setVolume(next);
  };

  return (
    <div className="pb-volume" onWheel={onWheel}>
      <button className="pb-icon" aria-label={muted ? t("toolbar.mute.off") : t("toolbar.mute.on")} onClick={toggleMute}>
        <VolIcon className="size-[18px]" />
      </button>
      <div className="pb-volume-track">
        <GlassSlider
          value={muted ? 0 : volume}
          onChange={setVolume}
          min={0}
          max={1}
          step={0.01}
          showTooltip
          formatTooltip={(v) => `${Math.round(v * 100)}%`}
          aria-label={t("toolbar.volume")}
        />
      </div>
    </div>
  );
}

const VIEW_TITLE: Record<string, string> = {
  home: "nav.home",
  search: "nav.search",
  songs: "nav.songs",
  liked: "nav.liked",
  history: "nav.history",
  albums: "nav.albums",
  artists: "nav.artists",
  album: "nav.albums",
  artist: "nav.albums",
};

function LanguageMenu() {
  const locale = useLocale();
  const [open, setOpen] = useState(false);
  return (
    <div className="hb-menu">
      <button
        type="button"
        className="pb-icon"
        aria-haspopup="menu"
        aria-expanded={open}
        aria-label={t("settings.language")}
        onClick={() => setOpen(!open)}
      >
        <Languages className="size-[18px]" />
      </button>
      {open ? (
        <div className="hb-menu-list" role="menu">
          {LOCALES.map((entry) => {
            const active = entry.code === locale;
            return (
              <button
                key={entry.code}
                type="button"
                className={cn("hb-menu-item", active && "is-active")}
                role="menuitemradio"
                aria-checked={active}
                onClick={() => {
                  setLocale(entry.code);
                  setOpen(false);
                }}
              >
                <span className="flex-1 text-left truncate">{entry.label}</span>
                {active ? <Check className="size-3.5" /> : null}
              </button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
}

/**
 * The slim bar above the content: where you are, and how to go back.
 */
export function Header() {
  useLocale();
  const view = useUIStore((s) => s.view);
  const navIndex = useUIStore((s) => s.navIndex);
  const navStack = useUIStore((s) => s.navStack);
  const goBack = useUIStore((s) => s.goBack);
  const goForward = useUIStore((s) => s.goForward);

  const canBack = navIndex > 0;
  const canForward = navIndex < navStack.length - 1;
  const key = view.name as string;
  // An album or an artist opens onto one named thing. The label travels with
  // the view so the bar can say which, rather than naming the list you came
  // from.
  const title = ("label" in view && view.label) || t(VIEW_TITLE[key] ?? "nav.home");

  return (
    <header className="header-bar">
      <div className="hb-nav">
        <button className="pb-icon" aria-label={t("toolbar.back")} onClick={goBack} disabled={!canBack}>
          <ChevronLeft className="size-[19px]" />
        </button>
        <button className="pb-icon" aria-label={t("toolbar.forward")} onClick={goForward} disabled={!canForward}>
          <ChevronRight className="size-[19px]" />
        </button>
      </div>
      <h1 className="hb-title">{title}</h1>
      <div className="hb-right">
        <LanguageMenu />
                <button className="pb-icon" aria-label={t("import.title")} onClick={() => useLibraryImport.getState().start()}>
          <FolderPlus className="size-[18px]" />
        </button>
<button className="pb-icon" aria-label={t("settings.title")} onClick={() => void handleUri("melyar://settings")}>
          <Settings className="size-[18px]" />
        </button>
      
      </div>
    </header>
  );
}

/**
 * The bar across the bottom: what is playing, the controls, and the volume.
 *
 * Every desktop music client puts the player here, and for a reason — the
 * bottom edge is where the hand already is, and it leaves the top of the
 * window for the two things that belong there: where you are, and how to
 * get back.
 */
export function PlayerBar() {
  useLocale();
  const current = usePlaybackStore((s) => s.current);
  const position = usePlaybackStore((s) => s.position);
  const duration = usePlaybackStore((s) => s.duration);
  const playing = usePlaybackStore((s) => s.playing);
  const liked = useLibraryStore((s) => s.liked);
  const toggleLike = useLibraryStore((s) => s.toggleLike);
  const setExpandedPlayerOpen = useUIStore((s) => s.setExpandedPlayerOpen);
  const queueOpen = useUIStore((s) => s.queueOpen);
  const setQueueOpen = useUIStore((s) => s.setQueueOpen);
  const lyricsOpen = useUIStore((s) => s.lyricsOpen);
  const setLyricsOpen = useUIStore((s) => s.setLyricsOpen);

  const track: TrackMeta | null = current;
  const isLiked = track ? liked.some((x) => x.id === track.id) : false;

  return (
    <footer className="player-bar">
      <div className="pb-zone pb-left">
        <button
          className="pb-track"
          onClick={() => setExpandedPlayerOpen(true)}
          aria-label={t("toolbar.expand")}
          disabled={!track}
        >
          <Artwork src={track?.thumbnail} label={track?.title} className="pb-artwork" />
          <span className="pb-meta">
            <span className="pb-title">{track ? track.title : t("toolbar.noTrack")}</span>
            <span className="pb-artist">{track ? track.artist : "Melyar"}</span>
          </span>
        </button>
        {track ? (
          <button
            className={cn("pb-like", isLiked && "is-liked")}
            aria-label={isLiked ? t("track.unlike") : t("track.like")}
            aria-pressed={isLiked}
            onClick={() => toggleLike(track)}
          >
            <Heart className={cn("size-[16px]", isLiked && "fill-current")} />
          </button>
        ) : null}
        <span className="pb-playing-indicator">
          <WaveBars />
        </span>
      </div>

      <div className="pb-zone pb-center">
        <TransportCapsule />
        <div className="pb-timeline">
          <span className="pb-time">{formatTime(position)}</span>
          <MiniTimeline />
          <span className="pb-time">{formatTime(duration)}</span>
        </div>
      </div>

      <div className="pb-zone pb-right">
        <button
          className="pb-icon"
          data-active={lyricsOpen}
          aria-label={lyricsOpen ? t("toolbar.lyrics.close") : t("toolbar.lyrics.open")}
          onClick={() => setLyricsOpen(!lyricsOpen)}
        >
          <Mic2 className="size-[18px]" />
        </button>
        <button
          className="pb-icon"
          data-active={queueOpen}
          aria-label={t("toolbar.queue")}
          onClick={() => setQueueOpen(!queueOpen)}
        >
          <ListMusic className="size-[18px]" />
        </button>
        <VolumeControl />
      </div>
    </footer>
  );
}
