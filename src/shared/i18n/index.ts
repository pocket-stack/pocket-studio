import { createI18n } from "vue-i18n";

import en from "./locales/en.json";
import zhCN from "./locales/zh-CN.json";

const localeStorageKey = "pocket-studio.locale";
const supportedLocales = ["en", "zh-CN"] as const;

export type AppLocale = (typeof supportedLocales)[number];

const messages = {
  en,
  "zh-CN": zhCN,
};

function isAppLocale(locale: string): locale is AppLocale {
  return supportedLocales.some((supported) => supported === locale);
}

function resolveLocale(): AppLocale {
  const savedLocale = localStorage.getItem(localeStorageKey);

  if (savedLocale && isAppLocale(savedLocale)) {
    return savedLocale;
  }

  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

export function createAppI18n() {
  const locale = resolveLocale();
  document.documentElement.lang = locale;

  return createI18n({
    legacy: false,
    locale,
    fallbackLocale: "en",
    messages,
  });
}

export function saveLocale(locale: AppLocale): void {
  localStorage.setItem(localeStorageKey, locale);
  document.documentElement.lang = locale;
}
