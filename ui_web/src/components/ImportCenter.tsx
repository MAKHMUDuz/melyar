import React, { useEffect } from "react";
import {
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  FileJson,
  FolderDown,
  ListMusic,
  Loader2,
  Music4,
  RefreshCw,
  ShieldCheck,
  Sparkles,
  Upload,
  X,
} from "lucide-react";
import { cn } from "@/lib/cn";
import { t, tCount, useLocale } from "@/lib/i18n";
import { useImportCenter } from "@/lib/importCenter";
import { useUIStore } from "@/store";
import { formatTime } from "@/lib/lyrics";

function sourceName(source: string) {
  if (source === "local") return t("import.source.local");
  if (source === "spotify") return t("import.source.spotify");
  if (source === "apple") return t("import.source.apple");
  if (source === "file") return t("import.source.file");
  return source;
}

function StatusBadge({ status, label }: { status: string; label?: string }) {
  const tone =
    status === "connected" || status === "ready"
      ? "ok"
      : status === "requires_configuration"
        ? "warn"
        : status === "export_only"
          ? "info"
          : "muted";
  return (
    <span className="import-badge" data-tone={tone}>
      <span className="import-badge-dot" />
      {label ?? status}
    </span>
  );
}

function SourceCard({
  source,
  status,
  onPrimary,
}: {
  source: string;
  status: string;
  onPrimary: () => void;
}) {
  const primaryLabel =
    source === "local"
      ? t("import.local.addFolder")
      : source === "spotify"
        ? t("import.spotify.connect")
        : source === "apple"
          ? t("import.apple.chooseXml")
          : t("import.file.choose");

  return (
    <article className="import-card" data-source={source} data-status={status}>
      <header className="import-card-head">
        <div className="import-card-icon">
          {source === "local" ? (
            <FolderDown className="size-[21px]" />
          ) : source === "spotify"
            ? <Music4 className="size-[21px]" />
            : source === "apple"
              ? <Music4 className="size-[21px]" />
              : <Upload className="size-[21px]" />
          }</div>
        <div className="min-w-0 flex-1">
          <h3 className="text-[15px] font-semibold tracking-tight text-text">
            {sourceName(source)}
          </h3>
          <StatusBadge status={status} label={status} />
        </div>
      </header>
      <p className="import-card-desc">
        {source === "local"
          ? t("import.local.desc")
          : source === "spotify"
            ? t("import.spotify.desc")
            : source === "apple"
              ? t("import.apple.desc")
              : t("import.file.desc")}
      </p>
      <footer className="import-card-footer">
        <button
          className="btn-glass chip import-primary-chip inline-flex items-center gap-1.5"
          onClick={onPrimary}
        >
          {primaryLabel}
        </button>
      </footer>
    </article>
  );
}

function StatsRow() {
  const snapshot = useImportCenter((s) => s.snapshot);
  const stats = snapshot?.stats;
  if (!stats) return null;
  return (
    <div className="import-stats">
      <div className="import-stat">{stats.items?.toLocaleString() ?? "0"}</div>
      <span>{t("import.stats.items")}</span>
      <div className="import-stat">{stats.playable?.toLocaleString() ?? "0"}</div>
      <span>{t("import.stats.playable")}</span>
      <div className="import-stat">{stats.metadataOnly?.toLocaleString() ?? "0"}</div>
      <span>{t("import.stats.metadataOnly")}</span>
    </div>
  );
}

function ImportCenter() {
  useLocale();
  const snapshot = useImportCenter((s) => s.snapshot);
  const refresh = useImportCenter((s) => s.refresh);
  const folders = snapshot?.folders ?? [];
  const spotify = snapshot?.providers?.spotify;

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <div className="import-center px-6 pb-8 pt-6 lg:px-9">
      <header className="import-hero">
        <div className="import-hero-copy">
          <p className="import-kicker">
            <Sparkles className="size-3.5" />
            {t("import.kicker")}
          </p>
          <h1 className="import-hero-title">{t("import.title")}</h1>
          <p className="import-hero-subtitle">{t("import.subtitle")}</p>
        </div>
        <div className="import-hero-stats">{StatsRow()}</div>
      </header>

      <section className="import-section">
        <div className="import-section-head">
          <p className="import-section-kicker">{t("import.sources.kicker")}</p>
          <h2 className="import-section-title">{t("import.sources.title")}</h2>
        </div>
        <div className="import-source-grid">
          <SourceCard
            source="local"
            status={folders.length > 0 ? "ready" : "needs_attention"}
            onPrimary={() => useImportCenter.getState().openFolderPicker()}
          />
          <SourceCard
            source="spotify"
            status={spotify?.status ?? "requires_configuration"}
            onPrimary={() => {
              const spotify = useImportCenter.getState().snapshot?.providers?.spotify;
              if (spotify?.connected) {
                useImportCenter.getState().previewSpotify();
              } else {
                useImportCenter.getState().connectSpotify();
              }
            }}
          />
          <SourceCard
            source="apple"
            status="export_only"
            onPrimary={() => useImportCenter.getState().openExportFilePicker()}
          />
          <SourceCard
            source="file"
            status="ready"
            onPrimary={() => useImportCenter.getState().openExportFilePicker()}
          />
        </div>
      </section>

      {folders.length > 0 && (
        <div className="import-folders">
          <h3 className="import-folders-kicker">{t("import.folders.kicker")}</h3>
          <p className="import-folders-hint">{t("import.folders.hint")}</p>
          {folders.map((folder) => (
            <div key={folder.path} className="import-folder-row">
              <div className="import-folder-main">
                <p className="truncate text-[13px] font-medium" title={folder.path}>
                  {folder.path}
                </p>
                <p className="text-[11.5px] text-text-tertiary">
                  {tCount("import.folder.tracks", folder.tracks)} tracks ·
                  {tCount("import.folder.albums", folder.albums)} albums
                </p>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

export { ImportCenter };