import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import type { UiLanguage } from "@/bindings";
import en from "@/locales/en/common.json";
import es from "@/locales/es/common.json";

export const UI_LANGUAGES = ["en", "es"] as const;
type SupportedUiLanguage = (typeof UI_LANGUAGES)[number];

const DEFAULT_UI_LANGUAGE: SupportedUiLanguage = "en";

export const resources = {
  en: { common: en },
  es: { common: es },
} as const;

/**
 * Picks the interface language: the user's choice, or the first OS language
 * we support, falling back to English.
 */
export function resolveUiLanguage(
  setting: UiLanguage,
  systemLanguages: readonly string[],
): SupportedUiLanguage {
  if (setting !== "system") return setting;
  for (const tag of systemLanguages) {
    const base = tag.toLowerCase().split("-")[0];
    const match = UI_LANGUAGES.find((lang) => lang === base);
    if (match) return match;
  }
  return DEFAULT_UI_LANGUAGE;
}

void i18n.use(initReactI18next).init({
  resources,
  lng: resolveUiLanguage("system", navigator.languages),
  fallbackLng: DEFAULT_UI_LANGUAGE,
  defaultNS: "common",
  interpolation: { escapeValue: false },
});

export default i18n;
