<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import { saveLocale, type AppLocale } from "../../shared/i18n";
import {
  FACTORY_SSH_PASSWORD,
  useDefaultSshPassword,
} from "../../shared/preferences/sshPassword";
import { useTheme, type ThemePreference } from "../../shared/theme";
import StudioInput from "../../shared/ui/StudioInput.vue";
import StudioPanel from "../../shared/ui/StudioPanel.vue";
import StudioSegmented from "../../shared/ui/StudioSegmented.vue";

const { t, locale } = useI18n();
const theme = useTheme();
const sshPassword = useDefaultSshPassword();
const sshModel = computed({
  get: () => sshPassword.stored.value,
  set: (value) => sshPassword.setDefaultSshPassword(value),
});

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
        <div class="min-w-0">
          <span class="block font-medium">{{ t("settings.ssh.title") }}</span>
          <span class="block text-xs text-muted">{{
            t("settings.ssh.hint")
          }}</span>
        </div>
        <StudioInput
          v-model="sshModel"
          type="password"
          size="sm"
          class="w-[180px] shrink-0"
          autocomplete="off"
          maxlength="1024"
          :secret="FACTORY_SSH_PASSWORD"
          :label="t('settings.ssh.title')"
        />
      </div>
      <div class="flex items-center justify-between gap-4 px-4 py-3">
        <span class="font-medium">{{ t("settings.about.version") }}</span>
        <span class="font-mono text-muted">0.1.0</span>
      </div>
    </StudioPanel>
  </div>
</template>
