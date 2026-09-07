<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import { saveLocale, type AppLocale } from "../../shared/i18n";
import { useTheme, type ThemePreference } from "../../shared/theme";
import StudioPanel from "../../shared/ui/StudioPanel.vue";
import StudioSegmented from "../../shared/ui/StudioSegmented.vue";

const { t, locale } = useI18n();
const theme = useTheme();

const themeOptions = computed(() =>
  (["system", "light", "dark"] as ThemePreference[]).map((value) => ({
    value,
    label: t(`settings.theme.${value}`),
  })),
);
const localeOptions = computed(() =>
  (["zh-CN", "en"] as AppLocale[]).map((value) => ({
    value,
    label: t(`settings.language.${value}`),
  })),
);
const themeModel = computed({
  get: () => theme.preference.value,
  set: (value) => theme.setThemePreference(value as ThemePreference),
});
const localeModel = computed({
  get: () => locale.value,
  set: (value) => {
    locale.value = value as AppLocale;
    saveLocale(value as AppLocale);
  },
});
</script>

<template>
  <div class="flex flex-col gap-3 text-sm">
    <p class="text-muted">{{ t("settings.subtitle") }}</p>
    <StudioPanel :padded="false" class="bg-canvas/60">
      <div class="flex items-center justify-between gap-4 px-4 py-3">
        <span class="font-medium">{{ t("settings.theme.title") }}</span>
        <StudioSegmented
          v-model="themeModel"
          :options="themeOptions"
          :label="t('settings.theme.title')"
        />
      </div>
      <div class="flex items-center justify-between gap-4 px-4 py-3">
        <span class="font-medium">{{ t("settings.language.title") }}</span>
        <StudioSegmented
          v-model="localeModel"
          :options="localeOptions"
          :label="t('settings.language.title')"
        />
      </div>
      <div class="flex items-center justify-between gap-4 px-4 py-3">
        <span class="font-medium">{{ t("settings.about.version") }}</span>
        <span class="font-mono text-muted">0.1.0</span>
      </div>
    </StudioPanel>
  </div>
</template>
