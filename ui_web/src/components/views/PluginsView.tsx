import React from "react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Plug, ChevronLeft, DoorOpen, Settings2, LogOut, X } from "lucide-react";
import { cn } from "@/lib/cn";
import { t, useLocale } from "@/lib/i18n";
import { GlassToggle } from "@/components/GlassComponents";
import { usePlugins, type SourceInfo } from "@/lib/plugins";
import { useUIStore } from "@/store";
import { useImportCenter } from "@/lib/importCenter";
import { getBridge } from "@/lib/client";

type Account = {
  id: string;
  name: string;
  connected: boolean;
  user: string;
  requiresConfig: boolean;
  configHint: string;
};

type ConfigField = { key: string; setting: string; label: string; secret?: boolean; placeholder?: string };

const CONFIG_FIELDS: Record<string, ConfigField[]> = {
  google: [
    { key: "clientId", setting: "google.clientId", label: "Client ID", placeholder: "123-abc.apps.googleusercontent.com" },
    { key: "clientSecret", setting: "google.clientSecret", label: "Client secret (opsiyonel)", secret: true, placeholder: "GOCSPX-..." },
  ],
  spotify: [
    { key: "clientId", setting: "spotify.clientId", label: "Client ID", placeholder: "Spotify app client id" },
  ],
  yandex: [
    { key: "clientId", setting: "yandex.clientId", label: "Client ID", placeholder: "Yandex OAuth client id" },
  ],
  apple: [],
  custom: [
    { key: "baseUrl", setting: "custom.baseUrl", label: "API Base URL", placeholder: "https://music.example.com" },
    { key: "user", setting: "custom.user", label: "Kullanıcı adı", placeholder: "admin" },
    { key: "password", setting: "custom.password", label: "Şifre", secret: true, placeholder: "" },
    { key: "token", setting: "custom.token", label: "Token (opsiyonel)", secret: true, placeholder: "" },
  ],
};

const REDIRECT_HINT: Record<string, string> = {
  google: "Google Cloud Console → OAuth client (Desktop app) → Authorized redirect URIs: http://127.0.0.1:43822/callback",
  spotify: "Spotify Dashboard → Redirect URIs: http://127.0.0.1:43821/callback",
  yandex: "Yandex OAuth → Redirect URI: http://127.0.0.1:43823/callback",
};

/** One source, as a row. */
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

function ProviderIcon({ id }: { id: string }) {
  if (id === "google") return <DoorOpen className="size-5" />;
  if (id === "spotify") return <span className="text-lg font-bold">♪</span>;
  if (id === "yandex") return <span className="text-lg font-bold">Y</span>;
  if (id === "apple") return <span className="text-lg font-bold">A</span>;
  return <span className="text-lg">⚙</span>;
}

function AccountCard({
  acc,
  statusText,
  onConfigure,
  onConnect,
  onDisconnect,
  onImport,
  busy,
}: {
  acc: Account;
  statusText?: string;
  onConfigure: () => void;
  onConnect: () => void;
  onDisconnect: () => void;
  onImport: () => void;
  busy: boolean;
}) {
  return (
    <div className="rounded-xl border border-line/40 bg-panel/40 p-4 flex flex-col gap-3">
      <div className="flex items-center gap-3">
        <div className="size-10 rounded-lg bg-accent/10 grid place-items-center text-accent-text shrink-0">
          <ProviderIcon id={acc.id} />
        </div>
        <div className="min-w-0 flex-1">
          <p className="text-[13.5px] font-medium text-text truncate">{acc.name}</p>
          <p className="text-[11px] text-text-tertiary truncate">
            {acc.connected && acc.user ? acc.user : (acc.configHint || "")}
          </p>
        </div>
        <button
          className="btn-glass icon-btn text-text-tertiary hover:text-text shrink-0"
          aria-label="Ayarlar"
          onClick={onConfigure}
          title="Yapılandır"
        >
          <Settings2 className="size-4" />
        </button>
      </div>
      <div className="flex gap-2 flex-wrap">
        {acc.connected ? (
          <>
            <button className="btn-glass chip text-xs" onClick={onImport} disabled={busy}>
              {t("import.action") || "İçe aktar"}
            </button>
            <button className="btn-glass chip text-xs" onClick={onDisconnect} disabled={busy}>
              <LogOut className="size-3.5 inline mr-1" />
              Çıkış yap
            </button>
          </>
        ) : (
          <button
            className="btn-glass chip text-xs"
            onClick={onConnect}
            disabled={busy || (CONFIG_FIELDS[acc.id]?.length ? false : false)}
          >
            {t("plugins.connect")}
          </button>
        )}
      </div>
      {statusText ? (
        <p className="text-[11px] text-text-tertiary leading-snug">{statusText}</p>
      ) : null}
    </div>
  );
}

export function PluginsView() {
  useLocale();
  const navigate = useUIStore((s) => s.navigate);
  const importRefresh = useImportCenter((s) => s.refresh);
  const sources = usePlugins((s) => s.sources);
  const loading = usePlugins((s) => s.loading);
  const load = usePlugins((s) => s.load);

  const [accounts, setAccounts] = useState<Account[]>([
    { id: "spotify", name: "Spotify", connected: false, user: "", requiresConfig: true, configHint: "" },
    { id: "google", name: "Google", connected: false, user: "", requiresConfig: true, configHint: "" },
    { id: "yandex", name: "Yandex Music", connected: false, user: "", requiresConfig: true, configHint: "" },
    { id: "apple", name: "Apple Music", connected: false, user: "", requiresConfig: true, configHint: "" },
    { id: "custom", name: "Custom API", connected: false, user: "", requiresConfig: true, configHint: "" },
  ]);
  const [ytModalOpen, setYtModalOpen] = useState(false);
  const [ytBusy, setYtBusy] = useState(false);

  // Config modal state
  const [configProvider, setConfigProvider] = useState<string | null>(null);
  const [configValues, setConfigValues] = useState<Record<string, string>>({});
  const [configSaving, setConfigSaving] = useState(false);
  const [configError, setConfigError] = useState<string | null>(null);

  // Status/progress
  const [statusLine, setStatusLine] = useState<string | null>(null);
  const [progressLine, setProgressLine] = useState<string | null>(null);
  const [busyProvider, setBusyProvider] = useState<string | null>(null);

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

  const loadSettings = useCallback(() => {
    const b = getBridge();
    if (!b || typeof b.loadSettings !== "function") return {} as Record<string, string>;
    try {
      return JSON.parse(b.loadSettings() || "{}");
    } catch {
      return {} as Record<string, string>;
    }
  }, []);

  const openConfig = useCallback((providerId: string) => {
    const s = loadSettings();
    const fields = CONFIG_FIELDS[providerId] || [];
    const vals: Record<string, string> = {};
    for (const f of fields) vals[f.key] = (s as any)[f.setting] || "";
    setConfigValues(vals);
    setConfigError(null);
    setConfigProvider(providerId);
  }, [loadSettings]);

  const saveConfig = useCallback(async () => {
    const b = getBridge();
    if (!b || !configProvider) return;
    const fields = CONFIG_FIELDS[configProvider] || [];
    setConfigSaving(true);
    setConfigError(null);
    try {
      for (const f of fields) {
        const v = (configValues[f.key] || "").trim();
        if (v) {
          if (typeof b.saveSetting !== "function" || !b.saveSetting(f.setting, v)) {
            throw new Error(`Kaydedilemedi: ${f.setting}`);
          }
        } else {
          if (typeof b.removeSetting === "function") b.removeSetting(f.setting);
        }
      }
      setConfigProvider(null);
      await loadAccounts();
    } catch (e: any) {
      setConfigError(String(e?.message || e));
    } finally {
      setConfigSaving(false);
    }
  }, [configProvider, configValues, loadAccounts]);

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
          if (ev.type === "provider_connected" && ev.provider) {
            const who = ev.account ? ` (${ev.account})` : "";
            setStatusLine(`${ev.provider}${who} bağlandı — kütüphane içe aktarılıyor…`);
            // auto-import library after successful connect
            const bb = getBridge();
            if (bb && typeof bb.importAccount === "function") {
              bb.importAccount(ev.provider, "{}");
            } else if (ev.provider === "spotify" && bb && typeof bb.importSpotifyPreview === "function") {
              bb.importSpotifyPreview(JSON.stringify({ autoCommit: true, tracks: true, albums: true, playlists: true }));
            }
          }
        }
        if ((ev.type === "account_auth_url" || ev.type === "spotify_auth_url") && ev.url) {
          if (typeof b.openExternalUrl === "function") b.openExternalUrl(ev.url);
          else window.open(ev.url, "_blank");
        }
        if (ev.type === "error") {
          const msg = ev.message || ev.code || "Hata";
          setStatusLine(`Hata (${ev.code || "?"}): ${msg}`);
          setBusyProvider(null);
          if (ev.code === "requires_configuration") {
            // prompt config for the provider if known
            if (ev.provider) openConfig(ev.provider);
          }
        }
        if (ev.type === "progress") {
          const detail = ev.detail ? JSON.stringify(ev.detail) : "";
          setProgressLine(`${ev.phase || "aktarım"} ${ev.current || 0}/${ev.total || "?"} ${detail}`);
        }
        if (ev.type === "done" || ev.type === "complete") {
          setProgressLine(null);
          setStatusLine(`${ev.phase || ev.provider || "aktarım"} tamamlandı.`);
          setBusyProvider(null);
          void importRefresh();
          void loadAccounts();
        }
      } catch {}
    };
    b.importEvent.connect(handler);
    return () => {
      try { b.importEvent.disconnect(handler); } catch {}
    };
  }, [loadAccounts, openConfig, importRefresh]);

  const connectProvider = useCallback((id: string) => {
    const b = getBridge();
    if (!b || typeof b.connectAccount !== "function") return;
    // If config missing, open config modal first.
    const acc = accounts.find((a) => a.id === id);
    if (acc?.requiresConfig) {
      setStatusLine(`${acc.name}: önce ⚙ Yapılandır ile Client ID girin.`);
      openConfig(id);
      return;
    }
    setBusyProvider(id);
    setStatusLine(null);
    if (id === "google") setYtModalOpen(true);
    else b.connectAccount(id);
  }, [accounts, openConfig]);

  const disconnectProvider = useCallback((id: string) => {
    const b = getBridge();
    if (!b || typeof b.disconnectAccount !== "function") return;
    b.disconnectAccount(id);
  }, []);

  const importProvider = useCallback((id: string) => {
    const b = getBridge();
    if (!b || typeof b.importAccount !== "function") return;
    setBusyProvider(id);
    b.importAccount(id, "{}");
  }, []);

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

      {statusLine ? (
        <div className="mb-4 rounded-lg border border-line/40 bg-panel/40 px-3 py-2 text-[12px] text-text-tertiary">
          {statusLine}
        </div>
      ) : null}
      {progressLine ? (
        <div className="mb-4 rounded-lg border border-line/40 bg-panel/40 px-3 py-2 text-[12px] text-text-tertiary">
          {progressLine}
        </div>
      ) : null}

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
            accounts.map((acc) => (
              <AccountCard
                key={acc.id}
                acc={acc}
                statusText={busyProvider === acc.id ? "İşleniyor…" : undefined}
                onConfigure={() => openConfig(acc.id)}
                onConnect={() => connectProvider(acc.id)}
                onDisconnect={() => disconnectProvider(acc.id)}
                onImport={() => importProvider(acc.id)}
                busy={busyProvider === acc.id}
              />
            ))
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
            {accounts.find((a) => a.id === "google")?.requiresConfig ? (
              <div className="rounded-lg border border-line/40 bg-panel/40 p-3 text-[12px] text-text-tertiary">
                Önce "Yapılandır" (⚙) ile Google Client ID girin.
                <br />
                <span className="text-[10px]">{REDIRECT_HINT.google}</span>
              </div>
            ) : ytBusy ? (
              <p className="text-[12.5px] text-text-tertiary">{t("plugins.loading")}</p>
            ) : (
              <button
                className="btn-glass import-primary-chip self-start"
                onClick={() => {
                  const b = getBridge();
                  if (!b || typeof b.connectAccount !== "function") return;
                  setYtBusy(true);
                  b.connectAccount("google");
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

      {/* Config modal */}
      {configProvider && (
        <div
          className="settings-overlay fixed inset-0 z-[var(--z-modal-panel)] flex items-center justify-center p-6"
          onClick={(e) => {
            if (e.target === e.currentTarget) setConfigProvider(null);
          }}
        >
          <div
            role="dialog"
            aria-modal="true"
            className="settings-panel w-full max-w-lg max-h-[85vh] rounded-2xl shadow-2xl flex flex-col overflow-hidden p-6 gap-4"
          >
            <div className="flex items-center justify-between">
              <h2 className="text-xl font-bold tracking-tight text-text">
                {accounts.find((a) => a.id === configProvider)?.name || configProvider} — Yapılandır
              </h2>
              <button className="btn-glass icon-btn" onClick={() => setConfigProvider(null)}>
                <X className="size-4" />
              </button>
            </div>
            <div className="rounded-lg border border-line/40 bg-panel/40 p-3 text-[12px] text-text-tertiary leading-snug space-y-1">
              <p className="font-medium text-text">Nasıl alınır?</p>
              {configProvider === "google" && (
                <>
                  <p>1. console.cloud.google.com → APIs & Services → Credentials → Create OAuth client (Desktop app).</p>
                  <p>2. YouTube Data API v3'ü etkinleştirin.</p>
                  <p>3. Client ID'yi (ve varsa Secret'ı) aşağıya yapıştırın.</p>
                </>
              )}
              {configProvider === "spotify" && (
                <>
                  <p>1. developer.spotify.com/dashboard → Create app.</p>
                  <p>2. Settings → Redirect URIs: <code className="text-text">http://127.0.0.1:43821/callback</code></p>
                  <p>3. Client ID'yi aşağıya yapıştırın.</p>
                </>
              )}
              {configProvider === "yandex" && (
                <p>Yandex OAuth client ID girin (deneysel; resmi API kapalı).</p>
              )}
              {configProvider === "custom" && (
                <p>Subsonic/Navidrome sunucunuzun adresi ve kullanıcı bilgilerini girin.</p>
              )}
              {REDIRECT_HINT[configProvider] ? <p className="text-[10px] mt-1">{REDIRECT_HINT[configProvider]}</p> : null}
            </div>
            {(CONFIG_FIELDS[configProvider] || []).map((f) => (
              <label key={f.key} className="flex flex-col gap-1">
                <span className="text-[12px] font-medium text-text">{f.label}</span>
                <input
                  type={f.secret ? "password" : "text"}
                  className="input-glass"
                  value={configValues[f.key] || ""}
                  placeholder={f.placeholder}
                  onChange={(e) =>
                    setConfigValues((v) => ({ ...v, [f.key]: e.target.value }))
                  }
                />
              </label>
            ))}
            {configError ? (
              <p className="text-[12px] text-red-400">{configError}</p>
            ) : null}
            <div className="flex gap-2 justify-end">
              <button className="btn-glass chip text-xs" onClick={() => setConfigProvider(null)}>
                {t("common.close") || "İptal"}
              </button>
              <button
                className="btn-glass chip text-xs"
                onClick={() => void saveConfig()}
                disabled={configSaving}
              >
                {configSaving ? t("plugins.loading") : "Kaydet"}
              </button>
              <button
                className="btn-glass import-primary-chip text-xs"
                onClick={async () => {
                  await saveConfig();
                  const id = configProvider;
                  if (id) {
                    setConfigProvider(null);
                    if (id === "google") setYtModalOpen(true);
                    else {
                      const b = getBridge();
                      b?.connectAccount?.(id);
                    }
                  }
                }}
                disabled={configSaving}
              >
                Kaydet ve bağlan
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}