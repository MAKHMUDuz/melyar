import React from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Plug, ChevronLeft, DoorOpen } from "lucide-react";
import { cn } from "@/lib/cn";
import { t, useLocale } from "@/lib/i18n";
import { GlassToggle } from "@/components/GlassComponents";
import { usePlugins, type SourceInfo } from "@/lib/plugins";
import { useUIStore } from "@/store";
import { getBridge } from "@/lib/client";

/** One source, as a row: what it is, whether it is on, and one control. */
function SourceRow({ source }: { source: SourceInfo }) {
  const toggle = usePlugins((s) => s.toggle);
  const pending = usePlugins((s) => s.pending);
  const busy = pending === source.id;

  const locked = source.default;

  return (
    <div className="flex items-center gap-4 py-3.5 border-b border-line/40 last:border-0">
      <div className="min-w-0 flex-1">
        <div className="flex items-baseline gap-2">
          <span className="text-[13.5px] font-medium text-text truncate">{source.name}</span>
          <span className="text-[10px] uppercase tracking-wide text-text-tertiary shrink-0">
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

function AccountCard({
  icon,
  name,
  description,
  connected,
  onConnect,
  onImport,
}: {
  icon: React.ReactNode;
  name: string;
  description: string;
  connected: boolean;
  onConnect: () => void;
  onImport?: () => void;
}) {
  return (
    <div className="rounded-xl border border-line/40 bg-panel/40 p-4 flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <div className="size-10 rounded-lg bg-accent/10 grid place-items-center text-accent-text shrink-0">
          {icon}
        </div>
        <div className="min-w-0">
          <p className="text-[13.5px] font-medium text-text truncate">{name}</p>
          <p className="text-[11px] text-text-tertiary truncate">{description}</p>
        </div>
      </div>
      <div className="flex gap-2">
        <button
          className={cn(
            "btn-glass chip text-xs",
            connected && "bg-accent/15 text-accent-text",
          )}
          onClick={onConnect}
        >
          {connected ? t("plugins.connected") : t("plugins.connect")}
        </button>
        {connected && onImport && (
          <button
            className="btn-glass chip text-xs"
            onClick={onImport}
          >
            {t("import.action") || "Import"}
          </button>
        )}
      </div>
    </div>
  );
}

export function PluginsView() {
  useLocale();
  const navigate = useUIStore((s) => s.navigate);
  const sources = usePlugins((s) => s.sources);
  const loading = usePlugins((s) => s.loading);
  const load = usePlugins((s) => s.load);

  const [accounts, setAccounts] = useState<any[]>([
    { id: "spotify", name: "Spotify", connected: false, user: "", requiresConfig: true, configHint: "" },
    { id: "google", name: "Google", connected: false, user: "", requiresConfig: true, configHint: "" },
    { id: "yandex", name: "Yandex Music", connected: false, user: "", requiresConfig: true, configHint: "" },
    { id: "apple", name: "Apple Music", connected: false, user: "", requiresConfig: true, configHint: "" },
    { id: "custom", name: "Custom API", connected: false, user: "", requiresConfig: true, configHint: "" },
  ]);
  const [ytModalOpen, setYtModalOpen] = useState(false);
  const [ytBusy, setYtBusy] = useState(false);

  const loadAccounts = useCallback(async () => {
    const b = getBridge();
    if (!b || typeof b.listAccounts !== "function") return;
    try {
      const raw = await b.listAccounts();
      const parsed = raw ? JSON.parse(raw) : [];
      setAccounts(Array.isArray(parsed) ? parsed : []);
    } catch (e) {
      console.error("[Melyar] listAccounts failed", e);
    }
  }, []);

  // Load sources + accounts when the view opens.
  useEffect(() => {
    void load();
    void loadAccounts();
  }, [load, loadAccounts]);

  // Listen to import events for account connect/import updates.
  useEffect(() => {
    const b = getBridge();
    if (!b || typeof b.importEvent?.connect !== "function") return;
    const handler = (json: string) => {
      try {
        const ev = JSON.parse(json);
        if (ev.type === "provider_connected" || ev.type === "provider_disconnected") {
          void loadAccounts();
        }
        if (ev.type === "account_auth_url" && ev.provider === "google") {
          // open browser
          b.openExternalUrl?.(ev.url);
        }
      } catch {}
    };
    b.importEvent.connect(handler);
    return () => {
      try { b.importEvent.disconnect(handler); } catch {}
    };
  }, [loadAccounts]);

  return (
    <div className="px-6 pb-10 pt-6 lg:px-9">
      <header className="mb-6 flex items-center gap-3">
        <button
          className="btn-glass icon-btn text-text-tertiary hover:text-text"
          aria-label={t("common.back")}
          onClick={() => navigate({ name: "home" })}
        >
          <ChevronLeft className="size-5" />
        </button>
        <Plug className="size-5 text-text-tertiary" />
        <h1 className="text-2xl font-bold tracking-tight text-text">{t("nav.plugins")}</h1>
      </header>

      <p className="text-[12.5px] text-text-tertiary mb-6 max-w-2xl">{t("plugins.intro")}</p>

      {/* Section 1: music sources */}
      <section className="mb-8">
        <h2 className="text-[15px] font-bold tracking-tight text-text mb-3">
          {t("plugins.section.sources")}
        </h2>
        {loading && sources.length === 0 ? (
          <p className="text-[12.5px] text-text-tertiary py-6">{t("plugins.loading")}</p>
        ) : sources.length === 0 ? (
          <p className="text-[12.5px] text-text-tertiary py-6">{t("plugins.empty")}</p>
        ) : (
          <div className="rounded-xl border border-line/40 bg-panel/20 px-4">
            {sources.map((s) => (
              <SourceRow key={s.id} source={s} />
            ))}
          </div>
        )}
      </section>

      {/* Section 2: accounts & API */}
      <section>
        <h2 className="text-[15px] font-bold tracking-tight text-text mb-3">
          {t("plugins.section.accounts")}
        </h2>
        <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
          {accounts.length === 0 ? (
            <p className="text-[12.5px] text-text-tertiary col-span-full py-4">
              {t("plugins.loading")}
            </p>
          ) : (
            accounts.map((acc) => {
              const isGoogle = acc.id === "google";
              const icon = isGoogle ? (
                <DoorOpen className="size-5" />
              ) : acc.id === "spotify" ? (
                <span className="text-lg font-bold">♪</span>
              ) : acc.id === "yandex" ? (
                <span className="text-lg font-bold">Y</span>
              ) : acc.id === "apple" ? (
                <span className="text-lg font-bold">A</span>
              ) : acc.id === "custom" ? (
                <span className="text-lg">⚙</span>
              ) : (
                <span className="text-lg font-bold">G</span>
              );
              return (
                <AccountCard
                  key={acc.id}
                  icon={icon}
                  name={acc.name}
                  description={acc.configHint || ""}
                  connected={Boolean(acc.connected)}
                  onConnect={() => {
                    if (isGoogle) {
                      setYtModalOpen(true);
                    } else {
                      const b = getBridge();
                      if (!b || typeof b.connectAccount !== "function") return;
                      b.connectAccount(acc.id);
                    }
                  }}
                  onImport={() => {
                    const b = getBridge();
                    if (!b || typeof b.importAccount !== "function") return;
                    b.importAccount(acc.id, "{}");
                  }}
                />
              );
            })
          )}
        </div>
      </section>

      {/* YouTube door modal (big) */}
      {ytModalOpen && (
        <div
          className="settings-overlay fixed inset-0 z-[var(--z-modal-panel)] flex items-center justify-center p-6"
          onClick={(e) => {
            if (e.target === e.currentTarget) setYtModalOpen(false);
          }}
        >
          <div
            role="dialog"
            aria-modal="true"
            aria-label="YouTube"
            className="settings-panel w-full max-w-2xl max-h-[85vh] rounded-2xl shadow-2xl flex flex-col overflow-hidden p-6 gap-4"
          >
            <div className="flex items-center gap-3">
              <DoorOpen className="size-6 text-accent-text" />
              <h2 className="text-xl font-bold tracking-tight text-text">YouTube</h2>
            </div>
            <p className="text-[13px] text-text-tertiary leading-snug">
              {t("plugins.desc.youtube")}
            </p>
            {ytBusy ? (
              <p className="text-[12.5px] text-text-tertiary">{t("plugins.loading")}</p>
            ) : (
              <button
                className="btn-glass import-primary-chip self-start"
                onClick={() => {
                  const b = getBridge();
                  if (!b || typeof b.connectAccount !== "function") return;
                  setYtBusy(true);
                  b.connectAccount("google");
                  // reset busy after a short delay; real completion comes via event
                  setTimeout(() => setYtBusy(false), 1500);
                }}
              >
                {t("plugins.connect")}
              </button>
            )}
            <button
              className="btn-glass chip text-xs self-start"
              onClick={() => setYtModalOpen(false)}
            >
              {t("common.close") || "Close"}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}