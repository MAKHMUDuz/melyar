import { useEffect, useRef } from "react";
import { Check, Download, Link2, Loader2, Trash2, X } from "lucide-react";
import { cn } from "@/lib/cn";
import { useUIStore } from "@/store";
import { useDownloadStore } from "@/stores/downloadStore";
import { Artwork } from "@/components/Toolbar";
import type { TrackMeta } from "@/lib/types";
import { t, useLocale } from "@/lib/i18n";

/**
 * The share sheet.
 *
 * It offers the two things a listener actually wants from a track: a link
 * they can pass on, and a copy they can keep. The download writes the audio
 * to disk, and the player prefers that file over the stream, so the track
 * plays with no connection at all.
 */
export function ShareDialog() {
  useLocale();
  const track = useUIStore((s) => s.shareTrack);
  const setShareTrack = useUIStore((s) => s.setShareTrack);
  const toastNote = useUIStore((s) => s.toastNote);
  const downloading = useDownloadStore((s) => s.downloading);
  const downloaded = useDownloadStore((s) => s.downloaded);
  const download = useDownloadStore((s) => s.download);
  const remove = useDownloadStore((s) => s.remove);
  const isDownloaded = useDownloadStore((s) => s.isDownloaded);

  const panelRef = useRef<HTMLDivElement>(null);
  const copyRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!track) return;
    // Focus the first control so the sheet is usable from the keyboard.
    copyRef.current?.focus();

    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.stopPropagation();
        setShareTrack(null);
      }
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [track, setShareTrack]);

  if (!track) return null;

  const id = track.id;
  const isBusy = Boolean(downloading[id]);
  const hasCopy = Boolean(downloaded[id]) || isDownloaded(id);

  const handleCopy = () => {
    const text = `${track.title} - ${track.artist}`;
    const url = `https://music.youtube.com/watch?v=${id}`;
    const payload = `${text}\n${url}`;
    navigator.clipboard
      .writeText(payload)
      .then(() => toastNote(t("download.linkCopied")))
      .catch(() => toastNote(t("download.linkFailed")));
  };

  const handleDownload = () => {
    download(track as TrackMeta);
  };

  const handleRemove = () => {
    remove(track as TrackMeta);
    toastNote(t("download.removed"));
  };

  return (
    <div
      className="fixed inset-0 z-[95] flex items-center justify-center p-6"
      role="dialog"
      aria-modal="true"
      aria-label={t("download.share")}
    >
      <div
        className="absolute inset-0 bg-black/60 backdrop-blur-sm"
        onClick={() => setShareTrack(null)}
        aria-hidden="true"
      />

      <div
        ref={panelRef}
        className="relative z-[96] w-full max-w-sm rounded-[24px] border border-glass-border bg-panel shadow-2xl shadow-glass-shadow-xl overflow-hidden"
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 pt-4 pb-3 border-b border-glass-border">
          <h2 className="text-[15px] font-bold tracking-tight text-text">
            {t("download.share")}
          </h2>
          <button
            className="icon-btn text-text-secondary hover:bg-glass-hover hover:text-text"
            aria-label={t("common.close")}
            onClick={() => setShareTrack(null)}
          >
            <X className="size-5" />
          </button>
        </div>

        {/* Track */}
        <div className="px-5 py-4 flex items-center gap-3">
          <Artwork src={track.thumbnail} label={track.title} className="size-12 rounded-lg" />
          <div className="min-w-0 flex-1">
            <p className="truncate text-[15px] font-medium text-text">{track.title}</p>
            <p className="truncate text-[13px] text-text-secondary">{track.artist}</p>
          </div>
        </div>

        {/* Actions */}
        <div className="px-4 pb-4 flex flex-col gap-1.5">
          <button
            ref={copyRef}
            className="compact-tile flex items-center gap-3 px-3 py-3 rounded-xl hover:bg-white/5 transition-colors text-left"
            onClick={handleCopy}
          >
            <span className="grid size-9 place-items-center rounded-full bg-white/5 text-text-secondary">
              <Link2 className="size-4" />
            </span>
            <span className="min-w-0 flex-1">
              <span className="block text-sm font-medium text-text">{t("download.copyLink")}</span>
              <span className="block text-[12px] text-text-tertiary truncate">{t("download.copyLinkHint")}</span>
            </span>
            <Check className="size-4 text-text-tertiary opacity-40" />
          </button>

          {hasCopy ? (
            <button
              className="compact-tile flex items-center gap-3 px-3 py-3 rounded-xl hover:bg-white/5 transition-colors text-left"
              onClick={handleRemove}
            >
              <span className="grid size-9 place-items-center rounded-full bg-white/5 text-text-secondary">
                <Trash2 className="size-4" />
              </span>
              <span className="min-w-0 flex-1">
                <span className="block text-sm font-medium text-text">{t("download.removeDownload")}</span>
                <span className="block text-[12px] text-text-tertiary">{t("download.offlineReady")}</span>
              </span>
            </button>
          ) : (
            <button
              className="compact-tile flex items-center gap-3 px-3 py-3 rounded-xl hover:bg-white/5 transition-colors text-left disabled:opacity-60 disabled:pointer-events-none"
              onClick={handleDownload}
              disabled={isBusy}
            >
              <span
                className={cn(
                  "grid size-9 place-items-center rounded-full",
                  isBusy ? "bg-accent-soft text-accent-text" : "bg-white/5 text-text-secondary",
                )}
              >
                {isBusy ? (
                  <Loader2 className="size-4 animate-spin" />
                ) : (
                  <Download className="size-4" />
                )}
              </span>
              <span className="min-w-0 flex-1">
                <span className="block text-sm font-medium text-text">
                  {isBusy ? t("download.downloading") : t("download.download")}
                </span>
                <span className="block text-[12px] text-text-tertiary">{t("download.offlineHint")}</span>
              </span>
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
