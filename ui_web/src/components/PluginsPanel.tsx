import { useCallback, useEffect, useRef } from "react";
import { X, Plug } from "lucide-react";
import { cn } from "@/lib/cn";
import { t, useLocale } from "@/lib/i18n";
import { GlassToggle } from "@/components/GlassComponents";
import { usePlugins, type SourceInfo } from "@/lib/plugins";

/** One source, as a row: what it is, whether it is on, and one control. */
function SourceRow({ source }: { source: SourceInfo }) {
  const toggle = usePlugins((s) => s.toggle);
  const pending = usePlugins((s) => s.pending);
  const busy = pending === source.id;

  // YouTube is the default and the anchor every bare track id resolves
  // against. The switch stays visible but disabled with a label that says
  // why, rather than vanishing and leaving the user to wonder where it went.
  const locked = source.default;

  return (
    <div className="flex items-center gap-4 py-3.5">
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline gap-2">
          <span className="text-[13.5px] font-medium text-text truncate">{source.name}</span>
          <span
            className={cn(
              "text-[10px] uppercase tracking-wide text-text-tertiary shrink-0",
              source.kind === "yt-dlp" && "text-text-tertiary",
            )}
          >
            {source.kind === "yt-dlp" ? "yt-dlp" : "API"}
          </span>
        </div>
        <p className="text-[11.5px] text-text-tertiary mt-0.5 leading-snug">
          {source.description}
        </p>
      </div>
      <div className="shrink-0 flex items-center gap-2">
        {locked ? (
          <span className="text-[11px] text-text-tertiary">{t("plugins.default")}</span>
        ) : null}
        <GlassToggle
          size="md"
          checked={source.enabled}
          disabled={locked || busy}
          // Even locked, the label names the source: "Default" on its own
          // tells a screen-reader user nothing about which row it belongs to.
          aria-label={
            locked
              ? `${source.name} — ${t("plugins.default")}`
              : `${source.name} — ${source.enabled ? t("plugins.on") : t("plugins.off")}`
          }
          onChange={(next) => void toggle(source.id, next)}
        />
      </div>
    </div>
  );
}

export function PluginsPanel() {
  useLocale();
  const open = usePlugins((s) => s.open);
  const setOpen = usePlugins((s) => s.setOpen);
  const sources = usePlugins((s) => s.sources);
  const loading = usePlugins((s) => s.loading);

  const panelRef = useRef<HTMLDivElement>(null);

  const handleEscape = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    },
    [setOpen],
  );

  useEffect(() => {
    if (!open) return;
    window.addEventListener("keydown", handleEscape);
    return () => window.removeEventListener("keydown", handleEscape);
  }, [open, handleEscape]);

  useEffect(() => {
    if (open && panelRef.current) {
      // The dialog takes focus, not a control inside it. Nothing here is
      // destructive, but a switch under the cursor is a switch a stray Enter
      // can throw — so the panel, not the first toggle, gets focus.
      panelRef.current.focus();
    }
  }, [open]);

  if (!open) return null;

  return (
    <div
      className="settings-overlay fixed inset-0 z-[var(--z-modal-panel)] flex items-center justify-center p-6"
      onClick={(e) => {
        if (e.target === e.currentTarget) setOpen(false);
      }}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label={t("plugins.title")}
        tabIndex={-1}
        className="settings-panel w-full max-w-lg max-h-[85vh] rounded-2xl shadow-2xl flex flex-col overflow-hidden"
      >
        <header className="flex items-center justify-between px-5 py-3.5 shrink-0">
          <div className="flex items-center gap-2">
            <Plug className="size-4 text-text-tertiary" />
            <h2 className="text-[15px] font-bold tracking-tight text-text">
              {t("plugins.title")}
            </h2>
          </div>
          <button
            className="size-8 rounded-full grid place-items-center text-text-tertiary transition-colors hover:bg-hover hover:text-text"
            aria-label={t("common.close")}
            onClick={() => setOpen(false)}
          >
            <X className="size-4" />
          </button>
        </header>

        <div className="scroll-thin flex-1 min-h-0 overflow-y-auto px-5 py-2">
          <p className="text-[11.5px] text-text-tertiary px-1 pb-1 leading-snug">
            {t("plugins.intro")}
          </p>
          {loading && sources.length === 0 ? (
            <p className="text-[12.5px] text-text-tertiary px-1 py-6">{t("plugins.loading")}</p>
          ) : sources.length === 0 ? (
            <p className="text-[12.5px] text-text-tertiary px-1 py-6">{t("plugins.empty")}</p>
          ) : (
            <div className="divide-y divide-line">
              {sources.map((s) => (
                <SourceRow key={s.id} source={s} />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
