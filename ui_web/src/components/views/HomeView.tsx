import { Heart, Clock3 } from "lucide-react";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { useLibraryStore } from "@/store";
import { libraryTracks } from "@/lib/registry";
import { Artwork } from "@/components/Toolbar";
import { TrackTable } from "@/components/TrackTable";
import type { TrackMeta, View } from "@/lib/types";
import { t, useLocale } from "@/lib/i18n";

const GENRES = [
  { key: "Pop", label: "Pop" },
  { key: "Rock", label: "Rock" },
  { key: "Turkish Pop", label: "home.genre.turkishPop" },
  { key: "Jazz", label: "Jazz" },
  { key: "Lo-fi", label: "Lo-fi" },
  { key: "Hip-Hop", label: "Hip-Hop" },
  { key: "Electronic", label: "home.genre.electronic" },
  { key: "Classical", label: "home.genre.classical" },
  { key: "Arabesk", label: "Arabesk" },
  { key: "Indie", label: "Indie" },
];

export function greeting() {
  const h = new Date().getHours();
  if (h < 6) return t("home.greeting.night");
  if (h < 12) return t("home.greeting.morning");
  if (h < 18) return t("home.greeting.day");
  return t("home.greeting.evening");
}

function CardTile({
  track,
  onOpen,
  round = false,
  label,
  sub,
}: {
  track?: TrackMeta;
  onOpen: () => void;
  round?: boolean;
  label: string;
  sub: string;
}) {
  return (
    <button className="grid-item group flex flex-col gap-3" onClick={onOpen}>
      <div className="relative">
        <Artwork src={track?.thumbnail} label={label} className={round ? "aspect-square w-full rounded-full" : "aspect-square w-full"} />
        <div className="absolute inset-0 grid place-items-center opacity-0 transition-opacity group-hover:opacity-100">
          <span className="grid size-11 place-items-center rounded-full bg-accent text-white shadow-xl">
            <svg viewBox="0 0 24 24" className="ml-0.5 size-5 fill-current"><path d="M8 5v14l11-7z" /></svg>
          </span>
        </div>
      </div>
      <div className="min-w-0">
        <p className="truncate text-[13.5px] font-semibold">{label}</p>
        <p className="truncate text-[12px] text-text-tertiary">{sub}</p>
      </div>
    </button>
  );
}

export function HomeView() {
  useLocale();
  const history = useLibraryStore((s) => s.history);
  const liked = useLibraryStore((s) => s.liked);
  const playTrack = usePlaybackStore((s) => s.playTrack);
  const navigate = useUIStore((s) => s.navigate);
  const setQuery = useUIStore((s) => s.setQuery);

  const library = libraryTracks(liked, history);
  const featured = history[0];

  return (
    <div className="px-6 pb-10 pt-6 lg:px-9">
      <header className="mb-8">
        <h1 className="text-3xl md:text-4xl font-bold tracking-tight">{greeting()}</h1>
        {featured ? (
          <p className="mt-1.5 text-sm text-text-tertiary">
            {t("home.resume")} <span className="font-medium text-text-secondary">{featured.artist}</span>
          </p>
        ) : (
          <p className="mt-1.5 text-sm text-text-tertiary">{t("home.subtitle")}</p>
        )}
      </header>

      {featured ? (
        <button
          className="mb-8 card flex w-full items-center gap-5 overflow-hidden"
          onClick={() => playTrack(featured, history)}
        >
          <Artwork src={featured.thumbnail} label={featured.title} className="size-24 lg:size-28" />
          <div className="min-w-0">
            <p className="text-xs font-bold uppercase tracking-wider text-accent-text">{t("home.kicker.recent")}</p>
            <p className="mt-1 truncate text-2xl md:text-3xl font-bold tracking-tight">{featured.title}</p>
            <p className="truncate text-sm text-text-tertiary">{featured.artist}</p>
          </div>
        </button>
      ) : null}

      {history.length > 0 ? (
        <section className="mb-10">
          <div className="mb-4 flex items-end justify-between">
            <h2 className="text-xl font-bold tracking-tight">{t("home.recentlyPlayed")}</h2>
            <button className="text-sm font-medium text-text-tertiary hover:text-text-secondary hover:underline" onClick={() => navigate({ name: "history" })}>
              {t("home.showAll")}
            </button>
          </div>
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4 xl:grid-cols-6">
            {history.slice(0, 6).map((t) => (
              <CardTile key={t.id} track={t} label={t.title} sub={t.artist} onOpen={() => playTrack(t, history)} />
            ))}
          </div>
        </section>
      ) : null}

      {liked.length > 0 ? (
        <section className="mb-10">
          <div className="mb-4 flex items-end justify-between">
            <h2 className="text-xl font-bold tracking-tight">{t("home.liked")}</h2>
            <button className="text-sm font-medium text-text-tertiary hover:text-text-secondary hover:underline" onClick={() => navigate({ name: "liked" })}>
              {t("home.showAll")}
            </button>
          </div>
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 md:grid-cols-4 xl:grid-cols-6">
            {liked.slice(0, 6).map((t) => (
              <CardTile key={t.id} track={t} label={t.title} sub={t.artist} onOpen={() => playTrack(t, liked)} />
            ))}
          </div>
        </section>
      ) : null}

      {library.length === 0 ? (
        <section className="card rounded-xl border border-glass-border p-8 text-center">
          <h2 className="text-lg font-bold">{t("home.libraryEmpty.title")}</h2>
          <p className="mx-auto mt-2 max-w-md text-sm text-text-tertiary">
            {t("home.libraryEmpty.body")}
          </p>
          <div className="mt-5 flex flex-wrap justify-center gap-2">
            {GENRES.slice(0, 6).map((g) => (
              <button
                key={g.key}
                className="chip"
                onClick={() => {
                  setQuery(g.key);
                  navigate({ name: "search" });
                }}
              >
                {g.label.includes(".") ? t(g.label) : g.label}
              </button>
            ))}
          </div>
        </section>
      ) : (
        <section>
          <h2 className="mb-4 text-xl font-bold tracking-tight">{t("home.genres")}</h2>
          <div className="flex flex-wrap gap-2">
            {GENRES.map((g) => (
              <button
                key={g.key}
                className="chip"
                onClick={() => {
                  setQuery(g.key);
                  navigate({ name: "search" });
                }}
              >
                {g.label.includes(".") ? t(g.label) : g.label}
              </button>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}

export function HistoryBadge() {
  return <Clock3 className="size-4" />;
}

export function LikedBadge() {
  return <Heart className="size-4" />;
}

export { TrackTable };
export type { View };