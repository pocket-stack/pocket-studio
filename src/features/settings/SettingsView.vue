<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { saveLocale, type AppLocale } from "../../shared/i18n";
import { useTheme, type ThemePreference } from "../../shared/theme";
import { useGateway } from "../../shared/gateway";

const { t, locale } = useI18n();
const theme = useTheme();
const gateway = useGateway();

const themes: ThemePreference[] = ["system", "light", "dark"];
const locales: AppLocale[] = ["zh-CN", "en"];

function chooseLocale(next: AppLocale): void {
  locale.value = next;
  saveLocale(next);
}
</script>

<template>
  <div class="mx-auto flex max-w-3xl flex-col gap-5">
    <header>
      <h1 class="text-2xl font-semibold tracking-tight">
        {{ t("settings.title") }}
      </h1>
      <p class="mt-1 text-sm text-muted">{{ t("settings.subtitle") }}</p>
    </header>

    <section class="card p-5">
      <h2 class="text-sm font-semibold">{{ t("settings.theme.title") }}</h2>
      <div class="mt-3 flex gap-2">
        <button
          v-for="option in themes"
          :key="option"
          class="btn"
          :class="
            theme.preference.value === option ? 'btn-primary' : 'btn-secondary'
          "
          @click="theme.setThemePreference(option)"
        >
          {{ t(`settings.theme.${option}`) }}
        </button>
      </div>
    </section>

    <section class="card p-5">
      <h2 class="text-sm font-semibold">{{ t("settings.language.title") }}</h2>
      <div class="mt-3 flex gap-2">
        <button
          v-for="option in locales"
          :key="option"
          class="btn"
          :class="locale === option ? 'btn-primary' : 'btn-secondary'"
          @click="chooseLocale(option)"
        >
          {{ t(`settings.language.${option}`) }}
        </button>
      </div>
    </section>

    <section class="card p-5 text-sm">
      <h2 class="text-sm font-semibold">{{ t("settings.about.title") }}</h2>
      <dl class="mt-3 grid grid-cols-[140px_1fr] gap-y-1 text-muted">
        <dt>{{ t("settings.about.version") }}</dt>
        <dd class="font-mono text-ink">0.1.0</dd>
        <dt>{{ t("settings.about.backend") }}</dt>
        <dd class="font-mono text-ink">
          {{
            gateway.flavor === "tauri"
              ? "Tauri demo driver"
              : "Browser simulation"
          }}
        </dd>
        <dt>{{ t("settings.about.toolkit") }}</dt>
        <dd class="text-ink">
          Legacy iOS Kit (Rust) — {{ t("settings.about.toolkitNote") }}
        </dd>
      </dl>
    </section>
  </div>
</template>
