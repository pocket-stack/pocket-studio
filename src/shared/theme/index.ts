import { readonly, ref } from "vue";

const themeStorageKey = "pocket-studio.theme";

export type ThemePreference = "light" | "dark" | "system";
export type ResolvedTheme = Exclude<ThemePreference, "system">;

const preference = ref<ThemePreference>("system");
const resolvedTheme = ref<ResolvedTheme>("light");

let systemTheme: MediaQueryList;

function readPreference(): ThemePreference {
  const savedTheme = localStorage.getItem(themeStorageKey);

  if (savedTheme === "light" || savedTheme === "dark") {
    return savedTheme;
  }

  return "system";
}

function resolveTheme(): ResolvedTheme {
  if (preference.value !== "system") {
    return preference.value;
  }

  return systemTheme.matches ? "dark" : "light";
}

function applyTheme(): void {
  resolvedTheme.value = resolveTheme();
  document.documentElement.dataset.theme = resolvedTheme.value;
}

export function initializeTheme(): void {
  systemTheme = window.matchMedia("(prefers-color-scheme: dark)");
  preference.value = readPreference();
  systemTheme.addEventListener("change", applyTheme);
  applyTheme();
}

export function setThemePreference(theme: ThemePreference): void {
  preference.value = theme;
  localStorage.setItem(themeStorageKey, theme);
  applyTheme();
}

export function useTheme() {
  return {
    preference: readonly(preference),
    resolvedTheme: readonly(resolvedTheme),
    setThemePreference,
  };
}
