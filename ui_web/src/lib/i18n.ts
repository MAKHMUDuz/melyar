import { useSyncExternalStore } from "react";
import { en } from "@/locales/en";
import { tr } from "@/locales/tr";
import { uz } from "@/locales/uz";
import { es } from "@/locales/es";
import { de } from "@/locales/de";
import { fr } from "@/locales/fr";
import { it } from "@/locales/it";
import { pt } from "@/locales/pt";
import { ru } from "@/locales/ru";
import { uk } from "@/locales/uk";
import { ja } from "@/locales/ja";
import { zh } from "@/locales/zh";
import { ko } from "@/locales/ko";
import { sq } from "@/locales/sq";

export type Locale =
  | "en"
  | "tr"
  | "uz"
  | "es"
  | "de"
  | "fr"
  | "it"
  | "pt"
  | "ru"
  | "uk"
  | "ja"
  | "zh"
  | "sq"
  | "ko";

export const LOCALES: { code: Locale; label: string }[] = [
  { code: "en", label: "English" },
  { code: "tr", label: "Türkçe" },
  { code: "uz", label: "Oʻzbekcha" },
  { code: "es", label: "Español" },
  { code: "de", label: "Deutsch" },
  { code: "fr", label: "Français" },
  { code: "it", label: "Italiano" },
  { code: "pt", label: "Português" },
  { code: "ru", label: "Русский" },
  { code: "uk", label: "Українська" },
  { code: "ja", label: "日本語" },
  { code: "zh", label: "简体中文" },
  { code: "ko", label: "한국어" },
  { code: "sq", label: "Shqip" },

];

const MESSAGES: Record<Locale, Record<string, string>> = {
  en,
  tr,
  uz,
  es,
  de,
  fr,
  it,
  pt,
  ru,
  uk,
  ja,
  zh,
  ko,
  sq,
};

const STORAGE_KEY = "melyar.locale";
const FALLBACK: Locale = "en";

function isLocale(value: unknown): value is Locale {
  return typeof value === "string" && value in MESSAGES;
}

function detect(): Locale {
  // An explicit ?lang= wins, so a launcher can force a language.
  try {
    const forced = new URLSearchParams(window.location.search).get("lang");
    if (isLocale(forced)) return forced;
  } catch {
    // No window (tests, SSR) - fall through.
  }

  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (isLocale(stored)) return stored;
  } catch {
    // Private mode or a locked-down profile - fall through to the navigator.
  }

  const nav = typeof navigator !== "undefined" ? navigator.language : "";
  const short = nav.toLowerCase().split("-")[0];
  if (isLocale(short)) return short;
  return FALLBACK;
}

let current: Locale = detect();
const listeners = new Set<() => void>();

// The audit has to know which strings the interface is allowed to show, so
// it can hold every accessible label to them instead of trusting that a
// label was translated. Only the audit reads this.
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
    // Persisting is best effort; the session still switches language.
  }
  document.documentElement.lang = locale;
  publishDict();
  emit();
}

/**
 * Translate a key. Unknown keys fall back to the key itself so a missing
 * entry is visible rather than silently blank.
 *
 * Placeholders use {name} and are replaced from `params`.
 */
export function t(key: string, params?: Record<string, string | number>): string {
  const template = MESSAGES[current]?.[key] ?? MESSAGES[FALLBACK]?.[key] ?? key;
  return interpolate(template, params);
}

/**
 * The plural form of a count for the current language.
 *
 * English, German, Spanish, French, Italian, Portuguese and Uzbek have two
 * forms. Russian and Ukrainian have three, and count differently for every
 * digit.
 */
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

/**
 * Translate a counted string, choosing the right plural form.
 *
 * The key is looked up as `<key>.one`, `<key>.few` or `<key>.many`, so the
 * wording can follow the language rather than just appending an s.
 */
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

/** Subscribe to locale changes. Re-renders the component when it happens. */
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
