import { Clock3, Download, Heart, Library, Music2 } from "lucide-react";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { useLibraryStore } from "@/store";
import { useLibraryImport } from "@/lib/import";
import { allKnown, findAlbum, findArtist, groupAlbums, groupArtists, libraryTracks } from "@/lib/registry";
import { TrackTable } from "@/components/TrackTable";
import { Artwork } from "@/components/Toolbar";
import { formatTime } from "@/lib/lyrics";
import type { View } from "@/lib/types";
import { t, tCount, useLocale } from "@/lib/i18n";

function LibraryHeader({
  icon: Icon,
  kicker,
  title,
  meta,
  gradient,
  round = false,
}: {
  icon: typeof Heart;
  kicker: string;
  title: string;
  meta: string;
  gradient: string;
  round?: boolean;
}) {
  return (
    <header className="mb-8 flex items-end gap-6">
      <div
        className={`cover grid ${round ? "rounded-full" : ""} size-36 shrink-0 place-items-center text-white lg:size-40`}
        style={{ background: gradient }}
      >
        <Icon className="size-12 opacity-90" />
      </div>
      <div className="min-w-0 pb-1">
        <p className="text-xs font-bold uppercase tracking-wider text-text-tertiary">{kicker}</p>
        <h1 className="mt-1 truncate text-4xl font-bold tracking-tight lg:text-5xl">{title}</h1>
        <p className="mt-2 text-sm text-text-tertiary">{meta}</p>
      </div>
    </header>
  );
}

export function SongsView() {
  useLocale();
  const liked = useLibraryStore((s) => s.liked);
  const history = useLibraryStore((s) => s.history);
  const library = libraryTracks(liked, history);
  const totalSecs = library.reduce((acc, t) => acc + (t.duration || 0), 0);
  return (
    <div className="px-6 pb-10 pt-6 lg:px-9">
      <LibraryHeader
        icon={Library}
        kicker={t("library.kicker")}
        title={t("library.songs.title")}
        meta={tCount("library.countDuration", library.length, { time: formatTime(totalSecs) })}
        gradient="linear-gradient(135deg,#8e8e93,#48484a)"
      />
      <TrackTable tracks={library} emptyText={t("library.songs.empty")} />
    </div>
  );
}

export function LikedView() {
  useLocale();
  const liked = useLibraryStore((s) => s.liked);
  const playTrack = usePlaybackStore((s) => s.playTrack);
  const startImport = useLibraryImport((s) => s.start);
  const totalSecs = liked.reduce((acc, t) => acc + (t.duration || 0), 0);
  return (
    <div className="px-6 pb-10 pt-6 lg:px-9">
      <LibraryHeader
        icon={Heart}
        kicker={t("library.liked.kicker")}
        title={t("library.liked.title")}
        meta={tCount("library.countDuration", liked.length, { time: formatTime(totalSecs) })}
        gradient="linear-gradient(135deg,#fc5c7d,#fa233b)"
      />
      {liked.length > 0 ? (
        <button
          className="play-btn-main mb-5"
          aria-label={t("library.playAll")}
          onClick={() => {
            const first = liked[0];
            if (first) playTrack(first, liked);
          }}
        >
          <svg viewBox="0 0 24 24" className="ml-0.5 size-5 fill-current"><path d="M8 5v14l11-7z" /></svg>
        </button>
      ) : null}
      <div className="mb-5 flex items-center gap-3">
        <button
          className="btn-glass chip text-xs inline-flex items-center gap-1.5"
          onClick={() => void startImport()}
        >
          <Download className="size-3.5" />
          {t("import.action")}
        </button>
        <span className="text-[11.5px] text-text-tertiary">{t("import.hint")}</span>
      </div>
      <TrackTable tracks={liked} emptyText={t("library.liked.empty")} />
    </div>
  );
}

export function HistoryView() {
  useLocale();
  const history = useLibraryStore((s) => s.history);
  const playTrack = usePlaybackStore((s) => s.playTrack);
  return (
    <div className="px-6 pb-10 pt-6 lg:px-9">
      <LibraryHeader
        icon={Clock3}
        kicker={t("library.history.kicker")}
        title={t("library.history.title")}
        meta={tCount("library.count", history.length)}
        gradient="linear-gradient(135deg,#5e5ce6,#30d158)"
      />
      {history.length > 0 ? (
        <button
          className="play-btn-main mb-5"
          aria-label={t("library.playAll")}
          onClick={() => {
            const first = history[0];
            if (first) playTrack(first, history);
          }}
        >
          <svg viewBox="0 0 24 24" className="ml-0.5 size-5 fill-current"><path d="M8 5v14l11-7z" /></svg>
        </button>
      ) : null}
      <TrackTable tracks={history} emptyText={t("library.history.empty")} />
    </div>
  );
}

export function AlbumsView() {
  useLocale();
  const liked = useLibraryStore((s) => s.liked);
  const history = useLibraryStore((s) => s.history);
  const navigate = useUIStore((s) => s.navigate);
  const playTrack = usePlaybackStore((s) => s.playTrack);
  // Personal albums first, then anything else discovered via search
  const albums = [...groupAlbums(libraryTracks(liked, history)), ...groupAlbums(allKnown())].filter(
    (a, i, arr) => arr.findIndex((x) => x.key === a.key) === i,
  );

  if (albums.length === 0) {
    return (
      <div className="px-6 pb-10 pt-6 lg:px-9">
        <h1 className="mb-8 text-4xl font-bold tracking-tight">{t("library.albums.title")}</h1>
        <TrackTable tracks={[]} emptyText={t("library.albums.empty")} />
      </div>
    );
  }

  return (
    <div className="px-6 pb-10 pt-6 lg:px-9">
      <h1 className="mb-6 text-4xl font-bold tracking-tight">{t("library.albums.title")}</h1>
      <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6">
        {albums.map((album) => (
          <div key={album.key} className="grid-item group flex flex-col gap-3">
            <button
              onClick={() => navigate({ name: "album", key: album.key, label: album.name })}
              className="relative"
              aria-label={t("library.openAlbum", { name: album.name })}
            >
              <Artwork src={album.cover} label={album.name} className="aspect-square w-full" />
              <div className="absolute inset-0 grid place-items-center opacity-0 transition-opacity group-hover:opacity-100">
                <span
                  className="grid size-11 place-items-center rounded-full bg-accent text-white shadow-xl"
                  onClick={(e) => {
                    e.stopPropagation();
                    playTrack(album.tracks[0]!, album.tracks);
                  }}
                  role="button"
                  aria-label={t("library.playAlbum", { name: album.name })}
                >
                  <svg viewBox="0 0 24 24" className="ml-0.5 size-5 fill-current"><path d="M8 5v14l11-7z" /></svg>
                </span>
              </div>
            </button>
            <div className="min-w-0">
              <button className="block w-full truncate text-left text-[13.5px] font-semibold hover:underline" onClick={() => navigate({ name: "album", key: album.key, label: album.name })}>
                {album.name}
              </button>
              <p className="truncate text-[12px] text-text-tertiary">{album.artist}</p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

export function ArtistsView() {
  useLocale();
  const liked = useLibraryStore((s) => s.liked);
  const history = useLibraryStore((s) => s.history);
  const navigate = useUIStore((s) => s.navigate);
  const artists = [...groupArtists(libraryTracks(liked, history)), ...groupArtists(allKnown())].filter(
    (a, i, arr) => arr.findIndex((x) => x.key === a.key) === i,
  );

  return (
    <div className="px-6 pb-10 pt-6 lg:px-9">
      <h1 className="mb-6 text-4xl font-bold tracking-tight">{t("library.artists.title")}</h1>
      {artists.length === 0 ? (
        <TrackTable tracks={[]} emptyText={t("library.artists.empty")} />
      ) : (
        <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-6">
          {artists.map((a) => (
            <button key={a.key} className="grid-item group flex flex-col gap-3" onClick={() => navigate({ name: "artist", key: a.key, label: a.name })}>
              <Artwork src={a.cover} label={a.name} className="aspect-square w-full rounded-full" />
              <div className="min-w-0 text-center">
                <p className="truncate text-[13.5px] font-semibold">{a.name}</p>
                <p className="text-[11.5px] text-text-tertiary">{t("library.col.artist")}</p>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

export function AlbumDetailView({ albumKey }: { albumKey: string }) {
  // Subscribe. Without it the headings stay in the previous language after a
  // switch, because this view was the one that read t() without listening.
  useLocale();
  const playTrack = usePlaybackStore((s) => s.playTrack);
  const navigate = useUIStore((s) => s.navigate);
  const album = findAlbum(allKnown(), albumKey);

  if (!album) {
    return (
      <div className="px-6 pb-10 pt-6 lg:px-9">
        <p className="py-16 text-center text-sm text-text-tertiary">{t("library.albums.notFound")}</p>
      </div>
    );
  }
  const totalSecs = album.tracks.reduce((acc, t) => acc + (t.duration || 0), 0);

  return (
    <div className="px-6 pb-10 pt-6 lg:px-9">
      <header className="mb-8 flex flex-col items-center gap-6 text-center sm:flex-row sm:items-end sm:text-left">
        <Artwork src={album.cover} label={album.name} className="size-40 shrink-0 lg:size-48" />
        <div className="min-w-0">
          <p className="text-xs font-bold uppercase tracking-wider text-text-tertiary">{t("library.albums.label")}</p>
          <h1 className="mt-1 truncate text-4xl font-bold tracking-tight lg:text-5xl">{album.name}</h1>
          <p className="mt-2 text-sm text-text-tertiary">
            <span className="font-medium text-text-secondary">{album.artist}</span> · {tCount("library.count", album.tracks.length)} ·{" "}
            {formatTime(totalSecs)}
          </p>
        </div>
      </header>
      <TrackTable tracks={album.tracks} showAlbum={false} />
      <button className="mt-6 text-sm text-text-tertiary hover:text-text-secondary hover:underline" onClick={() => navigate({ name: "albums" })}>
        {t("library.albums.back")}
      </button>
    </div>
  );
}

export function ArtistDetailView({ artistKey }: { artistKey: string }) {
  // Subscribe, as in AlbumDetailView: this view read t() without listening.
  useLocale();
  const playTrack = usePlaybackStore((s) => s.playTrack);
  const navigate = useUIStore((s) => s.navigate);
  const artist = findArtist(allKnown(), artistKey);

  if (!artist) {
    return (
      <div className="px-6 pb-10 pt-6 lg:px-9">
        <p className="py-16 text-center text-sm text-text-tertiary">{t("library.artists.notFound")}</p>
      </div>
    );
  }
  const albums = groupAlbums(artist.tracks);

  return (
    <div className="px-6 pb-10 pt-6 lg:px-9">
      <header className="mb-8 flex flex-col items-center gap-6 text-center sm:flex-row sm:items-end sm:text-left">
        <Artwork src={artist.cover} label={artist.name} className="size-36 shrink-0 rounded-full lg:size-44" />
        <div className="min-w-0">
          <p className="flex items-center justify-center gap-1.5 text-xs font-bold uppercase tracking-wider text-text-tertiary sm:justify-start">
            <Music2 className="size-3.5" /> {t("library.artists.label")}
          </p>
          <h1 className="mt-1 truncate text-4xl font-bold tracking-tight lg:text-5xl">{artist.name}</h1>
          <p className="mt-2 text-sm text-text-tertiary">{tCount("library.artists.knownTracks", artist.tracks.length)}</p>
        </div>
      </header>

      {albums.length > 1 ? (
        <section className="mb-8">
          <h2 className="mb-4 text-xl font-bold tracking-tight">{t("library.albums.title")}</h2>
          <div className="flex gap-4 overflow-x-auto pb-2">
            {albums.map((album) => (
              <button key={album.key} className="card w-36 shrink-0" onClick={() => navigate({ name: "album", key: album.key, label: album.name })}>
                <Artwork src={album.cover} label={album.name} className="mb-3 aspect-square w-full" />
                <p className="truncate text-left text-[13px] font-semibold">{album.name}</p>
                <p className="text-left text-[11.5px] text-text-tertiary">{tCount("library.count", album.tracks.length)}</p>
              </button>
            ))}
          </div>
        </section>
      ) : null}

      <section>
        <h2 className="mb-3 text-xl font-bold tracking-tight">{t("library.detail.tracks")}</h2>
        <TrackTable tracks={artist.tracks} />
      </section>
    </div>
  );
}

export type { View };