import { useEffect, useState } from "react";
import { ListOrdered, Music4, RefreshCw } from "lucide-react";
import { useImportCenter } from "@/lib/importCenter";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { register } from "@/lib/registry";
import { Artwork } from "@/components/Toolbar";
import { TrackTable } from "@/components/TrackTable";
import { formatTime } from "@/lib/lyrics";
import type { TrackMeta } from "@/lib/types";
import { t, tCount, useLocale } from "@/lib/i18n";

function sourceLabel(source: string) {
  if (source.startsWith("spotify")) return t("import.source.spotify");
  if (source.startsWith("apple")) return t("import.source.apple");
  if (source.startsWith("local")) return t("import.source.local");
  if (source.startsWith("file")) return t("import.source.file");
  return source;
}

function playlistTracks(playlistId: string): TrackMeta[] {
  const bridge = typeof window !== "undefined" ? (window as any).bridge : null;
  if (!bridge || typeof bridge.importPlaylistItems !== "function") return [];
  let raw: any[] = [];
  try {
    raw = JSON.parse(bridge.importPlaylistItems(playlistId));
  } catch {
    return [];
  }
  if (!Array.isArray(raw)) return [];
  return raw
    .filter((item) => item && typeof item.itemId === "string")
    .map((item) => {
      const playable = Boolean(item.playable);
      return register({
        id: playable ? String(item.trackId || item.itemId) : String(item.itemId),
        title: String(item.title || ""),
        artist: String(item.artist || ""),
        album: String(item.album || ""),
        duration: Number(item.duration) || 0,
        thumbnail: String(item.thumbnail || ""),
        playable,
        source: String(item.source || ""),
        availability: playable ? "provider" : "metadata_only",
      });
    });
}

export function PlaylistsView() {
  useLocale();
  const snapshot = useImportCenter((s) => s.snapshot);
  const refresh = useImportCenter((s) => s.refresh);
  const navigate = useUIStore((s) => s.navigate);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const playlists = snapshot?.playlists ?? [];

  return (
    <div className="px-6 pb-10 pt-6 lg:px-9">
      <header className="mb-7 flex flex-wrap items-end justify-between gap-4">
        <div>
          <p className="text-xs font-bold uppercase tracking-wider text-text-tertiary">{t("import.playlists.kicker")}</p>
          <h1 className="mt-1 text-4xl font-bold tracking-tight">{t("import.playlists.title")}</h1>
          <p className="mt-2 text-sm text-text-tertiary">
            {playlists.length > 0 ? tCount("import.playlists.count", playlists.length) : t("import.playlists.emptyHint")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <button className="btn-glass chip inline-flex items-center gap-1.5 text-xs" onClick={() => void refresh()}>
            <RefreshCw className="size-3.5" />
            {t("common.refresh")}
          </button>
          <button
            className="btn-glass chip inline-flex items-center gap-1.5 text-xs"
            onClick={() => navigate({ name: "import" })}
          >
            <ListOrdered className="size-3.5" />
            {t("nav.import")}
          </button>
        </div>
      </header>

      {playlists.length === 0 ? (
        <div className="import-empty glass-panel">
          <div className="import-empty-icon">
            <Music4 className="size-7" />
          </div>
          <h2 className="text-lg font-semibold">{t("import.playlists.empty")}</h2>
          <p className="mt-1 text-sm text-text-tertiary">{t("import.playlists.emptyHint")}</p>
          <button className="btn-glass chip mt-5 text-xs" onClick={() => navigate({ name: "import" })}>
            {t("import.openCenter")}
          </button>
        </div>
      ) : (
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
          {playlists.map((playlist) => (
            <button
              key={playlist.id}
              className="grid-item group flex flex-col gap-3 text-left"
              onClick={() =>
                navigate({ name: "playlist", key: playlist.id, label: playlist.name })
              }
            >
              <Artwork src={playlist.thumbnail} label={playlist.name} className="aspect-square w-full" />
              <div className="min-w-0">
                <p className="truncate text-[13.5px] font-semibold">{playlist.name}</p>
                <p className="truncate text-[11.5px] text-text-tertiary">
                  {sourceLabel(playlist.source)} · {tCount("import.playlists.tracks", playlist.tracks)}
                </p>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export function PlaylistDetailView({ playlistId, label }: { playlistId: string; label?: string }) {
  useLocale();
  const playTrack = usePlaybackStore((s) => s.playTrack);
  const navigate = useUIStore((s) => s.navigate);
  const snapshot = useImportCenter((s) => s.snapshot);
  const [tracks, setTracks] = useState<TrackMeta[] | null>(null);

  useEffect(() => {
    setTracks(playlistTracks(playlistId));
  }, [playlistId]);

  const playlist = snapshot?.playlists.find((entry) => entry.id === playlistId);
  const name = playlist?.name || label || t("import.playlists.title");
  const source = playlist ? sourceLabel(playlist.source) : "";
  const totalSecs = (tracks ?? []).reduce((acc, track) => acc + (track.duration || 0), 0);
  const playable = (tracks ?? []).filter((track) => track.playable !== false);

  return (
    <div className="px-6 pb-10 pt-6 lg:px-9">
      <header className="mb-8 flex flex-col items-center gap-6 text-center sm:flex-row sm:items-end sm:text-left">
        <Artwork src={playlist?.thumbnail} label={name} className="size-40 shrink-0 lg:size-48" />
        <div className="min-w-0">
          <p className="text-xs font-bold uppercase tracking-wider text-text-tertiary">
            {t("import.playlists.label")}
            {source ? ` · ${source}` : ""}
          </p>
          <h1 className="mt-1 truncate text-4xl font-bold tracking-tight lg:text-5xl">{name}</h1>
          <p className="mt-2 text-sm text-text-tertiary">
            {tCount("import.playlists.tracks", tracks?.length ?? playlist?.tracks ?? 0)}
            {totalSecs > 0 ? ` · ${formatTime(totalSecs)}` : ""}
          </p>
        </div>
      </header>

      {playable.length > 0 ? (
        <button
          className="play-btn-main mb-5"
          aria-label={t("library.playAll")}
          onClick={() => {
            if (playable[0]) playTrack(playable[0], playable);
          }}
        >
          <svg viewBox="0 0 24 24" className="ml-0.5 size-5 fill-current"><path d="M8 5v14l11-7z" /></svg>
        </button>
      ) : null}

      {tracks === null ? (
        <p className="px-6 py-14 text-center text-sm text-text-tertiary">{t("import.playlists.loading")}</p>
      ) : (
        <TrackTable tracks={tracks} emptyText={t("import.playlists.empty")} />
      )}

      <button
        className="mt-6 text-sm text-text-tertiary hover:text-text-secondary hover:underline"
        onClick={() => navigate({ name: "playlists" })}
      >
        {t("import.playlists.back")}
      </button>
    </div>
  );
}
