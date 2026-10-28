import { Heart, MoreHorizontal, Play } from "lucide-react";
import { cn } from "@/lib/cn";
import { formatTime } from "@/lib/lyrics";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { useLibraryStore } from "@/store";
import { Artwork } from "@/components/Toolbar";
import type { TrackMeta } from "@/lib/types";
import { t, useLocale } from "@/lib/i18n";

function Row({
  track,
  index,
  context,
  showAlbum = true,
}: {
  track: TrackMeta;
  index: number;
  context: TrackMeta[];
  showAlbum?: boolean;
}) {
  useLocale();
  const current = usePlaybackStore((s) => s.current);
  const playing = usePlaybackStore((s) => s.playing);
  const likedIds = useLibraryStore((s) => s.liked);
  const menu = useUIStore((s) => s.menu);
  const playTrack = usePlaybackStore((s) => s.playTrack);
  const toggleLike = useLibraryStore((s) => s.toggleLike);
  const openMenu = useUIStore((s) => s.openMenu);
  const navigate = useUIStore((s) => s.navigate);

  const isCurrent = current?.id === track.id;
  const isLiked = likedIds.some((t) => t.id === track.id);
  const menuOpen = menu?.track.id === track.id;

  return (
    <div
      className={cn("track-row", isCurrent && "data-[current:true]", menuOpen && "data-[menu-open:true]")}
      data-current={isCurrent}
      data-menu-open={menuOpen}
      onDoubleClick={() => playTrack(track, context)}
    >
      <button
        className="grid size-7 shrink-0 place-items-center text-[12px] tabular-nums text-text-tertiary"
        aria-label={isCurrent && playing ? t("track.pause") : t("track.play")}
        onClick={() => (isCurrent ? usePlaybackStore.getState().toggle() : playTrack(track, context))}
      >
        {isCurrent && playing ? (
          <span className="eq-bars" aria-hidden>
            <i />
            <i />
            <i />
          </span>
        ) : (
          <span className="relative">
            <span className="group-hover:opacity-0">{index + 1}</span>
            <Play className="absolute inset-0 m-auto size-3.5 fill-current opacity-0 group-hover:opacity-100" />
          </span>
        )}
      </button>

      <button className="group flex min-w-0 items-center gap-3 text-left" onClick={() => (isCurrent ? usePlaybackStore.getState().toggle() : playTrack(track, context))}>
        <Artwork src={track.thumbnail} label={track.title} className="size-10" />
        <span className="min-w-0">
          <span className={cn("track-title block truncate text-[13.5px] font-medium", isCurrent && "text-accent-text")}>
            {track.title}
          </span>
          <span className="block truncate text-[12px] text-text-tertiary hover:underline">{track.artist}</span>
        </span>
      </button>

      <button
        className={cn("min-w-0 truncate text-left text-[12.5px] text-text-tertiary hover:underline", !showAlbum && "invisible")}
        onClick={() => {
          const key = `${(track.album || track.title).toLowerCase()}|${track.artist.toLowerCase()}`;
          navigate({ name: "album", key });
        }}
      >
        {track.album || "—"}
      </button>

      <span className="text-right text-[12px] tabular-nums text-text-tertiary">{formatTime(track.duration)}</span>

      <button
        className="row-extra grid size-9 place-items-center rounded-full"
        aria-label={isLiked ? t("track.unlike") : t("track.like")}
        aria-pressed={isLiked}
        onClick={() => toggleLike(track)}
      >
        <Heart className={cn("size-[15px]", isLiked ? "fill-accent text-accent-text" : "text-text-tertiary")} />
      </button>

      <button
        className={cn("grid size-9 place-items-center rounded-full", menuOpen ? "text-white" : "text-text-tertiary row-extra")}
        aria-label={t("track.options")}
        onClick={(e) => {
          const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
          openMenu({ track, x: rect.left, y: rect.bottom + 4 });
        }}
      >
        <MoreHorizontal className="size-[17px]" />
      </button>
    </div>
  );
}

export function TrackTable({
  tracks,
  showAlbum = true,
  emptyText = t("track.empty"),
}: {
  tracks: TrackMeta[];
  showAlbum?: boolean;
  emptyText?: string;
}) {
  useLocale();
  if (tracks.length === 0) {
    return <p className="px-6 py-14 text-center text-sm text-text-tertiary">{emptyText}</p>;
  }
  return (
    <div className="group/table">
      <div className="track-head">
        <span className="text-center">#</span>
        <span>{t("track.col.track")}</span>
        <span className={showAlbum ? "" : "invisible"}>{t("track.col.album")}</span>
        <span className="text-right">{t("track.col.duration")}</span>
        <span />
        <span />
      </div>
      {tracks.map((track, i) => (
        <Row key={`${track.id}-${i}`} track={track} index={i} context={tracks} showAlbum={showAlbum} />
      ))}
    </div>
  );
}