import { useEffect, useRef } from "react";
import { Search as SearchIcon } from "lucide-react";
import { usePlaybackStore } from "@/store";
import { useUIStore } from "@/store";
import { runSearch } from "@/lib/client";
import { groupArtists } from "@/lib/registry";
import { TrackTable } from "@/components/TrackTable";
import { Artwork } from "@/components/Toolbar";
import { t, useLocale } from "@/lib/i18n";

export function SearchView() {
  useLocale();
  const query = useUIStore((s) => s.query);
  const results = useUIStore((s) => s.results);
  const searching = useUIStore((s) => s.searching);
  const playTrack = usePlaybackStore((s) => s.playTrack);
  const navigate = useUIStore((s) => s.navigate);
  const inputRef = useRef<HTMLInputElement>(null);
  const timer = useRef<ReturnType<typeof setTimeout>>(0);

  // Debounced live search
  useEffect(() => {
    const q = query.trim();
    if (!q) {
      useUIStore.setState({ results: [], searching: false });
      return;
    }
    clearTimeout(timer.current);
    timer.current = setTimeout(() => runSearch(q), 350);
    return () => clearTimeout(timer.current);
  }, [query]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const artists = groupArtists(results).slice(0, 10);

  return (
    <div className="px-6 pb-10 pt-6 lg:px-9">
      <label className="mb-6 search-bar glass-input px-4 py-3 max-w-xl">
        <SearchIcon className="size-5 text-text-tertiary" />
        <input
          ref={inputRef}
          id="melyar-search"
          value={query}
          onChange={(e) => useUIStore.getState().setQuery(e.target.value)}
          placeholder={t("search.placeholder")}
          className="w-full bg-transparent text-[15px] outline-none placeholder:text-text-tertiary"
          spellCheck={false}
        />
      </label>

      {!query.trim() ? (
        <div className="grid place-items-center py-20 text-center">
          <SearchIcon className="mb-4 size-10 text-text-tertiary" />
          <p className="text-lg font-semibold">Melyar'da ara</p>
          <p className="mt-1 max-w-sm text-sm text-text-tertiary">
            {t("search.hint")}
          </p>
        </div>
      ) : searching && results.length === 0 ? (
        <div className="space-y-3">
          {Array.from({ length: 6 }).map((_, i) => (
            <div key={i} className="flex items-center gap-4 px-2">
              <div className="skeleton size-10 rounded-lg" />
              <div className="flex-1 space-y-2">
                <div className="skeleton h-3.5 w-1/3" />
                <div className="skeleton h-3 w-1/5" />
              </div>
            </div>
          ))}
        </div>
      ) : results.length === 0 ? (
        <p className="py-16 text-center text-sm text-text-tertiary">{t("search.noResults", { query })}</p>
      ) : (
        <>
          {artists.length > 0 ? (
            <section className="mb-8">
              <h2 className="mb-4 text-xl font-bold tracking-tight">{t("search.artists")}</h2>
              <div className="flex gap-4 overflow-x-auto pb-2">
                {artists.map((a) => (
                  <button key={a.key} className="card w-28 shrink-0" onClick={() => navigate({ name: "artist", key: a.key, label: a.name })}>
                    <Artwork src={a.cover} label={a.name} className="mb-3 aspect-square w-full rounded-full" />
                    <p className="truncate text-center text-[12.5px] font-medium">{a.name}</p>
                    <p className="text-center text-[11px] text-text-tertiary">{t("library.col.artist")}</p>
                  </button>
                ))}
              </div>
            </section>
          ) : null}

          <section>
            <h2 className="mb-3 text-xl font-bold tracking-tight">{t("search.songs")}</h2>
            <TrackTable tracks={results} />
          </section>
        </>
      )}
    </div>
  );
}