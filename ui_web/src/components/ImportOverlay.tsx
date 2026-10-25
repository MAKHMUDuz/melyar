import { X } from "lucide-react";
import { useLibraryImport } from "@/lib/import";
import { t, tCount, useLocale } from "@/lib/i18n";
import { useLibraryStore } from "@/store";

export type ImportFormat = "spotify" | "json" | "csv" | "m3u" | "pls";

const FORMAT_INFO: Record<ImportFormat, { label: string; description: string }> = {
  spotify: {
    label: "Spotify JSON Export",
    description: "Export from Spotify account (YourLibrary.json)",
  },
  json: {
    label: "JSON Array",
    description: "JSON array of tracks with title, artist, album fields",
  },
  csv: {
    label: "CSV File",
    description: "CSV file with header row (title, artist, album columns)",
  },
  m3u: {
    label: "M3U Playlist",
    description: ".m3u playlist file with #EXTINF entries",
  },
  pls: {
    label: "PLS Playlist",
    description: ".pls playlist file with INI-style entries",
  },
};

/**
 * The overlay that shows while a library import is running. It reports how
 * many tracks were matched so far and how many could not be found.
 */
export function ImportOverlay() {
  // Subscribe, or the headings stay in the previous language after a switch.
  useLocale();
  const running = useLibraryImport((s) => s.running);
  const progress = useLibraryImport((s) => s.progress);
  const cancel = useLibraryImport((s) => s.cancel);
  const reloadFavorites = useLibraryStore((s) => s.loadFavorites);

  if (!running) return null;

  const percent = progress.total > 0 ? Math.round((progress.current / progress.total) * 100) : 0;

  return (
    <div
      className="settings-overlay fixed inset-0 z-50 flex items-center justify-center p-6"
      role="dialog"
      aria-modal="true"
      aria-label={t("import.title")}
    >
      <div className="settings-panel w-full max-w-sm rounded-2xl shadow-2xl p-6">
        <div className="flex items-start justify-between gap-4 mb-4">
          <div>
            <h2 className="text-[15px] font-bold tracking-tight text-text">
              {t("import.title")}
            </h2>
            <p className="text-[12px] text-text-tertiary mt-1">
              {tCount("import.progress", progress.total, {
                total: progress.total,
                current: progress.current,
              })}
            </p>
          </div>
          <button
            className="btn-glass icon-btn text-text hover:bg-hover"
            aria-label={t("import.cancel")}
            onClick={() => {
              cancel();
              reloadFavorites();
            }}
          >
            <X className="size-4" />
          </button>
        </div>

        <div className="h-1.5 w-full rounded-full bg-white/10 overflow-hidden">
          <div
            className="h-full rounded-full bg-accent transition-[width] duration-300"
            style={{ width: `${percent}%` }}
          />
        </div>

        <div className="mt-3 flex items-center justify-between text-[11.5px] text-text-tertiary">
          <span>{tCount("import.matched", progress.matched)}</span>
          <span>{tCount("import.notFound", progress.skipped)}</span>
        </div>
      </div>
    </div>
  );
}
