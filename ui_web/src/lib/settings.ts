import { create } from "zustand";
import { getBridge } from "@/lib/client";
import { getLocale, setLocale as applyLocale, type Locale } from "@/lib/i18n";

export type Theme = "dark" | "light" | "system";
export type StreamingQuality = "low" | "medium" | "high" | "lossless";
export type LyricsFontSize = "small" | "medium" | "large" | "extraLarge";
export type LyricsAlignment = "left" | "center" | "right";

export type Settings = {
  // Appearance
  theme: Theme;
  accentColor: string;
  compactMode: boolean;
  showAlbumArt: boolean;
  animationsEnabled: boolean; // deprecated, kept for migration
  reducedMotion: boolean;
  // Playback
  crossfadeEnabled: boolean;
  crossfadeDuration: number;
  gaplessPlayback: boolean;
  normalizeVolume: boolean;
  equalizerEnabled: boolean;
  streamingQuality: StreamingQuality;
  autoPlayNext: boolean;
  showNotifications: boolean;
  showLyricsAutomatically: boolean;
  // Audio
  volume: number;
  outputDevice?: string;
  playbackSpeed: number;
  audioNormalization: boolean;
  // Library
  musicFolders: string[];
  autoScan: boolean;
  artworkCacheSize: number;
  // Lyrics
  autoFollowLyrics: boolean;
  lyricsFontSize: LyricsFontSize;
  lyricsAlignment: LyricsAlignment;
  // Advanced
  cacheSize: number;
  // Interface
  language: Locale;
  // Playback preferences that follow the user between sessions
  shuffle: boolean;
  repeat: number;
  /** The gains of the graphic equaliser, in decibels, one per band. */
  equalizerBands: number[];
};

/** The band centres of the equaliser, in hertz. Mirrors the Rust side. */
export const EQUALIZER_BAND_FREQUENCIES = [
  31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000,
];

export const DEFAULT_SETTINGS: Settings = {
  theme: "dark",
  accentColor: "#fa233b",
  compactMode: false,
  showAlbumArt: true,
  animationsEnabled: true,
  reducedMotion: false,
  crossfadeEnabled: false,
  crossfadeDuration: 5,
  gaplessPlayback: true,
  normalizeVolume: false,
  equalizerEnabled: false,
  streamingQuality: "high",
  autoPlayNext: true,
  showNotifications: true,
  showLyricsAutomatically: false,
  volume: 0.8,
  outputDevice: "default",
  playbackSpeed: 1,
  audioNormalization: false,
  musicFolders: [],
  autoScan: true,
  artworkCacheSize: 200,
  autoFollowLyrics: true,
  lyricsFontSize: "medium",
  lyricsAlignment: "center",
  cacheSize: 500,
  language: "en",
  shuffle: false,
  repeat: 0,
  equalizerBands: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
};

/** Keys that are stored as numbers or booleans need their type restored. */
const NUMERIC_KEYS = new Set<keyof Settings>([
  "crossfadeDuration",
  "cacheSize",
  "volume",
  "repeat",
  "playbackSpeed",
  "artworkCacheSize",
]);
const BOOLEAN_KEYS = new Set<keyof Settings>([
  "compactMode",
  "showAlbumArt",
  "animationsEnabled",
  "reducedMotion",
  "crossfadeEnabled",
  "gaplessPlayback",
  "normalizeVolume",
  "equalizerEnabled",
  "autoPlayNext",
  "showNotifications",
  "showLyricsAutomatically",
  "audioNormalization",
  "autoScan",
  "autoFollowLyrics",
  "shuffle",
]);

const ARRAY_KEYS = new Set<keyof Settings>(["equalizerBands", "musicFolders"]);

function coerceArray(raw: unknown, length: number): number[] {
  const values = Array.isArray(raw) ? raw : [];
  const bands: number[] = [];
  for (let i = 0; i < length; i += 1) {
    const value = Number(values[i]);
    bands.push(Number.isFinite(value) ? value : 0);
  }
  return bands;
}

function coerce(key: keyof Settings, raw: unknown): Settings[keyof Settings] {
  if (ARRAY_KEYS.has(key)) {
    const expected = (DEFAULT_SETTINGS[key] as number[]).length;
    return coerceArray(raw, expected) as Settings[keyof Settings];
  }
  if (NUMERIC_KEYS.has(key)) {
    const n = Number(raw);
    return (Number.isFinite(n) ? n : 0) as Settings[keyof Settings];
  }
  if (BOOLEAN_KEYS.has(key)) {
    if (typeof raw === "boolean") return raw as Settings[keyof Settings];
    const s = String(raw).toLowerCase();
    return (s === "true" || s === "1") as Settings[keyof Settings];
  }
  return raw as Settings[keyof Settings];
}

function persist(key: keyof Settings, value: Settings[keyof Settings]) {
  const b = getBridge();
  if (!b || typeof b.saveSetting !== "function") return;
  try {
    const stored = Array.isArray(value) ? JSON.stringify(value) : String(value);
    b.saveSetting(String(key), stored);
  } catch (e) {
    console.error("[Melyar] saveSetting failed", e);
  }
}

/**
 * Apply the equaliser to the audio engine. When it is off, an empty list
 * removes the filter chain.
 */
function applyEqualizer(settings: Settings) {
  const b = getBridge();
  if (!b || typeof b.setEqualizerBands !== "function") return;
  try {
    const bands = settings.equalizerEnabled ? settings.equalizerBands : [];
    b.setEqualizerBands(bands);
  } catch (e) {
    console.error("[Melyar] setEqualizerBands failed", e);
  }
}

/** Resolve the effective theme from a theme setting, handling "system". */
export function resolveTheme(theme: Settings["theme"]): "dark" | "light" {
  if (theme === "system") {
    if (typeof window !== "undefined" && window.matchMedia) {
      return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
    }
    return "dark";
  }
  return theme;
}

let systemThemeListener: (() => void) | null = null;

/** Set up a listener for system theme changes when theme is "system". */
export function setupSystemThemeListener(): void {
  if (typeof window === "undefined" || !window.matchMedia) return;
  if (systemThemeListener) return;

  const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = () => {
    const state = useSettings.getState();
    if (state.settings.theme === "system") {
      applySettingsToDocument(state.settings);
    }
  };

  if (mediaQuery.addEventListener) {
    mediaQuery.addEventListener("change", handler);
  } else {
    (mediaQuery as any).addListener(handler);
  }

  systemThemeListener = () => {
    if (mediaQuery.removeEventListener) {
      mediaQuery.removeEventListener("change", handler);
    } else {
      (mediaQuery as any).removeListener(handler);
    }
    systemThemeListener = null;
  };
}

/**
 * Apply the settings that are expressed in the DOM. Kept here so that the
 * store, the boot sequence and the settings panel all agree on the result.
 */
export function applySettingsToDocument(settings: Settings) {
  const root = document.documentElement;
  const effectiveTheme = resolveTheme(settings.theme);
  root.setAttribute("data-theme", effectiveTheme);
  root.setAttribute("data-accent", settings.accentColor);
  root.dataset.compact = settings.compactMode ? "true" : "false";
  root.dataset.animations = settings.animationsEnabled ? "true" : "false";
  root.dataset.reducedMotion = settings.reducedMotion ? "true" : "false";

  // The accent colour drives a handful of custom properties.
  //
  // The alpha steps are computed here rather than with color-mix() in the
  // stylesheet. color-mix() only exists from Chromium 111, and Qt WebEngine
  // on Qt 6.4 — which is what Ubuntu 24.04 ships — is older than that. When
  // the declaration is dropped there, what remains is Tailwind's fallback for
  // a runtime variable, which is the solid colour: a twenty percent tint
  // becomes a solid block. Computing the steps here gives every engine the
  // same value.
  const accent = settings.accentColor;
  root.style.setProperty("--accent", accent);
  root.style.setProperty("--accent-soft", hexToRgba(accent, 0.16));
  root.style.setProperty("--on-accent", readableTextColor(accent));
  for (const step of ACCENT_STEPS) {
    root.style.setProperty(`--accent-${step}`, hexToRgba(accent, step / 100));
  }
  // The colour used when the accent is written as text. It lands on the
  // accent's own tints in the dark theme and on white in the light one, and
  // an accent that is readable on one is not necessarily readable on the
  // other, so it is mixed towards white or black until it clears WCAG AA
  // against both. An accent that already clears it is left as it is.
  //
  // The dark variant is set as well, because the full screen player keeps a
  // dark surface in both themes and reads that one instead.
  root.style.setProperty("--accent-text", accentTextColour(accent, settings.theme));
  root.style.setProperty("--accent-text-dark", accentTextColour(accent, "dark"));
  // The gradient behind artwork that has none. It mixes the accent with the
  // two darks it sits on, so it follows the accent like everything else.
  root.style.setProperty("--cover-start", mixHex(accent, "#2a2a2e", 0.45));
  root.style.setProperty("--cover-end", mixHex(accent, "#151518", 0.15));
}

/** The alpha steps of the accent that the interface asks for by name. */
const ACCENT_STEPS = [10, 15, 20, 25, 30, 35, 40, 45, 50];

function hexToRgba(hex: string, alpha: number): string {
  const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex.trim());
  if (!m) return `rgba(250, 35, 59, ${alpha})`;
  const r = parseInt(m[1], 16);
  const g = parseInt(m[2], 16);
  const b = parseInt(m[3], 16);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

/** Relative luminance of a colour, as WCAG defines it. */
function relativeLuminance(hex: string): number {
  const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex.trim());
  if (!m) return 0;
  const channel = (value: string) => {
    const c = parseInt(value, 16) / 255;
    return c <= 0.03928 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
  };
  return 0.2126 * channel(m[1]) + 0.7152 * channel(m[2]) + 0.0722 * channel(m[3]);
}

/**
 * The accent as text: readable where it is used, which is over the accent's
 * own tints in the dark theme and over white in the light one.
 *
 * The two directions are opposite. Over a near-black tint the accent has to
 * be lighter than itself; over white it has to be darker, and a pale accent
 * may have to become quite dark. The surfaces are built from the accent
 * itself — the plain panel, then the accent over it at the ten, sixteen and
 * twenty percent the interface uses — so this holds whichever colour is
 * chosen, not only the default one. It mixes until every one of them clears
 * 4.6:1, the floor the audit holds text to.
 *
 * The swatches, the play button and the tints keep the accent itself; only
 * this changes.
 */
function accentTextColour(hex: string, theme: string): string {
  const dark = theme !== "light";
  const base = dark ? "#131317" : "#fafafc";
  const surfaces = [0, 0.1, 0.16, 0.2].map((tint) => mixHex(hex, base, tint));
  if (!dark) surfaces.push("#ffffff");
  const towards = dark ? "#ffffff" : "#000000";
  for (let step = 0; step <= 100; step += 1) {
    const candidate = mixHex(hex, towards, 1 - step / 100);
    if (surfaces.every((s) => contrastWith(candidate, s) >= 4.6)) return candidate;
  }
  return towards;
}

/** Contrast ratio between two colours, per WCAG. */
function contrastWith(first: string, second: string): number {
  const a = relativeLuminance(first);
  const b = relativeLuminance(second);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

/**
 * Mix two opaque colours in sRGB. color-mix() would do this in oklab, which
 * differs slightly, but the difference is not visible in a gradient and this
 * runs on engines that do not have color-mix() at all.
 */
function mixHex(first: string, second: string, firstShare: number): string {
  const channels = (hex: string) => {
    const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex.trim());
    return m ? m.slice(1, 4).map((v) => parseInt(v, 16)) : [42, 42, 46];
  };
  const a = channels(first);
  const b = channels(second);
  return `#${a
    .map((v, i) =>
      Math.round(v * firstShare + b[i] * (1 - firstShare))
        .toString(16)
        .padStart(2, "0"),
    )
    .join("")}`;
}

function readableTextColor(hex: string): string {
  const m = /^#?([a-f\d]{2})([a-f\d]{2})([a-f\d]{2})$/i.exec(hex.trim());
  if (!m) return "#ffffff";
  const r = parseInt(m[1], 16);
  const g = parseInt(m[2], 16);
  const b = parseInt(m[3], 16);
  // Rec. 709 luma
  const luma = (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255;
  return luma > 0.6 ? "#101014" : "#ffffff";
}

/**
 * The equaliser is the one control that is dragged rather than clicked, so it
 * gets a trailing debounce. The engine is read back from the store when the
 * timer fires, not captured: a toggle that lands while a write is pending
 * must win over the snapshot the drag started from.
 */
let equalizerTimer: ReturnType<typeof setTimeout> | null = null;

function scheduleEqualizer(_snapshot: Settings) {
  if (equalizerTimer) clearTimeout(equalizerTimer);
  equalizerTimer = setTimeout(() => {
    equalizerTimer = null;
    applyEqualizer(useSettings.getState().settings);
  }, 80);
}

function cancelScheduledEqualizer() {
  if (equalizerTimer) {
    clearTimeout(equalizerTimer);
    equalizerTimer = null;
  }
  for (const timer of persistTimers.values()) clearTimeout(timer);
  persistTimers.clear();
}

const persistTimers = new Map<string, ReturnType<typeof setTimeout>>();

/** Coalesce the disk writes of a dragging control into one round trip. */
function schedulePersist(
  key: keyof Settings,
  value: Settings[keyof Settings],
  delay = 350,
) {
  const name = String(key);
  const existing = persistTimers.get(name);
  if (existing) clearTimeout(existing);
  persistTimers.set(
    name,
    setTimeout(() => {
      persistTimers.delete(name);
      persist(key, value);
    }, delay),
  );
}

type SettingsState = {
  settings: Settings;
  loaded: boolean;
  open: boolean;
  load: () => Promise<void>;
  set: <K extends keyof Settings>(key: K, value: Settings[K]) => void;
  reset: () => void;
  setOpen: (open: boolean) => void;
};

export const useSettings = create<SettingsState>()((set, get) => ({
  settings: { ...DEFAULT_SETTINGS },
  loaded: false,
  open: false,

  load: () => {
    return (async () => {
      const b = getBridge();
      let merged: Settings = { ...DEFAULT_SETTINGS };

      if (b && typeof b.loadSettings === "function") {
        try {
          // QWebChannel invocations are asynchronous: the method returns a
          // promise that resolves with the actual return value.
          const raw = await b.loadSettings();
          const parsed = raw ? JSON.parse(raw) : {};
          if (parsed && typeof parsed === "object") {
            for (const key of Object.keys(DEFAULT_SETTINGS) as (keyof Settings)[]) {
              if (key in parsed) {
                (merged as Record<string, unknown>)[key] = coerce(key, parsed[key]);
              }
            }
          }
        } catch (e) {
          console.error("[Melyar] loadSettings failed", e);
        }
      }

      // The language lives in the i18n layer, which owns its own persistence.
      merged.language = getLocale();

      applySettingsToDocument(merged);
      applyEqualizer(merged);
      set({ settings: merged, loaded: true });
      setupSystemThemeListener();
    })();
  },
  set: (key, value) => {
    const settings = { ...get().settings, [key]: value };
    set({ settings });

    // The band gains do not reach the DOM, and a drag fires this on every
    // pointer move - recomputing the theme and the accent colours here would
    // be pure waste on the hot path.
    if (key !== "equalizerBands") {
      applySettingsToDocument(settings);
    }

    if (key === "language") {
      applyLocale(value as Locale);
    }

    if (key === "equalizerEnabled") {
      // Toggling is a single, deliberate action: apply it at once so the
      // change is audible immediately, and drop any pending band write.
      cancelScheduledEqualizer();
      applyEqualizer(settings);
      persist(key, value);
    } else if (key === "equalizerBands") {
      // A drag sends a burst of these. Apply the newest state on a short
      // trailing delay so mpv is not rebuilt on every pixel, and persist
      // even less often because a setting write is a round trip.
      scheduleEqualizer(settings);
      schedulePersist(key, value);
    } else {
      persist(key, value);
    }
  },

  reset: () => {
    cancelScheduledEqualizer();
    const settings = { ...DEFAULT_SETTINGS, language: getLocale() };
    applySettingsToDocument(settings);
    applyEqualizer(settings);
    set({ settings });
    applyLocale(settings.language);
    for (const key of Object.keys(DEFAULT_SETTINGS) as (keyof Settings)[]) {
      persist(key, settings[key]);
    }
  },

  setOpen: (open) => set({ open }),
}));