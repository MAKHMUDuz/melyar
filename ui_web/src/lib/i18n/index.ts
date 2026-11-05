/// <reference types="vite/client" />
import { useSyncExternalStore } from "react";

// Manifest of available languages. Adding a language = add a folder under
// `locales/<code>/translation.json` and an entry here. No other code change.
import languagesManifest from "./locales/languages.json";

export interface LanguageEntry {
  code: string;
  title: string;
}

export const LANGUAGES: LanguageEntry[] = languagesManifest as LanguageEntry[];

// Union of supported locale codes. Keep in sync with languages.json.
export type Locale =
  | "en"
  | "tr"
  | "es"
  | "de"
  | "fr"
  | "it"
  | "pt"
  | "ru"
  | "uk"
  | "ja"
  | "zh"
  | "ko"
  | "sq"
  | "uz"
  | "uz-Cyrl";

export const LOCALES: { code: Locale; label: string }[] = LANGUAGES.map((l) => ({
  code: l.code as Locale,
  label: l.title,
}));

// Eagerly bundle every locale's translation file so the app keeps working
// when loaded from file:// (Qt WebEngine). Lazy loading would risk blocked
// dynamic imports in that environment.
const modules = import.meta.glob("./locales/*/translation.json", { eager: true }) as Record<
  string,
  { default: Record<string, string> }
>;

const MESSAGES: Record<string, Record<string, string>> = {};
for (const path of Object.keys(modules)) {
  // path: "./locales/<code>/translation.json"
  const m = path.match(/^\.\/locales\/([^/]+)\/translation\.json$/);
  if (!m) continue;
  const code = m[1];
  MESSAGES[code] = modules[path].default ?? (modules[path] as any);
}

const STORAGE_KEY = "melyar.locale";
const FALLBACK: Locale = "en";

function isLocale(value: unknown): value is Locale {
  return typeof value === "string" && value in MESSAGES;
}

// Normalize a browser language tag to one of our supported codes.
// e.g. "tr-TR" -> "tr", "uz-UZ" -> "uz", "uz-Cyrl-UZ" -> "uz-Cyrl".
function normalizeLocaleTag(tag: string): string {
  const lower = tag.toLowerCase();
  if (isLocale(lower)) return lower;
  // Try the full primary + script form (uz-cyrl).
  const parts = lower.split("-");
  if (parts.length >= 2) {
    const scriptCandidate = `${parts[0]}-${parts[1] === "cyrl" ? "Cyrl" : parts[1]}`;
    // match case-insensitively against known codes
    for (const code of Object.keys(MESSAGES)) {
      if (code.toLowerCase() === scriptCandidate.toLowerCase()) return code;
    }
  }
  const short = parts[0];
  if (isLocale(short)) return short;
  return FALLBACK;
}

function detect(): Locale {
  try {
    const forced = new URLSearchParams(window.location.search).get("lang");
    if (forced && isLocale(forced)) return forced as Locale;
    if (forced) {
      const n = normalizeLocaleTag(forced);
      if (isLocale(n)) return n as Locale;
    }
  } catch {
    // No window (tests) - fall through.
  }

  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isLocale(stored)) return stored as Locale;
  } catch {
    // Private mode - fall through.
  }

  const nav = typeof navigator !== "undefined" ? navigator.language : "";
  const normalized = normalizeLocaleTag(nav);
  if (isLocale(normalized)) return normalized as Locale;
  return FALLBACK;
}

let current: Locale = detect();
const listeners = new Set<() => void>();

function publishDict() {
  if (typeof window === "undefined") return;
  (window as unknown as Record<string, unknown>).__melyarDict = MESSAGES[current];
  (window as unknown as Record<string, unknown>).__melyarDictEn = MESSAGES[FALLBACK];
}
publishDict();

function emit() {
  for (const listener of listeners) listener();
}

export function getLocale(): Locale {
  return current;
}

export function setLocale(locale: Locale) {
  if (!isLocale(locale) || locale === current) return;
  current = locale;
  try {
    localStorage.setItem(STORAGE_KEY, locale);
  } catch {
    // Persisting is best effort.
  }
  document.documentElement.lang = locale;
  publishDict();
  emit();
}

export function t(key: string, params?: Record<string, string | number>): string {
  const template = MESSAGES[current]?.[key] ?? MESSAGES[FALLBACK]?.[key] ?? key;
  return interpolate(template, params);
}

function pluralForm(locale: Locale, n: number): "one" | "few" | "many" {
  if (n === 1) return "one";
  if (n === 0) return "many";
  if (locale === "ru" || locale === "uk") {
    const last = n % 10;
    const lastTwo = n % 100;
    if (last >= 2 && last <= 4 && !(lastTwo >= 12 && lastTwo <= 14)) return "few";
  }
  return "many";
}

export function tCount(
  key: string,
  n: number,
  params?: Record<string, string | number>,
): string {
  const form = pluralForm(current, n);
  const full = `${key}.${form}`;
  const template = MESSAGES[current]?.[full] ?? MESSAGES[FALLBACK]?.[full] ?? full;
  return interpolate(template, { n, ...params });
}

function interpolate(
  template: string,
  params?: Record<string, string | number>,
): string {
  if (!params) return template;
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    name in params ? String(params[name]) : match,
  );
}

export function useLocale(): Locale {
  return useSyncExternalStore(
    (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    getLocale,
    getLocale,
  );
}
