<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import { operationProgress } from "../../shared/composables/useOperations";
import AppIcon from "../../shared/ui/AppIcon.vue";
import ProgressBar from "../../shared/ui/ProgressBar.vue";
import StatusPill from "../../shared/ui/StatusPill.vue";
import { formatBytes } from "./compatibility";
import type { PackageView } from "./useStore";

const props = defineProps<{ item: PackageView; selected: boolean }>();
defineEmits<{ select: [] }>();
const { t } = useI18n();

const categoryGlyph: Record<string, string> = {
  runtime: "bolt",
  tool: "settings",
  app: "device",
  game: "play",
};

const badge = computed(() => {
  const { verdict, installed, operation } = props.item;
  if (operation?.status === "running")
    return { tone: "signal" as const, label: t("store.state.installing") };
  if (installed)
    return { tone: "success" as const, label: t("store.state.installed") };
  switch (verdict) {
    case "compatible":
      return { tone: "info" as const, label: t("store.state.compatible") };
    case "requiresPreparation":
      return {
        tone: "warning" as const,
        label: t("store.state.requiresPreparation"),
      };
    case "noDevice":
      return { tone: "neutral" as const, label: t("store.state.noDevice") };
    default:
      return { tone: "danger" as const, label: t("store.state.incompatible") };
  }
});
</script>

<template>
  <button
    class="card flex w-full flex-col gap-3 p-4 text-left transition hover:border-muted"
    :class="selected ? 'ring-2 ring-signal/60' : ''"
    @click="$emit('select')"
  >
    <div class="flex items-start gap-3">
      <span
        class="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl bg-signal/12 text-signal"
      >
        <AppIcon
          :name="categoryGlyph[item.entry.category] ?? 'device'"
          :size="22"
        />
      </span>
      <div class="min-w-0 flex-1">
        <div class="flex items-center gap-2">
          <h3 class="truncate text-sm font-semibold">
            {{ t(`catalog.${item.entry.id}.name`) }}
          </h3>
          <span class="font-mono text-xs text-muted"
            >v{{ item.entry.version }}</span
          >
        </div>
        <p class="truncate text-xs text-muted">{{ item.entry.developer }}</p>
      </div>
      <StatusPill :tone="badge.tone">{{ badge.label }}</StatusPill>
    </div>
    <p class="line-clamp-2 text-xs text-muted">
      {{ t(`catalog.${item.entry.id}.summary`) }}
    </p>
    <div class="flex items-center gap-2 text-[11px] text-muted">
      <span>{{ t(`store.category.${item.entry.category}`) }}</span>
      <span>·</span>
      <span>{{ formatBytes(item.entry.sizeBytes) }}</span>
      <span v-if="item.entry.compatibility.requiresJailbreak"
        >· {{ t("store.requiresJailbreak") }}</span
      >
    </div>
    <ProgressBar
      v-if="item.operation?.status === 'running'"
      :percent="operationProgress(item.operation)"
      active
      compact
    />
  </button>
</template>
