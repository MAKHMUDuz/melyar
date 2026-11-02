import { useCallback, useEffect, useRef } from "react";
import { X, RotateCcw } from "lucide-react";
import { cn } from "@/lib/cn";
import { t, useLocale, LOCALES } from "@/lib/i18n";
import { useSettings, EQUALIZER_BAND_FREQUENCIES, type Settings } from "@/lib/settings";
import { GlassToggle, GlassSegmented, GlassSlider } from "@/components/GlassComponents";

const ACCENT_PRESETS = ["#fa233b", "#1db954", "#ff8a00", "#7c5cff", "#00a6c8", "#e0218a"];

function Row({
  label,
  description,
  align = "center",
  children,
}: {
  label: string;
  description?: string;
  /** A control taller than one line — the language grid is five — reads
      better with its label as a heading at the top than floating at half
      its height. */
  align?: "center" | "start";
  children: React.ReactNode;
}) {
  return (
    <div
      className={cn(
        "flex justify-between gap-4 py-3",
        align === "start" ? "items-start" : "items-center",
      )}
    >
      <div className="min-w-0">
        <div className="text-[13px] font-medium text-text">{label}</div>
        {description ? (
          <div className="text-[11.5px] text-text-tertiary mt-0.5">{description}</div>
        ) : null}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <section className="mb-5 last:mb-0">
      <h3 className="text-[11px] font-bold uppercase tracking-wider text-text-tertiary mb-0.5 px-1">
        {title}
      </h3>
      {/* The rows sit directly on the panel, separated by hairlines.
          Wrapping each group in its own bordered card nested one more box
          inside a panel that is already a box, so the eye counted three
          borders before it reached a single setting — and the group's title
          floated above a container that had nothing to do with it. */}
      <div className="divide-y divide-line">{children}</div>
    </section>
  );
}

export function SettingsPanel() {
  useLocale();
  const open = useSettings((s) => s.open);
  const setOpen = useSettings((s) => s.setOpen);
  const settings = useSettings((s) => s.settings);
  const set = useSettings((s) => s.set);
  const reset = useSettings((s) => s.reset);

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
      // The dialog takes focus, not the first control inside it. The first
      // control is "reset everything", so opening the panel used to arm the
      // one action the user is least likely to want — a stray Enter keypress
      // threw away every setting, with no confirmation and no undo.
      panelRef.current.focus();
    }
  }, [open]);

  if (!open) return null;

  const setBool = (key: keyof Settings) => (v: boolean) => set(key, v as Settings[typeof key]);
  const setNum = (key: keyof Settings) => (v: number) => set(key, v as Settings[typeof key]);
  const setStr = (key: keyof Settings) => (v: string) => set(key, v as Settings[typeof key]);

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
        aria-label={t("settings.title")}
        tabIndex={-1}
        className="settings-panel w-full max-w-lg max-h-[85vh] rounded-2xl shadow-2xl flex flex-col overflow-hidden"
      >
        <header className="flex items-center justify-between px-5 py-3.5 shrink-0">
          <h2 className="text-[15px] font-bold tracking-tight text-text">{t("settings.title")}</h2>
          <div className="flex items-center gap-1">
            <button
              className="size-8 rounded-full grid place-items-center text-text-tertiary transition-colors hover:bg-hover hover:text-text"
              aria-label={t("settings.reset")}
              onClick={() => reset()}
            >
              <RotateCcw className="size-4" />
            </button>
            <button
              className="size-8 rounded-full grid place-items-center text-text-tertiary transition-colors hover:bg-hover hover:text-text"
              aria-label={t("common.close")}
              onClick={() => setOpen(false)}
            >
              <X className="size-4" />
            </button>
          </div>
        </header>

        <div className="scroll-thin flex-1 min-h-0 overflow-y-auto px-5 py-4">
          <Section title={t("settings.section.appearance")}>
            <Row label={t("settings.theme")} description={t("settings.theme.description")}>
              <GlassSegmented
                aria-label={t("settings.theme")}
                value={settings.theme}
                onChange={setStr("theme")}
                options={[
                  { value: "dark", label: t("settings.theme.dark") },
                  { value: "light", label: t("settings.theme.light") },
                ]}
              />
            </Row>
            <Row label={t("settings.accent")} description={t("settings.accent.description")}>
              <div className="flex items-center gap-1.5">
                {ACCENT_PRESETS.map((color) => (
                  <button
                    key={color}
                    type="button"
                    className={cn(
                      "size-6 rounded-full border-2 transition-transform",
                      settings.accentColor === color
                        ? "border-white scale-110"
                        : "border-transparent hover:scale-110",
                    )}
                    style={{ backgroundColor: color }}
                    aria-label={color}
                    aria-pressed={settings.accentColor === color}
                    onClick={() => set("accentColor", color)}
                  />
                ))}
              </div>
            </Row>
            <Row
              label={t("settings.language")}
              description={t("settings.language.description")}
              align="start"
            >
              <div className="grid grid-cols-3 gap-1 w-60">
                {LOCALES.map((entry) => {
                  const active = entry.code === settings.language;
                  return (
                    <button
                      key={entry.code}
                      type="button"
                      className={cn(
                        "rounded-md px-2 py-1 text-[11px] font-medium transition-colors truncate",
                        active
                          ? "bg-accent/20 text-accent-text"
                          : "text-text-tertiary hover:text-text hover:bg-hover",
                      )}
                      aria-pressed={active}
                      onClick={() => set("language", entry.code)}
                    >
                      {entry.label}
                    </button>
                  );
                })}
              </div>
            </Row>
            <Row label={t("settings.compactMode")} description={t("settings.compactMode.description")}>
              <GlassToggle
                checked={settings.compactMode}
                onChange={setBool("compactMode")}
                aria-label={t("settings.compactMode")}
              />
            </Row>
            <Row label={t("settings.animations")} description={t("settings.animations.description")}>
              <GlassToggle
                checked={settings.animationsEnabled}
                onChange={setBool("animationsEnabled")}
                aria-label={t("settings.animations")}
              />
            </Row>
          </Section>

          <Section title={t("settings.section.playback")}>
            <Row label={t("settings.crossfade")} description={t("settings.crossfade.description")}>
              <GlassToggle
                checked={settings.crossfadeEnabled}
                onChange={setBool("crossfadeEnabled")}
                aria-label={t("settings.crossfade")}
              />
            </Row>
            {settings.crossfadeEnabled ? (
              <Row label={t("settings.crossfadeDuration")}>
                <div className="w-40">
                  <GlassSlider
                    min={1}
                    max={15}
                    step={1}
                    value={settings.crossfadeDuration}
                    onChange={setNum("crossfadeDuration")}
                    formatTooltip={(v) => `${v}s`}
                    aria-label={t("settings.crossfadeDuration")}
                  />
                </div>
              </Row>
            ) : null}
            <Row label={t("settings.gapless")} description={t("settings.gapless.description")}>
              <GlassToggle
                checked={settings.gaplessPlayback}
                onChange={setBool("gaplessPlayback")}
                aria-label={t("settings.gapless")}
              />
            </Row>
            <Row label={t("settings.autoPlayNext")} description={t("settings.autoPlayNext.description")}>
              <GlassToggle
                checked={settings.autoPlayNext}
                onChange={setBool("autoPlayNext")}
                aria-label={t("settings.autoPlayNext")}
              />
            </Row>
            <Row label={t("settings.normalizeVolume")}>
              <GlassToggle
                checked={settings.normalizeVolume}
                onChange={setBool("normalizeVolume")}
                aria-label={t("settings.normalizeVolume")}
              />
            </Row>
            <Row label={t("settings.quality")} description={t("settings.quality.description")}>
              <GlassSegmented
                aria-label={t("settings.quality")}
                value={settings.streamingQuality}
                onChange={setStr("streamingQuality")}
                options={[
                  { value: "low", label: t("settings.quality.low") },
                  { value: "medium", label: t("settings.quality.medium") },
                  { value: "high", label: t("settings.quality.high") },
                  { value: "lossless", label: t("settings.quality.lossless") },
                ]}
              />
            </Row>
          </Section>

          <Section title={t("settings.section.interface")}>
            <Row label={t("settings.showLyricsAutomatically")}>
              <GlassToggle
                checked={settings.showLyricsAutomatically}
                onChange={setBool("showLyricsAutomatically")}
                aria-label={t("settings.showLyricsAutomatically")}
              />
            </Row>
            {/* Notifications have no delivery consumer yet (#39). Keep the
                persisted preference/locale keys, but do not offer a no-op control. */}
            <Row label={t("settings.showAlbumArt")}>
              <GlassToggle
                checked={settings.showAlbumArt}
                onChange={setBool("showAlbumArt")}
                aria-label={t("settings.showAlbumArt")}
              />
            </Row>
            <Row label={t("settings.equalizer")} description={t("settings.equalizer.description")}>
              <GlassToggle
                checked={settings.equalizerEnabled}
                onChange={setBool("equalizerEnabled")}
                aria-label={t("settings.equalizer")}
              />
            </Row>
            {settings.equalizerEnabled ? (
              <div className="py-2.5">
                <div className="text-[11.5px] text-text-tertiary mb-2">
                  {t("settings.equalizer.bands")}
                </div>
                <div className="flex items-end justify-between gap-1">
                  {EQUALIZER_BAND_FREQUENCIES.map((frequency, index) => (
                    <div key={frequency} className="flex flex-col items-center gap-1 w-8">
                      <input
                        type="range"
                        min={-12}
                        max={12}
                        step={1}
                        value={settings.equalizerBands[index] ?? 0}
                        onChange={(e) => {
                          const bands = [...settings.equalizerBands];
                          bands[index] = Number(e.target.value);
                          set("equalizerBands", bands);
                        }}
                        className="w-full accent-[var(--accent)]"
                        aria-label={`${frequency} Hz`}
                        title={`${frequency} Hz`}
                      />
                      <span className="text-[9px] text-text-tertiary tabular-nums">
                        {frequency >= 1000 ? `${frequency / 1000}k` : frequency}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            ) : null}
          </Section>

          <Section title={t("settings.section.advanced")}>
            <Row label={t("settings.cacheSize")} description={t("settings.cacheSize.description")}>
              <div className="w-40">
                <GlassSlider
                  min={100}
                  max={2000}
                  step={100}
                  value={settings.cacheSize}
                  onChange={setNum("cacheSize")}
                  formatTooltip={(v) => `${v} MB`}
                  aria-label={t("settings.cacheSize")}
                />
              </div>
            </Row>
          </Section>
        </div>
      </div>
    </div>
  );
}
