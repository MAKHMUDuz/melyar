import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { X, RotateCcw, ChevronRight, ChevronLeft, Search, Settings as SettingsIcon, Palette, Music, Headphones, Database, Mic2, Plug, Keyboard, Shield, Info, Zap, Music2, SlidersHorizontal, RefreshCw, Disc2, User, Mic, TreePine } from "lucide-react";
import { cn } from "@/lib/cn";
import { t, useLocale, LOCALES } from "@/lib/i18n";
import { useSettings, type Settings, DEFAULT_SETTINGS } from "@/lib/settings";
import { GlassSurface, GlassToggle, GlassSegmented, GlassSlider, GlassButton, GlassMorphIndicator, GlassIconButton } from "@/components/GlassComponents";
import { prefersReducedMotion, getMotionTokens } from "@/lib/motion";

/* ============================================================
   CATEGORY DEFINITIONS
   ============================================================ */

interface SettingsCategory {
  id: string;
  label: string;
  description: string;
  icon: React.ReactNode;
  // Only include categories that have implemented settings
  isImplemented: boolean;
}

const CATEGORIES: SettingsCategory[] = [
  {
    id: "general",
    label: t("settings.category.general"),
    description: t("settings.category.general.description"),
    icon: <SettingsIcon className="size-5" />,
    isImplemented: true,
  },
  {
    id: "appearance",
    label: t("settings.category.appearance"),
    description: t("settings.category.appearance.description"),
    icon: <Palette className="size-5" />,
    isImplemented: true,
  },
  {
    id: "playback",
    label: t("settings.category.playback"),
    description: t("settings.category.playback.description"),
    icon: <Music className="size-5" />,
    isImplemented: true,
  },
  {
    id: "audio",
    label: t("settings.category.audio"),
    description: t("settings.category.audio.description"),
    icon: <Headphones className="size-5" />,
    isImplemented: true,
  },
  {
    id: "library",
    label: t("settings.category.library"),
    description: t("settings.category.library.description"),
    icon: <Database className="size-5" />,
    isImplemented: true,
  },
  {
    id: "lyrics",
    label: t("settings.category.lyrics"),
    description: t("settings.category.lyrics.description"),
    icon: <Mic2 className="size-5" />,
    isImplemented: true,
  },
  {
    id: "providers",
    label: t("settings.category.providers"),
    description: t("settings.category.providers.description"),
    icon: <Plug className="size-5" />,
    isImplemented: true,
  },
  {
    id: "keyboard",
    label: t("settings.category.keyboard"),
    description: t("settings.category.keyboard.description"),
    icon: <Keyboard className="size-5" />,
    isImplemented: false, // Not yet implemented
  },
  {
    id: "privacy",
    label: t("settings.category.privacy"),
    description: t("settings.category.privacy.description"),
    icon: <Shield className="size-5" />,
    isImplemented: false, // Not yet implemented
  },
  {
    id: "advanced",
    label: t("settings.category.advanced"),
    description: t("settings.category.advanced.description"),
    icon: <Zap className="size-5" />,
    isImplemented: true,
  },
  {
    id: "about",
    label: t("settings.category.about"),
    description: t("settings.category.about.description"),
    icon: <Info className="size-5" />,
    isImplemented: true,
  },
];

/* ============================================================
   REUSABLE SETTINGS ROW
   ============================================================ */

interface SettingsRowProps {
  label: string;
  description?: string;
  children: React.ReactNode;
  align?: "center" | "start";
  className?: string;
}

function SettingsRow({ label, description, align = "center", children, className }: SettingsRowProps) {
  return (
    <div className={cn("flex justify-between gap-4 py-3.5", align === "start" ? "items-start" : "items-center", className)}>
      <div className="min-w-0 flex-1 pr-4">
        <div className="text-[13.5px] font-medium text-text">{label}</div>
        {description ? (
          <div className="text-[11.5px] text-text-tertiary mt-1">{description}</div>
        ) : null}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

/* ============================================================
   SECTION CONTAINER
   ============================================================ */

interface SettingsSectionProps {
  title: string;
  description?: string;
  children: React.ReactNode;
  className?: string;
}

function SettingsSection({ title, description, children, className }: SettingsSectionProps) {
  return (
    <section className={cn("mb-8 last:mb-0", className)}>
      <div className="mb-4">
        <h3 className="text-[11px] font-bold uppercase tracking-wider text-text-tertiary mb-1">{title}</h3>
        {description && <p className="text-[12px] text-text-secondary">{description}</p>}
      </div>
      <div className="space-y-1">{children}</div>
    </section>
  );
}

/* ============================================================
   GLASS DROPDOWN / SELECT
   ============================================================ */

interface GlassDropdownProps {
  value: string;
  options: Array<{ value: string; label: string }>;
  onChange: (value: string) => void;
  placeholder?: string;
  "aria-label"?: string;
  disabled?: boolean;
  className?: string;
  maxHeight?: number;
}

function GlassDropdown({
  value,
  options,
  onChange,
  placeholder,
  "aria-label": ariaLabel,
  disabled = false,
  className,
  maxHeight = 280,
}: GlassDropdownProps) {
  const [isOpen, setIsOpen] = useState(false);
  const [highlightedIndex, setHighlightedIndex] = useState(-1);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const optionRefs = useRef<Map<number, HTMLButtonElement>>(new Map());
  const tokens = getMotionTokens();
  const reduced = prefersReducedMotion();

  const selectedOption = options.find((o) => o.value === value);

  useEffect(() => {
    if (isOpen) {
      const handleKeyDown = (e: KeyboardEvent) => {
        if (e.key === "Escape") {
          setIsOpen(false);
          triggerRef.current?.focus();
        } else if (e.key === "ArrowDown") {
          e.preventDefault();
          setHighlightedIndex((prev) => Math.min(prev + 1, options.length - 1));
        } else if (e.key === "ArrowUp") {
          e.preventDefault();
          setHighlightedIndex((prev) => Math.max(prev - 1, -1));
        } else if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          if (highlightedIndex >= 0) {
            onChange(options[highlightedIndex].value);
            setIsOpen(false);
            triggerRef.current?.focus();
          }
        } else if (e.key === "Tab") {
          setIsOpen(false);
        }
      };
      window.addEventListener("keydown", handleKeyDown);
      return () => window.removeEventListener("keydown", handleKeyDown);
    }
  }, [isOpen, options, highlightedIndex, onChange]);

  useEffect(() => {
    if (isOpen && listRef.current) {
      const highlighted = optionRefs.current.get(highlightedIndex);
      highlighted?.scrollIntoView({ block: "nearest" });
    }
  }, [highlightedIndex, isOpen]);

  const handleOutsideClick = useCallback((e: MouseEvent) => {
    if (triggerRef.current && !triggerRef.current.contains(e.target as Node)) {
      if (listRef.current && !listRef.current.contains(e.target as Node)) {
        setIsOpen(false);
      }
    }
  }, []);

  useEffect(() => {
    if (isOpen) {
      document.addEventListener("mousedown", handleOutsideClick);
      return () => document.removeEventListener("mousedown", handleOutsideClick);
    }
  }, [isOpen, handleOutsideClick]);

  const handleOptionClick = (optValue: string) => {
    onChange(optValue);
    setIsOpen(false);
    triggerRef.current?.focus();
  };

  return (
    <div className={cn("relative", className)}>
      <button
        ref={triggerRef}
        type="button"
        role="combobox"
        aria-expanded={isOpen}
        aria-haspopup="listbox"
        aria-label={ariaLabel}
        disabled={disabled}
        onClick={() => !disabled && setIsOpen(!isOpen)}
        onKeyDown={(e) => {
          if ((e.key === "Enter" || e.key === " " || e.key === "ArrowDown") && !isOpen) {
            e.preventDefault();
            setIsOpen(true);
            setHighlightedIndex(0);
          }
        }}
        className={cn(
          "flex items-center justify-between gap-3 w-full min-w-[180px] max-w-[320px] px-3 py-2 rounded-full glass-strong",
          "transition-colors focus-ring",
          disabled && "opacity-40 pointer-events-none",
          isOpen && "border-glass-border-focus ring-2 ring-accent/20"
        )}
        style={{ transitionDuration: reduced ? "0ms" : `${tokens.duration.toggle}ms` }}
      >
        <span className="text-sm font-medium text-text truncate flex-1">
          {selectedOption?.label ?? placeholder}
        </span>
        <ChevronRight className={cn("size-4 flex-shrink-0 text-text-tertiary transition-transform", isOpen && "rotate-90")} />
      </button>

      {isOpen && (
        <div
          ref={listRef}
          role="listbox"
          aria-label={ariaLabel}
          className="absolute z-[var(--z-toast)] mt-1.5 w-full min-w-[180px] max-w-[320px] glass-intense rounded-xl border border-glass-border-strong shadow-[0_12px_40px_rgba(0,0,0,0.5)] overflow-hidden"
          style={{
            maxHeight,
            animation: reduced ? "none" : `fadeIn ${tokens.duration.overlay}ms ${tokens.easing.panel}`,
          }}
        >
          <div className="p-1" style={{ maxHeight: maxHeight - 8, overflowY: "auto" }}>
            {options.map((option, index) => (
              <button
                key={option.value}
                ref={(el) => { if (el) optionRefs.current.set(index, el); }}
                role="option"
                aria-selected={option.value === value}
                onClick={() => handleOptionClick(option.value)}
                onMouseEnter={() => setHighlightedIndex(index)}
                className={cn(
                  "w-full flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors",
                  option.value === value
                    ? "bg-accent/15 text-accent-text"
                    : "text-text-secondary hover:bg-glass-hover hover:text-text"
                )}
              >
                {option.value === value && (
                  <svg className="size-4 flex-shrink-0 text-accent-text" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5">
                    <polyline points="20 6 9 17 4 12" />
                  </svg>
                )}
                <span className="truncate">{option.label}</span>
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

/* ============================================================
   ACCENT COLOR GRID
   ============================================================ */

const ACCENT_PRESETS = [
  { value: "#fa233b", label: "Melyar Red" },
  { value: "#1db954", label: "Spotify Green" },
  { value: "#ff8a00", label: "Amber" },
  { value: "#7c5cff", label: "Violet" },
  { value: "#00a6c8", label: "Cyan" },
  { value: "#e0218a", label: "Magenta" },
];

function AccentColorPicker({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  return (
    <div className="flex flex-wrap items-center gap-2" role="radiogroup" aria-label={t("settings.accent")}>
      {ACCENT_PRESETS.map((preset) => {
        const active = preset.value === value;
        return (
          <button
            key={preset.value}
            type="button"
            role="radio"
            aria-checked={active}
            aria-label={preset.label}
            onClick={() => onChange(preset.value)}
            className={cn(
              "relative size-10 rounded-full border-2 transition-all",
              "focus-ring",
              active
                ? "border-white scale-110 shadow-[0_0_0_3px_rgba(250,35,59,0.3)]"
                : "border-transparent hover:scale-110"
            )}
            style={{
              backgroundColor: preset.value,
              boxShadow: active ? `0 0 0 3px ${preset.value}40, 0 4px 12px ${preset.value}60` : undefined,
            }}
          >
            {active && (
              <svg className="absolute inset-0 size-6 text-white" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3">
                <polyline points="20 6 9 17 4 12" />
              </svg>
            )}
          </button>
        );
      })}
    </div>
  );
}

/* ============================================================
   LANGUAGE DROPDOWN (using GlassDropdown)
   ============================================================ */

function LanguageSelector({ value, onChange }: { value: string; onChange: (value: string) => void }) {
  return (
    <GlassDropdown
      value={value}
      options={LOCALES.map((l) => ({ value: l.code, label: l.label }))}
      onChange={onChange}
      placeholder={t("settings.language")}
      aria-label={t("settings.language")}
      maxHeight={320}
    />
  );
}

/* ============================================================
   RESET CONFIRMATION DIALOG
   ============================================================ */

interface ResetConfirmDialogProps {
  open: boolean;
  onClose: () => void;
  onConfirm: () => void;
}

function ResetConfirmDialog({ open, onClose, onConfirm }: ResetConfirmDialogProps) {
  const dialogRef = useRef<HTMLDivElement>(null);
  const tokens = getMotionTokens();
  const reduced = prefersReducedMotion();

  useEffect(() => {
    if (open) {
      const handleKey = (e: KeyboardEvent) => {
        if (e.key === "Escape") onClose();
      };
      window.addEventListener("keydown", handleKey);
      const focusable = dialogRef.current?.querySelector<HTMLElement>('button');
      focusable?.focus();
      return () => window.removeEventListener("keydown", handleKey);
    }
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div
      className="fixed inset-0 z-[var(--z-modal-panel)] flex items-center justify-center p-6"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
    >
      <div
        className="absolute inset-0 bg-black/50 backdrop-blur-sm"
        aria-hidden="true"
        style={{ animation: reduced ? "none" : `fadeIn ${tokens.duration.overlay}ms ${tokens.easing.panel}` }}
      />
      <div
        ref={dialogRef}
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="reset-dialog-title"
        aria-describedby="reset-dialog-desc"
        className="relative w-full max-w-md glass-intense rounded-2xl border border-glass-border-strong p-6 shadow-[0_20px_60px_rgba(0,0,0,0.6)]"
        style={{ animation: reduced ? "none" : `slideUp ${tokens.duration.panel}ms ${tokens.easing.panel}` }}
      >
        <div className="flex items-start gap-3">
          <div className="flex-shrink-0 size-10 rounded-full bg-accent/15 grid place-items-center text-accent-text">
            <RotateCcw className="size-5" />
          </div>
          <div className="flex-1">
            <h3 id="reset-dialog-title" className="text-lg font-semibold text-text">
              {t("settings.resetConfirm.title")}
            </h3>
            <p id="reset-dialog-desc" className="text-sm text-text-secondary mt-1">
              {t("settings.resetConfirm.description")}
            </p>
          </div>
        </div>
        <div className="flex justify-end gap-3 mt-6">
          <GlassButton
            variant="ghost"
            size="md"
            onClick={onClose}
            className="w-24"
          >
            {t("common.cancel")}
          </GlassButton>
          <GlassButton
            variant="primary"
            size="md"
            onClick={onConfirm}
            className="w-24"
          >
            {t("settings.resetConfirm.confirm")}
          </GlassButton>
        </div>
      </div>
    </div>
  );
}

/* ============================================================
   CATEGORY CONTENT COMPONENTS
   ============================================================ */

function GeneralContent({ settings, set }: { settings: Settings; set: <K extends keyof Settings>(key: K, value: Settings[K]) => void }) {
  return (
    <>
      <SettingsSection
        title={t("settings.section.general")}
        description={t("settings.section.general.description")}
      >
        <SettingsRow
          label={t("settings.showNotifications")}
          description={t("settings.showNotifications.description")}
        >
          <GlassToggle
            checked={settings.showNotifications}
            onChange={(v) => set("showNotifications", v)}
            aria-label={t("settings.showNotifications")}
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.showAlbumArt")}
          description={t("settings.showAlbumArt.description")}
        >
          <GlassToggle
            checked={settings.showAlbumArt}
            onChange={(v) => set("showAlbumArt", v)}
            aria-label={t("settings.showAlbumArt")}
          />
        </SettingsRow>
        <SettingsRow
          label={t("settings.showLyricsAutomatically")}
          description={t("settings.showLyricsAutomatically.description")}
        >
          <GlassToggle
            checked={settings.showLyricsAutomatically}
            onChange={(v) => set("showLyricsAutomatically", v)}
            aria-label={t("settings.showLyricsAutomatically")}
          />
        </SettingsRow>
      </SettingsSection>
    </>
  );
}

function AppearanceContent({ settings, set }: { settings: Settings; set: <K extends keyof Settings>(key: K, value: Settings[K]) => void }) {
  const themeOptions = [
    { value: "dark", label: t("settings.theme.dark") },
    { value: "light", label: t("settings.theme.light") },
    { value: "system", label: t("settings.theme.system") },
  ];

  return (
    <>
      <SettingsSection
        title={t("settings.section.appearance")}
        description={t("settings.section.appearance.description")}
      >
        <SettingsRow
          label={t("settings.theme")}
          description={t("settings.theme.description")}
        >
          <GlassSegmented
            aria-label={t("settings.theme")}
            value={settings.theme}
            onChange={(v) => set("theme", v as Settings["theme"])}
            options={themeOptions}
          />
        </SettingsRow>

        <SettingsRow
          label={t("settings.accent")}
          description={t("settings.accent.description")}
        >
          <AccentColorPicker value={settings.accentColor} onChange={(v) => set("accentColor", v)} />
        </SettingsRow>

        <SettingsRow
          label={t("settings.language")}
          description={t("settings.language.description")}
        >
          <LanguageSelector value={settings.language} onChange={(v) => set("language", v as Settings["language"])} />
        </SettingsRow>

        <SettingsRow
          label={t("settings.compactMode")}
          description={t("settings.compactMode.description")}
        >
          <GlassToggle
            checked={settings.compactMode}
            onChange={(v) => set("compactMode", v)}
            aria-label={t("settings.compactMode")}
          />
        </SettingsRow>

        <SettingsRow
          label={t("settings.reducedMotion")}
          description={t("settings.reducedMotion.description")}
        >
          <GlassToggle
            checked={settings.reducedMotion}
            onChange={(v) => set("reducedMotion", v)}
            aria-label={t("settings.reducedMotion")}
          />
        </SettingsRow>
      </SettingsSection>
    </>
  );
}

function PlaybackContent({ settings, set }: { settings: Settings; set: <K extends keyof Settings>(key: K, value: Settings[K]) => void }) {
  return (
    <>
      <SettingsSection
        title={t("settings.section.playback")}
        description={t("settings.section.playback.description")}
      >
        <SettingsRow
          label={t("settings.crossfade")}
          description={t("settings.crossfade.description")}
        >
          <GlassToggle
            checked={settings.crossfadeEnabled}
            onChange={(v) => set("crossfadeEnabled", v)}
            aria-label={t("settings.crossfade")}
          />
        </SettingsRow>

        {settings.crossfadeEnabled && (
          <SettingsRow label={t("settings.crossfadeDuration")}>
            <div className="w-48">
              <GlassSlider
                min={1}
                max={15}
                step={1}
                value={settings.crossfadeDuration}
                onChange={(v) => set("crossfadeDuration", v)}
                formatTooltip={(v) => `${v}s`}
                aria-label={t("settings.crossfadeDuration")}
              />
            </div>
          </SettingsRow>
        )}

        <SettingsRow
          label={t("settings.gapless")}
          description={t("settings.gapless.description")}
        >
          <GlassToggle
            checked={settings.gaplessPlayback}
            onChange={(v) => set("gaplessPlayback", v)}
            aria-label={t("settings.gapless")}
          />
        </SettingsRow>

        <SettingsRow
          label={t("settings.autoPlayNext")}
          description={t("settings.autoPlayNext.description")}
        >
          <GlassToggle
            checked={settings.autoPlayNext}
            onChange={(v) => set("autoPlayNext", v)}
            aria-label={t("settings.autoPlayNext")}
          />
        </SettingsRow>

        <SettingsRow
          label={t("settings.normalizeVolume")}
          description={t("settings.normalizeVolume.description")}
        >
          <GlassToggle
            checked={settings.normalizeVolume}
            onChange={(v) => set("normalizeVolume", v)}
            aria-label={t("settings.normalizeVolume")}
          />
        </SettingsRow>

        <SettingsRow
          label={t("settings.quality")}
          description={t("settings.quality.description")}
        >
          <GlassDropdown
            value={settings.streamingQuality}
            options={[
              { value: "low", label: t("settings.quality.low") },
              { value: "medium", label: t("settings.quality.medium") },
              { value: "high", label: t("settings.quality.high") },
              { value: "lossless", label: t("settings.quality.lossless") },
            ]}
            onChange={(v) => set("streamingQuality", v as Settings["streamingQuality"])}
            aria-label={t("settings.quality")}
          />
        </SettingsRow>
      </SettingsSection>
    </>
  );
}

function AudioContent({ settings, set }: { settings: Settings; set: <K extends keyof Settings>(key: K, value: Settings[K]) => void }) {
  const b = typeof window !== "undefined" ? (window as any).__melyarBridge : null;

  return (
    <>
      <SettingsSection
        title={t("settings.section.audio")}
        description={t("settings.section.audio.description")}
      >
        <SettingsRow
          label={t("settings.defaultVolume")}
          description={t("settings.defaultVolume.description")}
        >
          <div className="w-48">
            <GlassSlider
              min={0}
              max={100}
              step={1}
              value={Math.round(settings.volume * 100)}
              onChange={(v) => set("volume", v / 100)}
              formatTooltip={(v) => `${v}%`}
              aria-label={t("settings.defaultVolume")}
            />
          </div>
        </SettingsRow>

        {b && typeof b.getAudioDevices === "function" && (
          <SettingsRow
            label={t("settings.outputDevice")}
            description={t("settings.outputDevice.description")}
          >
            <GlassDropdown
              value={settings.outputDevice || "default"}
              options={[{ value: "default", label: t("settings.outputDevice.default") }]}
              onChange={(v) => set("outputDevice", v)}
              aria-label={t("settings.outputDevice")}
              disabled
            />
          </SettingsRow>
        )}

        <SettingsRow
          label={t("settings.playbackSpeed")}
          description={t("settings.playbackSpeed.description")}
        >
          <GlassDropdown
            value={String(settings.playbackSpeed || 1)}
            options={[
              { value: "0.5", label: "0.5x" },
              { value: "0.75", label: "0.75x" },
              { value: "1", label: "1x" },
              { value: "1.25", label: "1.25x" },
              { value: "1.5", label: "1.5x" },
              { value: "2", label: "2x" },
            ]}
            onChange={(v) => set("playbackSpeed", Number(v))}
            aria-label={t("settings.playbackSpeed")}
            disabled={!b || typeof b.setPlaybackSpeed !== "function"}
          />
        </SettingsRow>

        <SettingsRow
          label={t("settings.audioNormalization")}
          description={t("settings.audioNormalization.description")}
        >
          <GlassToggle
            checked={settings.audioNormalization || false}
            onChange={(v) => set("audioNormalization", v)}
            aria-label={t("settings.audioNormalization")}
            disabled={!b || typeof b.setAudioNormalization !== "function"}
          />
        </SettingsRow>
      </SettingsSection>
    </>
  );
}

function LibraryContent({ settings, set }: { settings: Settings; set: <K extends keyof Settings>(key: K, value: Settings[K]) => void }) {
  const b = typeof window !== "undefined" ? (window as any).__melyarBridge : null;

  return (
    <>
      <SettingsSection
        title={t("settings.section.library")}
        description={t("settings.section.library.description")}
      >
        <SettingsRow
          label={t("settings.musicFolders")}
          description={t("settings.musicFolders.description")}
          align="start"
        >
          <div className="flex items-center gap-2">
            <GlassButton
              variant="ghost"
              size="sm"
              onClick={() => b?.pickMusicFolder?.()}
              disabled={!b || typeof b.pickMusicFolder !== "function"}
            >
              {t("settings.musicFolders.add")}
            </GlassButton>
            {settings.musicFolders?.length && (
              <span className="text-sm text-text-tertiary">
                {t("settings.musicFolders.count", { count: settings.musicFolders.length })}
              </span>
            )}
          </div>
        </SettingsRow>

        {settings.musicFolders?.map((folder: string, index: number) => (
          <SettingsRow key={folder} label="" description={folder} align="start">
            <GlassButton
              variant="ghost"
              size="sm"
              onClick={() => b?.removeMusicFolder?.(index)}
              disabled={!b || typeof b.removeMusicFolder !== "function"}
            >
              {t("common.remove")}
            </GlassButton>
          </SettingsRow>
        ))}

        <SettingsRow
          label={t("settings.autoScan")}
          description={t("settings.autoScan.description")}
        >
          <GlassToggle
            checked={settings.autoScan}
            onChange={(v) => set("autoScan", v)}
            aria-label={t("settings.autoScan")}
            disabled={!b || typeof b.setAutoScan !== "function"}
          />
        </SettingsRow>

        <SettingsRow
          label={t("settings.manualScan")}
          description={t("settings.manualScan.description")}
          align="start"
        >
          <GlassButton
            variant="default"
            size="sm"
            onClick={() => b?.scanLibrary?.()}
            disabled={!b || typeof b.scanLibrary !== "function"}
          >
            {t("settings.manualScan.action")}
          </GlassButton>
        </SettingsRow>

        <SettingsRow
          label={t("settings.artworkCache")}
          description={t("settings.artworkCache.description")}
        >
          <div className="w-48">
            <GlassSlider
              min={50}
              max={500}
              step={50}
              value={settings.artworkCacheSize || 200}
              onChange={(v) => set("artworkCacheSize", v)}
              formatTooltip={(v) => `${v} MB`}
              aria-label={t("settings.artworkCache")}
            />
          </div>
        </SettingsRow>
      </SettingsSection>
    </>
  );
}

function LyricsContent({ settings, set }: { settings: Settings; set: <K extends keyof Settings>(key: K, value: Settings[K]) => void }) {
  return (
    <>
      <SettingsSection
        title={t("settings.section.lyrics")}
        description={t("settings.section.lyrics.description")}
      >
        <SettingsRow
          label={t("settings.autoFollowLyrics")}
          description={t("settings.autoFollowLyrics.description")}
        >
          <GlassToggle
            checked={settings.autoFollowLyrics}
            onChange={(v) => set("autoFollowLyrics", v)}
            aria-label={t("settings.autoFollowLyrics")}
          />
        </SettingsRow>

        <SettingsRow
          label={t("settings.lyricsFontSize")}
          description={t("settings.lyricsFontSize.description")}
        >
          <GlassDropdown
            value={String(settings.lyricsFontSize || "medium")}
            options={[
              { value: "small", label: t("settings.lyricsFontSize.small") },
              { value: "medium", label: t("settings.lyricsFontSize.medium") },
              { value: "large", label: t("settings.lyricsFontSize.large") },
              { value: "extraLarge", label: t("settings.lyricsFontSize.extraLarge") },
            ]}
            onChange={(v) => set("lyricsFontSize", v as Settings["lyricsFontSize"])}
            aria-label={t("settings.lyricsFontSize")}
          />
        </SettingsRow>

        <SettingsRow
          label={t("settings.lyricsAlignment")}
          description={t("settings.lyricsAlignment.description")}
        >
          <GlassSegmented
            aria-label={t("settings.lyricsAlignment")}
            value={settings.lyricsAlignment || "center"}
            onChange={(v) => set("lyricsAlignment", v as Settings["lyricsAlignment"])}
            options={[
              { value: "left", label: t("settings.lyricsAlignment.left") },
              { value: "center", label: t("settings.lyricsAlignment.center") },
              { value: "right", label: t("settings.lyricsAlignment.right") },
            ]}
          />
        </SettingsRow>
      </SettingsSection>
    </>
  );
}

function ProvidersContent({ settings, set }: { settings: Settings; set: <K extends keyof Settings>(key: K, value: Settings[K]) => void }) {
  const b = typeof window !== "undefined" ? (window as any).__melyarBridge : null;

  return (
    <>
      <SettingsSection
        title={t("settings.section.providers")}
        description={t("settings.section.providers.description")}
      >
        <SettingsRow
          label={t("settings.youtubeMusic")}
          description={t("settings.youtubeMusic.description")}
          align="start"
        >
          <span className={cn("inline-flex items-center gap-2 px-2.5 py-1 rounded-full text-xs font-medium", "bg-green/20 text-green")}>
            <span className="size-1.5 rounded-full bg-green" />
            {t("settings.youtubeMusic.enabled")}
          </span>
        </SettingsRow>

        {b && typeof b.getAvailableProviders === "function" && (
          <>
            {b.getAvailableProviders().map((provider: any) => (
              <SettingsRow key={provider.id} label={provider.name} description={provider.description} align="start">
                <GlassToggle
                  checked={provider.enabled}
                  onChange={(v) => b.toggleProvider?.(provider.id, v)}
                  aria-label={provider.name}
                  disabled={provider.required}
                />
              </SettingsRow>
            ))}
          </>
        )}
      </SettingsSection>
    </>
  );
}

/* ============================================================
   EQUALIZER PRESETS & COMPONENT
   ============================================================ */

interface EqualizerPreset {
  id: string;
  /** i18n key; resolved at render so the label follows the locale. */
  labelKey: string;
  icon: React.ReactNode;
  gains: number[];
}

const EQUALIZER_PRESETS: EqualizerPreset[] = [
  {
    id: "flat",
    labelKey: "settings.equalizer.preset.flat",
    icon: <SlidersHorizontal className="size-3.5" />,
    gains: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
  },
  {
    id: "bass",
    labelKey: "settings.equalizer.preset.bass",
    icon: <Disc2 className="size-3.5" />,
    gains: [6, 5, 4, 2, 0, -1, -2, -3, -4, -5],
  },
  {
    id: "rock",
    labelKey: "settings.equalizer.preset.rock",
    icon: <User className="size-3.5" />,
    gains: [4, 3, 1, -1, -2, 0, 2, 3, 4, 3],
  },
  {
    id: "vocal",
    labelKey: "settings.equalizer.preset.vocal",
    icon: <Mic className="size-3.5" />,
    gains: [-3, -2, -1, 1, 4, 5, 4, 2, 0, -1],
  },
  {
    id: "classical",
    labelKey: "settings.equalizer.preset.classical",
    icon: <TreePine className="size-3.5" />,
    gains: [3, 2, 1, 0, -1, -1, 0, 2, 3, 4],
  },
];

const EQUALIZER_BAND_FREQUENCIES = [31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000];

/** Gain bounds, in decibels. Mirrors the Rust side. */
const EQ_MIN_DB = -12;
const EQ_MAX_DB = 12;

interface EqualizerProps {
  enabled: boolean;
  bands: number[];
  onEnableChange: (enabled: boolean) => void;
  onBandsChange: (bands: number[]) => void;
  onReset: () => void;
  onPresetSelect: (presetId: string) => void;
}

function Equalizer({ enabled, bands, onEnableChange, onBandsChange, onReset, onPresetSelect }: EqualizerProps) {
  const [hoveredBand, setHoveredBand] = useState<number | null>(null);
  const [activeBand, setActiveBand] = useState<number | null>(null);
  const [selectedPreset, setSelectedPreset] = useState<string>("custom");
  const wheelRefs = useRef<Array<HTMLDivElement | null>>([]);
  const tokens = getMotionTokens();
  const reduced = prefersReducedMotion();

  // Check if current bands match a preset
  useEffect(() => {
    let matched = "custom";
    for (const preset of EQUALIZER_PRESETS) {
      if (bands.every((v, i) => v === preset.gains[i])) {
        matched = preset.id;
        break;
      }
    }
    setSelectedPreset(matched);
  }, [bands]);

  // Generate SVG path for the equalizer curve
  const curvePath = useMemo(() => {
    const width = 600; // logical width for calculation
    const height = 200; // logical height
    const paddingX = 40;
    const paddingY = 20;
    const innerWidth = width - paddingX * 2;
    const innerHeight = height - paddingY * 2;

    // Map frequency to x position (logarithmic scale)
    const freqToX = (freq: number) => {
      const minFreq = 31;
      const maxFreq = 16000;
      const logMin = Math.log(minFreq);
      const logMax = Math.log(maxFreq);
      const logFreq = Math.log(freq);
      const ratio = (logFreq - logMin) / (logMax - logMin);
      return paddingX + ratio * innerWidth;
    };

    // Map gain (-12 to +12 dB) to y position
    const gainToY = (gain: number) => {
      const maxGain = 12;
      const ratio = (gain + maxGain) / (2 * maxGain); // 0 to 1
      return paddingY + (1 - ratio) * innerHeight; // inverted: positive gain = up
    };

    const points = EQUALIZER_BAND_FREQUENCIES.map((freq, i) => ({
      x: freqToX(freq),
      y: gainToY(bands[i] ?? 0),
    }));

    // Create smooth curve using Catmull-Rom spline
    let path = `M ${points[0].x} ${points[0].y}`;
    for (let i = 0; i < points.length - 1; i++) {
      const p0 = points[i - 1] || points[0];
      const p1 = points[i];
      const p2 = points[i + 1];
      const p3 = points[i + 2] || points[points.length - 1];

      const tension = 0.5;
      const cp1x = p1.x + (p2.x - p0.x) * tension / 6;
      const cp1y = p1.y + (p2.y - p0.y) * tension / 6;
      const cp2x = p2.x - (p3.x - p1.x) * tension / 6;
      const cp2y = p2.y - (p3.y - p1.y) * tension / 6;

      path += ` C ${cp1x} ${cp1y} ${cp2x} ${cp2y} ${p2.x} ${p2.y}`;
    }

    return path;
  }, [bands]);

  const handleBandChange = (index: number, value: number) => {
    const newBands = [...bands];
    newBands[index] = Math.max(-12, Math.min(12, Math.round(value)));
    onBandsChange(newBands);
    setSelectedPreset("custom");
  };

  const handlePresetSelect = (preset: EqualizerPreset) => {
    onBandsChange([...preset.gains]);
    onPresetSelect(preset.id);
    setSelectedPreset(preset.id);
  };

  // React registers a wheel handler from JSX as a passive listener, so
  // preventDefault() there is ignored - the panel would scroll away while the
  // band moved. A native listener registered with { passive: false } is the
  // only way to claim the gesture. The refs are re-read on every render so
  // the handler always sees the current gains.
  const bandsRef = useRef(bands);
  bandsRef.current = bands;
  const changeRef = useRef(handleBandChange);
  changeRef.current = handleBandChange;

  useEffect(() => {
    const cleanups: Array<() => void> = [];
    wheelRefs.current.forEach((node, index) => {
      if (!node) return;
      const onWheel = (e: WheelEvent) => {
        e.preventDefault();
        const direction = e.deltaY > 0 ? -1 : 1;
        const step = e.shiftKey ? 3 : 1;
        const current = bandsRef.current[index] ?? 0;
        const next = Math.max(EQ_MIN_DB, Math.min(EQ_MAX_DB, current + direction * step));
        if (next !== current) changeRef.current(index, next);
      };
      node.addEventListener("wheel", onWheel, { passive: false });
      cleanups.push(() => node.removeEventListener("wheel", onWheel));
    });
    return () => cleanups.forEach((fn) => fn());
  }, [enabled]);

  const formatFrequency = (freq: number) => {
    if (freq >= 1000) return `${freq / 1000}k`;
    return `${freq}`;
  };

  const formatFrequencyFull = (freq: number) => {
    if (freq >= 1000) return `${freq / 1000} kHz`;
    return `${freq} Hz`;
  };

  return (
    <div className="space-y-6" data-equalizer>
      {/* Header: Enable Toggle + Presets + Reset */}
      <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <GlassToggle
            checked={enabled}
            onChange={onEnableChange}
            aria-label={t("settings.equalizer")}
            size="md"
          />
          <div>
            <div className="text-sm font-medium text-text">{t("settings.equalizer")}</div>
            <div className="text-[11px] text-text-tertiary">{t("settings.equalizer.description")}</div>
          </div>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          {EQUALIZER_PRESETS.map((preset) => (
            <button
              key={preset.id}
              type="button"
              onClick={() => handlePresetSelect(preset)}
              className={cn(
                "flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium transition-all",
                "focus-ring",
                selectedPreset === preset.id
                  ? "bg-accent/15 text-accent-text border border-accent/30"
                  : "text-text-secondary hover:text-text hover:bg-glass-hover border border-glass-border"
              )}
              style={{ transitionDuration: reduced ? "0ms" : `${tokens.duration.toggle}ms` }}
            >
              {preset.icon}
              <span>{t(preset.labelKey)}</span>
            </button>
          ))}
          <button
            type="button"
            onClick={onReset}
            className={cn(
              "flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium transition-all",
              "focus-ring",
              "text-text-secondary hover:text-text hover:bg-glass-hover border border-glass-border"
            )}
            style={{ transitionDuration: reduced ? "0ms" : `${tokens.duration.toggle}ms` }}
          >
            <RefreshCw className="size-3.5" />
            <span>{t("settings.equalizer.reset")}</span>
          </button>
        </div>
      </div>

      {/* Equalizer Visualization + Sliders */}
      <div className={cn("relative glass rounded-2xl p-6", enabled ? "" : "opacity-50 pointer-events-none")}>
        {/* SVG Curve Visualization */}
        <div className="relative h-40 mb-8" aria-hidden="true">
          {/* Decorative: the ten sliders below are the accessible interface,
              each with its own label and value, so this is hidden from the
              accessibility tree rather than announced twice. */}
          <svg
            viewBox="0 0 600 200"
            preserveAspectRatio="none"
            className="w-full h-full"
          >
            {/* Grid lines */}
            <defs>
              <pattern id="grid" width="50" height="25" patternUnits="userSpaceOnUse">
                <path d="M 50 0 L 0 0 0 25" fill="none" stroke="var(--glass-border)" strokeWidth="0.5" />
              </pattern>
            </defs>
            <rect x="40" y="20" width="520" height="160" fill="url(#grid)" />

            {/* Zero dB reference line */}
            <line
              x1="40"
              y1="100"
              x2="560"
              y2="100"
              stroke="var(--glass-border-strong)"
              strokeWidth="1"
              strokeDasharray="4 4"
              opacity="0.5"
            />

            {/* Frequency markers */}
            {EQUALIZER_BAND_FREQUENCIES.map((freq, i) => {
              const minFreq = 31;
              const maxFreq = 16000;
              const logMin = Math.log(minFreq);
              const logMax = Math.log(maxFreq);
              const logFreq = Math.log(freq);
              const ratio = (logFreq - logMin) / (logMax - logMin);
              const x = 40 + ratio * 520;
              return (
                <g key={i} className="pointer-events-none">
                  <line
                    x1={x}
                    y1="20"
                    x2={x}
                    y2="180"
                    stroke="var(--glass-border)"
                    strokeWidth="0.5"
                    opacity="0.3"
                  />
                  <text
                    x={x}
                    y="195"
                    textAnchor="middle"
                    fontSize="10"
                    fill="var(--text-tertiary)"
                    fontFamily="var(--font-sans)"
                  >
                    {formatFrequency(freq)}
                  </text>
                </g>
              );
            })}

            {/* Gain markers on left */}
            {[12, 6, 0, -6, -12].map((gain) => {
              const y = 20 + (1 - (gain + 12) / 24) * 160;
              return (
                <g key={gain} className="pointer-events-none">
                  <line
                    x1="35"
                    y1={y}
                    x2="40"
                    y2={y}
                    stroke="var(--glass-border-strong)"
                    strokeWidth="1"
                  />
                  <text
                    x="33"
                    y={y + 3}
                    textAnchor="end"
                    fontSize="9"
                    fill="var(--text-tertiary)"
                    fontFamily="var(--font-sans)"
                    style={{ fontVariantNumeric: "tabular-nums" }}
                  >
                    {gain > 0 ? `+${gain}` : gain}
                  </text>
                </g>
              );
            })}

            {/* Equalizer Curve */}
            <path
              d={curvePath}
              fill="none"
              stroke="var(--accent)"
              strokeWidth="2.5"
              strokeLinecap="round"
              strokeLinejoin="round"
              filter="drop-shadow(0 0 8px var(--accent))"
              style={{
                transition: reduced ? "none" : `stroke ${tokens.duration.press}ms ${tokens.easing.standard}`,
              }}
            />

            {/* Area under curve */}
            <path
              d={`${curvePath} L 560 180 L 40 180 Z`}
              fill="var(--accent-soft)"
              opacity="0.3"
              style={{
                transition: reduced ? "none" : `opacity ${tokens.duration.press}ms ${tokens.easing.standard}`,
              }}
            />

            {/* Band points on curve */}
            {EQUALIZER_BAND_FREQUENCIES.map((freq, i) => {
              const minFreq = 31;
              const maxFreq = 16000;
              const logMin = Math.log(minFreq);
              const logMax = Math.log(maxFreq);
              const logFreq = Math.log(freq);
              const ratio = (logFreq - logMin) / (logMax - logMin);
              const x = 40 + ratio * 520;
              const y = 20 + (1 - (bands[i] + 12) / 24) * 160;
              const isHovered = hoveredBand === i || activeBand === i;
              return (
                <circle
                  key={i}
                  cx={x}
                  cy={y}
                  r={isHovered ? 7 : 5}
                  fill="var(--accent)"
                  stroke="var(--bg)"
                  strokeWidth={isHovered ? 3 : 2}
                  className={cn("transition-all cursor-pointer", isHovered && "scale-125")}
                  style={{ transitionDuration: reduced ? "0ms" : `${tokens.duration.press}ms` }}
                />
              );
            })}
          </svg>
        </div>

        {/* Vertical Sliders */}
        <div
          className="flex items-end justify-between gap-1 sm:gap-2 px-2 sm:px-4 pb-2"
          role="group"
          aria-label={t("settings.equalizer.bands")}
        >
          {EQUALIZER_BAND_FREQUENCIES.map((freq, index) => {
            const gain = bands[index] ?? 0;
            const isCustom = gain !== 0;
            return (
              <div
                key={freq}
                className="flex flex-col items-center gap-2 flex-1 min-w-0"
                onMouseEnter={() => setHoveredBand(index)}
                onMouseLeave={() => setHoveredBand(null)}
              >
                {/* Gain Value Display */}
                <div
                  className={cn(
                    "text-[12px] font-mono tabular-nums px-1.5 py-0.5 rounded-md transition-colors",
                    activeBand === index || hoveredBand === index
                      ? "text-accent-text bg-accent/10"
                      : isCustom
                        ? "text-text"
                        : "text-text-tertiary"
                  )}
                  style={{ transitionDuration: reduced ? "0ms" : `${tokens.duration.press}ms` }}
                >
                  {gain > 0 ? `+${gain}` : gain}
                </div>

                {/* Vertical Slider. The wheel listener is attached to this
                    centred strip rather than the whole column so that a
                    scroll gesture in the gaps still moves the panel. */}
                <div
                  ref={(el) => { wheelRefs.current[index] = el; }}
                  className="relative flex justify-center w-12 h-44 sm:h-52"
                  onFocus={() => setActiveBand(index)}
                  onBlur={() => setActiveBand(null)}
                >
                  <GlassSlider
                    orientation="vertical"
                    min={EQ_MIN_DB}
                    max={EQ_MAX_DB}
                    step={1}
                    value={gain}
                    onChange={(v) => handleBandChange(index, v)}
                    trackHeight={6}
                    thumbSize={18}
                    showTooltip={false}
                    aria-label={formatFrequencyFull(freq)}
                  />
                </div>

                {/* Frequency Label */}
                <div className="text-[10px] font-semibold text-text-tertiary tabular-nums whitespace-nowrap">
                  {formatFrequency(freq)}
                </div>
              </div>
            );
          })}
        </div>

        {/* Scale legend */}
        <div className="flex items-center justify-between mt-3 px-2 text-[10px] text-text-tertiary">
          <span>{t("settings.equalizer.hint")}</span>
          <span className="tabular-nums">{EQ_MIN_DB} dB — {EQ_MAX_DB} dB</span>
        </div>
      </div>
    </div>
  );
}

function AdvancedContent({ settings, set }: { settings: Settings; set: <K extends keyof Settings>(key: K, value: Settings[K]) => void }) {
  const [cacheSizeValue, setCacheSizeValue] = useState(settings.cacheSize);

  const handleCacheSizeChange = useCallback(
    (v: number) => {
      setCacheSizeValue(v);
      set("cacheSize", v);
    },
    [set],
  );

  const handleEqualizerBandsChange = useCallback(
    (bands: number[]) => {
      set("equalizerBands", bands);
    },
    [set],
  );

  const handleEqualizerReset = useCallback(() => {
    set("equalizerBands", [0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
  }, [set]);

  const handleEqualizerPresetSelect = useCallback(() => {
    // Preset selection is handled by the Equalizer component
  }, []);

  return (
    <>
      <SettingsSection
        title={t("settings.section.advanced")}
        description={t("settings.section.advanced.description")}
      >
        {/* Cache Size with visible numeric value */}
        <SettingsRow
          label={t("settings.cacheSize")}
          description={t("settings.cacheSize.description")}
        >
          <div className="flex items-center gap-4 w-full max-w-xs">
            <div className="flex-1 min-w-0">
              <GlassSlider
                min={100}
                max={2000}
                step={100}
                value={cacheSizeValue}
                onChange={handleCacheSizeChange}
                formatTooltip={(v) => `${v} MB`}
                aria-label={t("settings.cacheSize")}
              />
            </div>
            <div
              className="w-20 text-right font-mono text-sm text-text tabular-nums"
              aria-live="polite"
            >
              {cacheSizeValue} MB
            </div>
          </div>
        </SettingsRow>

        {/* Professional Equalizer */}
        <SettingsRow label="" align="start" className="pt-4 border-t border-glass-border mt-4">
          <Equalizer
            enabled={settings.equalizerEnabled}
            bands={settings.equalizerBands}
            onEnableChange={(v) => set("equalizerEnabled", v)}
            onBandsChange={handleEqualizerBandsChange}
            onReset={handleEqualizerReset}
            onPresetSelect={handleEqualizerPresetSelect}
          />
        </SettingsRow>
      </SettingsSection>
    </>
  );
}

function AboutContent({ settings, set }: { settings: Settings; set: <K extends keyof Settings>(key: K, value: Settings[K]) => void }) {
  const version = typeof window !== "undefined" ? (window as any).__melyarVersion : "2.0.0";

  return (
    <>
      <SettingsSection
        title={t("settings.section.about")}
        description={t("settings.section.about.description")}
      >
        <div className="space-y-4">
          <div className="flex items-center gap-4 p-4 glass rounded-xl">
            <div className="size-16 rounded-xl bg-gradient-to-br from-accent to-[#d61d33] grid place-items-center">
              <Music2 className="size-8 text-on-accent" />
            </div>
            <div>
              <h4 className="text-lg font-semibold text-text">Melyar</h4>
              <p className="text-sm text-text-secondary">{t("settings.about.version", { version })}</p>
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <GlassButton
              variant="ghost"
              size="sm"
              onClick={() => window.open("https://github.com/melyar/melyar", "_blank")}
            >
              {t("settings.about.repository")}
            </GlassButton>
            <GlassButton
              variant="ghost"
              size="sm"
              onClick={() => window.open("https://github.com/melyar/melyar/blob/main/LICENSE", "_blank")}
            >
              {t("settings.about.license")}
            </GlassButton>
            <GlassButton
              variant="ghost"
              size="sm"
              onClick={() => window.open("https://github.com/melyar/melyar/blob/main/THIRD_PARTY_LICENSES.md", "_blank")}
            >
              {t("settings.about.thirdPartyLicenses")}
            </GlassButton>
            <GlassButton
              variant="ghost"
              size="sm"
              onClick={() => window.open("https://github.com/melyar/melyar/issues", "_blank")}
            >
              {t("settings.about.reportIssue")}
            </GlassButton>
          </div>

          <div className="pt-4 border-t border-glass-border">
            <p className="text-xs text-text-tertiary text-center">
              {t("settings.about.builtWith", { technologies: "React, TypeScript, Vite, Tailwind CSS, Rust, Qt/C++" })}
            </p>
          </div>
        </div>
      </SettingsSection>
    </>
  );
}

/* ============================================================
   CATEGORY CONTENT MAP
   ============================================================ */

function renderCategoryContent(
  categoryId: string,
  settings: Settings,
  set: <K extends keyof Settings>(key: K, value: Settings[K]) => void
) {
  switch (categoryId) {
    case "general":
      return <GeneralContent settings={settings} set={set} />;
    case "appearance":
      return <AppearanceContent settings={settings} set={set} />;
    case "playback":
      return <PlaybackContent settings={settings} set={set} />;
    case "audio":
      return <AudioContent settings={settings} set={set} />;
    case "library":
      return <LibraryContent settings={settings} set={set} />;
    case "lyrics":
      return <LyricsContent settings={settings} set={set} />;
    case "providers":
      return <ProvidersContent settings={settings} set={set} />;
    case "advanced":
      return <AdvancedContent settings={settings} set={set} />;
    case "about":
      return <AboutContent settings={settings} set={set} />;
    default:
      return null;
  }
}

/* ============================================================
   MAIN SETTINGS PANEL
   ============================================================ */

export function SettingsPanel() {
  useLocale();
  const open = useSettings((s) => s.open);
  const setOpen = useSettings((s) => s.setOpen);
  const settings = useSettings((s) => s.settings);
  const set = useSettings((s) => s.set);
  const reset = useSettings((s) => s.reset);

  const panelRef = useRef<HTMLDivElement>(null);
  const navRef = useRef<HTMLDivElement>(null);
  const [selectedCategory, setSelectedCategory] = useState("appearance");
  const [showResetConfirm, setShowResetConfirm] = useState(false);
  const [isNarrow, setIsNarrow] = useState(false);

  // Filter only implemented categories
  const availableCategories = CATEGORIES.filter((c) => c.isImplemented);

  // Update narrow state on resize
  useEffect(() => {
    const checkWidth = () => {
      setIsNarrow(window.innerWidth < 1024);
    };
    checkWidth();
    window.addEventListener("resize", checkWidth);
    return () => window.removeEventListener("resize", checkWidth);
  }, []);

  // Ensure selected category is valid
  useEffect(() => {
    if (!availableCategories.find((c) => c.id === selectedCategory)) {
      setSelectedCategory(availableCategories[0]?.id || "appearance");
    }
  }, [availableCategories, selectedCategory]);

  const handleEscape = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (showResetConfirm) {
          setShowResetConfirm(false);
        } else {
          setOpen(false);
        }
      }
    },
    [setOpen, showResetConfirm],
  );

  useEffect(() => {
    if (!open) return;
    window.addEventListener("keydown", handleEscape);
    return () => window.removeEventListener("keydown", handleEscape);
  }, [open, handleEscape]);

  useEffect(() => {
    if (open && panelRef.current) {
      panelRef.current.focus();
    }
  }, [open]);

  const handleReset = () => {
    setShowResetConfirm(true);
  };

  const confirmReset = () => {
    reset();
    setShowResetConfirm(false);
  };

  if (!open) return null;

  const tokens = getMotionTokens();
  const reduced = prefersReducedMotion();

  return (
    <div
      className="settings-overlay fixed inset-0 z-[var(--z-modal-panel)] flex items-center justify-center p-4"
      onClick={(e) => {
        if (e.target === e.currentTarget && !showResetConfirm) setOpen(false);
      }}
    >
      <div
        ref={panelRef}
        role="dialog"
        aria-modal="true"
        aria-label={t("settings.title")}
        tabIndex={-1}
        className={cn(
          "settings-panel glass-intense rounded-2xl shadow-[0_24px_80px_rgba(0,0,0,0.6)] flex flex-col overflow-hidden",
          isNarrow ? "w-full max-w-full max-h-[90vh]" : "w-full max-w-5xl max-h-[85vh]"
        )}
        style={{
          animation: reduced ? "none" : `slideIn ${tokens.duration.panel}ms ${tokens.easing.panel}`,
        }}
      >
        {/* Header */}
        <header className="flex items-center justify-between px-6 py-4 shrink-0 border-b border-glass-border">
          <div className="flex items-center gap-4">
            <div className="size-10 rounded-xl bg-accent/15 grid place-items-center text-accent-text">
              <SettingsIcon className="size-5" />
            </div>
            <div>
              <h2 className="text-[17px] font-bold tracking-tight text-text">{t("settings.title")}</h2>
              <p className="text-[12px] text-text-tertiary">{t("settings.subtitle")}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <GlassButton
              variant="ghost"
              size="sm"
              aria-label={t("settings.reset")}
              onClick={handleReset}
              className="hidden sm:flex"
            >
              <RotateCcw className="size-4" />
              <span>{t("settings.resetShort")}</span>
            </GlassButton>
            <GlassButton
              variant="ghost"
              size="sm"
              aria-label={t("settings.reset")}
              onClick={handleReset}
              className="sm:hidden"
            >
              <RotateCcw className="size-4" />
            </GlassButton>
            <GlassIconButton
              size="md"
              aria-label={t("common.close")}
              onClick={() => setOpen(false)}
            >
              <X className="size-4" />
            </GlassIconButton>
          </div>
        </header>

        {/* Main Content Area */}
        <div className="flex-1 min-h-0 overflow-hidden flex">
          {/* Category Navigation */}
          <nav
            ref={navRef}
            role="navigation"
            aria-label={t("settings.categories")}
            className={cn(
              "flex flex-col shrink-0 border-r border-glass-border",
              isNarrow ? "hidden" : "w-56"
            )}
          >
            <div className="flex-1 overflow-y-auto py-4 px-2">
              <div className="relative" role="tablist">
                {availableCategories.map((category) => {
                  const isActive = category.id === selectedCategory;
                  return (
                    <button
                      key={category.id}
                      role="tab"
                      aria-selected={isActive}
                      aria-controls={`panel-${category.id}`}
                      id={`tab-${category.id}`}
                      onClick={() => setSelectedCategory(category.id)}
                      className={cn(
                        "relative z-10 w-full flex items-center gap-3 px-3 py-2.5 rounded-xl text-sm font-medium",
                        "transition-colors focus-ring",
                        isActive
                          ? "text-accent-text bg-accent/10"
                          : "text-text-secondary hover:text-text hover:bg-glass-hover"
                      )}
                      style={{
                        transitionDuration: reduced ? "0ms" : `${tokens.duration.toggle}ms`,
                      }}
                    >
                      <span className={cn("flex-shrink-0 size-5 grid place-items-center", isActive && "text-accent-text")}>
                        {category.icon}
                      </span>
                      <span className="truncate">{category.label}</span>
                    </button>
                  );
                })}
              </div>
            </div>
            <div className="p-3 border-t border-glass-border">
              <p className="text-[11px] text-text-tertiary text-center">
                {t("settings.navigation.hint")}
              </p>
            </div>
          </nav>

          {/* Category Content */}
          <div
            role="tabpanel"
            id={`panel-${selectedCategory}`}
            aria-labelledby={`tab-${selectedCategory}`}
            className="flex-1 min-h-0 overflow-y-auto scroll-thin p-6"
          >
            {renderCategoryContent(selectedCategory, settings, set)}
          </div>
        </div>

        {/* Mobile category selector */}
        {isNarrow && (
          <div className="px-4 py-3 border-b border-glass-border shrink-0">
            <GlassDropdown
              value={selectedCategory}
              options={availableCategories.map((c) => ({ value: c.id, label: c.label }))}
              onChange={setSelectedCategory}
              placeholder={t("settings.category.select")}
              aria-label={t("settings.category.select")}
              className="w-full"
            />
          </div>
        )}
      </div>

      <ResetConfirmDialog
        open={showResetConfirm}
        onClose={() => setShowResetConfirm(false)}
        onConfirm={confirmReset}
      />
    </div>
  );
}

/* ============================================================
   CSS KEYFRAMES (injected via style tag for animation)
   ============================================================ */

// Animations are handled via CSS in glass.css or Tailwind

export default SettingsPanel;