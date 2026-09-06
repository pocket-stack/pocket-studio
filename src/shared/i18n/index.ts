import enConnection from "./locales/en/connection.json";
import zhConnection from "./locales/zh-CN/connection.json";
import { createI18n } from "vue-i18n";

import enStudio from "./locales/en/studio.json";
import zhStudio from "./locales/zh-CN/studio.json";

import enCatalog from "./locales/en/catalog.json";
import enCommon from "./locales/en/common.json";
import enDevice from "./locales/en/device.json";
import enLogs from "./locales/en/logs.json";
import enPreparationConsent from "./locales/en/preparation-consent.json";
import enPreparationFlow from "./locales/en/preparation-flow.json";
import enPreparationSteps from "./locales/en/preparation-steps.json";
import enReadiness from "./locales/en/readiness.json";
import enStore from "./locales/en/store.json";
import zhCatalog from "./locales/zh-CN/catalog.json";
import zhCommon from "./locales/zh-CN/common.json";
import zhDevice from "./locales/zh-CN/device.json";
import zhLogs from "./locales/zh-CN/logs.json";
import zhPreparationConsent from "./locales/zh-CN/preparation-consent.json";
import zhPreparationFlow from "./locales/zh-CN/preparation-flow.json";
import zhPreparationSteps from "./locales/zh-CN/preparation-steps.json";
import zhReadiness from "./locales/zh-CN/readiness.json";
import zhStore from "./locales/zh-CN/store.json";

const localeStorageKey = "pocket-studio.locale";
const supportedLocales = ["en", "zh-CN"] as const;

export type AppLocale = (typeof supportedLocales)[number];

type MessageValue = string | string[] | MessageTree | MessageTree[];
type MessageTree = { [key: string]: MessageValue };

/** Namespace files may share a top-level key (e.g. `preparation`), so merge recursively. */
function mergeMessages(...trees: MessageTree[]): MessageTree {
  const result: MessageTree = {};
  for (const tree of trees) {
    for (const [key, value] of Object.entries(tree)) {
      const existing = result[key];
      if (isTree(existing) && isTree(value)) {
        result[key] = mergeMessages(existing, value);
      } else {
        result[key] = value;
      }
    }
  }
  return result;
}

function isTree(value: unknown): value is MessageTree {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

const messages = {
  en: mergeMessages(
    enConnection,
    enStudio,
    enCommon,
    enDevice,
    enReadiness,
    enPreparationFlow,
    enPreparationConsent,
    enPreparationSteps,
    enStore,
    enCatalog,
    enLogs,
  ),
  "zh-CN": mergeMessages(
    zhConnection,
    zhStudio,
    zhCommon,
    zhDevice,
    zhReadiness,
    zhPreparationFlow,
    zhPreparationConsent,
    zhPreparationSteps,
    zhStore,
    zhCatalog,
    zhLogs,
  ),
};

const datetimeFormats = {
  time: {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  },
  date: { year: "numeric", month: "short", day: "numeric" },
} as const;

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
    datetimeFormats: { en: datetimeFormats, "zh-CN": datetimeFormats },
    missingWarn: false,
    fallbackWarn: false,
  });
}

export function saveLocale(locale: AppLocale): void {
  localStorage.setItem(localeStorageKey, locale);
  document.documentElement.lang = locale;
}
