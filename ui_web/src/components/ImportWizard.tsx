import React, { useEffect, useState } from "react";
import {
  AlertTriangle,
  ArrowRight,
  CheckCircle2,
  ChevronLeft,
  ChevronRight,
  X,
  FileJson,
  FolderDown,
  Loader2,
  Music4,
  Search,
  ShieldCheck,
  Sparkles,
  Upload,
} from "lucide-react";
import { cn } from "@/lib/cn";
import { useImportCenter } from "@/lib/importCenter";
import { useUIStore } from "@/store";
import { useLocale } from "@/lib/i18n";
import { t } from "@/lib/i18n";
import { formatTime } from "@/lib/lyrics";
import { TrackTable } from "@/components/TrackTable";

type ImportStep = 1 | 2 | 3 | 4 | 5 | 6 | 7;

function ProgressBar({ phase, current, total }: { phase: string; current: number; total: number }) {
  const percent = total > 0 ? Math.round((current / total) * 100) : 0;
  return (
    <div className="import-progress-bar">
      <span className="import-progress-text">{t("import.progress", { total, current })}</span>
      <div className="import-progress-bar-fill" style={{ width: `${percent}%` }} />
    </div>
  );
}

function Step1OpenFile({ openWizard, setStep, setPreview }) {
  return (
    <div className="import-wizard-step">
      <h3>{t("import.wizard.step1.title")}</h3>
      <p>{t("import.wizard.step1.description")}</p>
      <button
        className="btn-glass import-primary-chip mt-4"
        onClick={() => {
          useImportCenter.getState().openExportFilePicker();
        }}
      >
        {t("import.file.choose")}
      </button>
    </div>
  );
}

function Step1OpenFolder({ openFolderPicker, setStep, setPreview }) {
  return (
    <div className="import-wizard-step">
      <h3>{t("import.wizard.step1.title")}</h3>
      <p>{t("import.wizard.folderDescription")}</p>
      <button
        className="btn-glass import-primary-chip mt-4"
        onClick={() => useImportCenter.getState().openFolderPicker()}
      >
        {t("import.local.addFolder")}
      </button>
    </div>
  );
}

function Step2Preview({ preview, setStep, toggleItem, selectAll, clearSelection }) {
  if (!preview) {
    return (
      <div className="import-wizard-step">
        <h3>{t("import.wizard.step2.title")}</h3>
        <p>{t("import.wizard.step2.noPreview")}</p>
      </div>
    );
  }

  const playableItems = preview.items.filter((item) => item.row.is_playable);
  const metadataOnlyItems = preview.items.filter(
    (item) => !item.row.is_playable && item.availability === "metadata_only"
  );
  const unplayableItems = preview.items.filter(
    (item) => !item.row.is_playable && item.availability !== "metadata_only"
  );

  return (
    <div className="import-wizard-step">
      <h3>{t("import.wizard.step2.title")}</h3>

      {/* Summary stats */}
      <div className="import-wizard-step-summary">
        <div className="import-wizard-summary-row">
          <span>{t("import.wizard.total", { count: preview.items.length })}</span>
          <span>{t("import.wizard.playable", { count: playableItems.length })}</span>
          <span>{t("import.wizard.metadataOnly", { count: metadataOnlyItems.length })}</span>
          <span>{t("import.wizard.unplayable", { count: unplayableItems.length })}</span>
        </div>
        <div className="import-wizard-summary-row">
          <span>{t("import.wizard.playlists", { count: preview.playlists.length })}</span>
        </div>
      </div>

      {/* Tabs for tracks and playlists */}
      <div className="import-wizard-step-tabs">
        <button
          className={cn("import-wizard-tab", "import-wizard-tab-active")}
          onClick={() => {}}
        >
          {t("import.wizard.tracks")}
        </button>
        <button className="import-wizard-tab" onClick={() => {}}>
          {t("import.wizard.playlists")}
        </button>
      </div>

      {/* Tracks table */}
      <div className="import-wizard-step-content">
        <TrackTable
          tracks={preview.items}
          showAlbum={true}
          emptyText={t("import.wizard.noTracks")}
        />
      </div>
    </div>
  );
}

function Step3Review({ preview, selected, toggleItem, setStep, commit, cancelWizard }) {
  if (!preview) {
    return (
      <div className="import-wizard-step">
        <h3>{t("import.wizard.step3.title")}</h3>
        <p>{t("import.wizard.step3.noPreview")}</p>
      </div>
    );
  }

  const selectedItems = preview.items.filter((item) => selected.includes(item.itemId));
  const playableSelected = selectedItems.filter((item) => item.row.is_playable);
  const metadataSelected = selectedItems.filter(
    (item) => !item.row.is_playable && item.availability === "metadata_only"
  );

  return (
    <div className="import-wizard-step">
      <h3>{t("import.wizard.step3.title")}</h3>

      {/* Summary */}
      <div className="import-wizard-step-summary">
        <div className="import-wizard-summary-row">
          <span>{t("import.wizard.selected", { count: selected.length })}</span>
          <span>{t("import.wizard.playableSelected", { count: playableSelected.length })}</span>
          <span>{t("import.wizard.metadataSelected", { count: metadataSelected.length })}</span>
        </div>
      </div>

      {/* Review table */}
      <div className="import-wizard-step-content">
        <TrackTable
          tracks={preview.items}
          showAlbum={true}
          emptyText={t("import.wizard.noTracks")}
        />
      </div>

      {/* Action buttons */}
      <div className="import-wizard-step-actions">
        <button
          className="btn-glass"
          onClick={cancelWizard}
        >
          {t("import.cancel")}
        </button>
        <button
          className="btn-glass import-primary-chip"
          onClick={commit}
          disabled={selected.length === 0}
        >
          {t("import.wizard.import")}
        </button>
      </div>
    </div>
  );
}

function Step4Complete({ completion, step, resetWizard, loadLibrary }) {
  if (!completion) {
    return null;
  }

  const { imported, skipped, failed, total, playlists } = completion;

  return (
    <div className="import-wizard-step">
      <h3>{t("import.wizard.step4.title")}</h3>

      {/* Completion summary */}
      <div className="import-wizard-completion-summary">
        <div className="import-completion-stats">
          <div className="import-completion-stat">
            <span className="import-completion-number">{imported}</span>
            <span>{t("import.imported")}</span>
          </div>
          <div className="import-completion-stat">
            <span className="import-completion-number">{skipped}</span>
            <span>{t("import.skipped")}</span>
          </div>
          <div className="import-completion-stat">
            <span className="import-completion-number">{failed}</span>
            <span>{t("import.failed")}</span>
          </div>
        </div>
        {playlists > 0 && (
          <span>{t("import.wizard.importedPlaylists", { count: playlists })}</span>
        )}
      </div>

      {/* Action buttons */}
      <div className="import-wizard-step-actions">
        <button
          className="btn-glass"
          onClick={resetWizard}
        >
          {t("import.wizard.done")}
        </button>
        <button
          className="btn-glass import-primary-chip"
          onClick={loadLibrary}
        >
          {t("import.viewLibrary")}
        </button>
      </div>
    </div>
  );
}

function ImportWizard() {
  useLocale();
  const {
    wizardOpen,
    step,
    preview,
    selected,
    completion,
    running,
    error,
    progress,
    openWizard,
    openFolderPicker,
    openExportFilePicker,
    startFolderScan,
    handleDirectoryPicked,
    handleFilePicked,
    setStep,
    toggleItem,
    selectAll,
    clearSelection,
    pickCandidate,
    commit,
    cancel,
    closeWizard,
    loadLibrary,
    changeFolder,
    rescanFolder,
    removeFolder,
    setAutoScan,
    setSpotifyOption,
    spotifyConfigOpen,
    spotifyBusy,
    spotifyAuthPending,
    pendingManagedCopy,
    copyTarget,
    changingFolder,
  } = useImportCenter();

  // Handle directory picker result
  useEffect(() => {
    // This is handled by the bridge connect
  }, [pendingManagedCopy, copyTarget, changingFolder]);

  // Handle file picker result
  useEffect(() => {
    // This is handled by the bridge connect
  }, [handleFilePicked]);

  if (!wizardOpen && step === 1) {
    return null;
  }

  return (
    <div className="import-wizard">
      <div className="import-wizard-header">
        <h2>{t(`import.wizard.step${step}.title`)}</h2>
        <button
          className="import-wizard-close"
          onClick={closeWizard}
          aria-label={t("import.cancel")}
        >
          <X className="size-4" />
        </button>
      </div>

      <div className="import-wizard-steps">
        <div className="import-wizard-step-nav">
          <span className="import-wizard-step-number">1</span>
          <span className="import-wizard-step-label">{t("import.wizard.step1")}</span>
        </div>
        <div className="import-wizard-step-nav">
          <span className="import-wizard-step-number">2</span>
          <span className="import-wizard-step-label">{t("import.wizard.step2")}</span>
        </div>
        <div className="import-wizard-step-nav">
          <span className="import-wizard-step-number">3</span>
          <span className="import-wizard-step-label">{t("import.wizard.step3")}</span>
        </div>
        <div className="import-wizard-step-nav">
          <span className="import-wizard-step-number">4</span>
          <span className="import-wizard-step-label">{t("import.wizard.step4")}</span>
        </div>
        <div className="import-wizard-step-nav">
          <span className="import-wizard-step-number">5</span>
          <span className="import-wizard-step-label">{t("import.wizard.step5")}</span>
        </div>
        <div className="import-wizard-step-nav">
          <span className="import-wizard-step-number">6</span>
          <span className="import-wizard-step-label">{t("import.wizard.step6")}</span>
        </div>
        <div className="import-wizard-step-nav">
          <span className="import-wizard-step-number">7</span>
          <span className="import-wizard-step-label">{t("import.wizard.step7")}</span>
        </div>
      </div>

      <div className="import-wizard-content">
        {step === 1 && <Step1OpenFile openWizard={openWizard} setStep={setStep} setPreview={() => {}} />}
        {step === 1 && <Step1OpenFolder openFolderPicker={openFolderPicker} setStep={setStep} setPreview={() => {}} />}
        {step === 2 && <Step2Preview preview={preview} setStep={setStep} toggleItem={toggleItem} selectAll={selectAll} clearSelection={clearSelection} />}
        {step === 3 && <Step3Review preview={preview} selected={selected} toggleItem={toggleItem} setStep={setStep} commit={commit} cancelWizard={cancel} />}
        {step === 4 && <Step4Complete completion={completion} step={step} resetWizard={closeWizard} loadLibrary={loadLibrary} />}
      </div>
    </div>
  );
}

export { ImportWizard };