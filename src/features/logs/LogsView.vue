<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import { useOperationLog } from "../../shared/composables/useOperationLog";
import { useOperations } from "../../shared/composables/useOperations";
import { notify } from "../../shared/composables/useNotifications";
import type { LogEntry, LogLevel, LogSource } from "../../shared/gateway";
import AppIcon from "../../shared/ui/AppIcon.vue";

const { t, te, d } = useI18n();
const log = useOperationLog();
const { operations } = useOperations();

const levels: LogLevel[] = ["error", "warn", "info", "debug"];
const sources: Array<LogSource | "all"> = [
  "all",
  "device",
  "preparation",
  "store",
  "system",
];
const list = ref<HTMLElement | null>(null);
const follow = ref(true);

const operationIds = computed(() => [...operations.keys()].reverse());

function render(entry: LogEntry): string {
  return te(entry.code) ? t(entry.code, entry.params ?? {}) : entry.message;
}

function levelClass(level: LogLevel): string {
  switch (level) {
    case "error":
      return "text-danger";
    case "warn":
      return "text-warning";
    case "info":
      return "text-info";
    default:
      return "text-muted";
  }
}

async function copyVisible(): Promise<void> {
  const text = log.filtered.value
    .map(
      (entry) =>
        `${new Date(entry.timestamp).toISOString()} ${entry.level.toUpperCase().padEnd(5)} [${entry.source}] ${entry.operationId ?? "-"} ${entry.message}`,
    )
    .join("\n");
  try {
    await navigator.clipboard.writeText(text);
    notify("success", "notifications.logCopied", {
      count: String(log.filtered.value.length),
    });
  } catch {
    notify("error", "notifications.logCopyFailed");
  }
}

watch(
  () => log.filtered.value.length,
  async () => {
    if (!follow.value) return;
    await nextTick();
    list.value?.scrollTo({ top: list.value.scrollHeight });
  },
);

onMounted(() => void log.initialize());
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-4">
    <header class="flex flex-wrap items-end justify-between gap-3">
      <div>
        <h1 class="text-2xl font-semibold tracking-tight">
          {{ t("logs.title") }}
        </h1>
        <p class="mt-1 text-sm text-muted">{{ t("logs.subtitle") }}</p>
      </div>
      <div class="flex gap-2">
        <button class="btn btn-secondary" @click="copyVisible">
          <AppIcon name="copy" :size="16" />
          {{ t("logs.copy") }}
        </button>
        <button
          class="btn btn-secondary"
          :disabled="log.exporting.value"
          @click="log.exportLogs"
        >
          <AppIcon name="download" :size="16" />
          {{ t("logs.export") }}
        </button>
      </div>
    </header>

    <div class="flex flex-wrap items-center gap-3 text-xs">
      <div class="flex gap-1 rounded-lg border border-line bg-surface p-1">
        <button
          v-for="level in levels"
          :key="level"
          class="rounded-md px-2.5 py-1 font-medium transition"
          :class="
            log.levelFilter.value.has(level)
              ? 'bg-ink/8'
              : 'text-muted opacity-60'
          "
          @click="log.toggleLevel(level)"
        >
          <span :class="levelClass(level)">{{ level.toUpperCase() }}</span>
        </button>
      </div>
      <select v-model="log.sourceFilter.value" class="field py-1">
        <option v-for="source in sources" :key="source" :value="source">
          {{ t(`logs.source.${source}`) }}
        </option>
      </select>
      <select v-model="log.operationFilter.value" class="field py-1 font-mono">
        <option :value="null">{{ t("logs.allOperations") }}</option>
        <option v-for="id in operationIds" :key="id" :value="id">
          {{ id }}
        </option>
      </select>
      <label class="ml-auto flex items-center gap-2 text-muted">
        <input v-model="follow" type="checkbox" class="accent-signal" />
        {{ t("logs.follow") }}
      </label>
      <span class="text-muted">{{
        t("logs.count", {
          shown: log.filtered.value.length,
          total: log.entries.value.length,
        })
      }}</span>
    </div>

    <div
      ref="list"
      class="scroll-thin card min-h-0 flex-1 overflow-y-auto font-mono text-xs"
    >
      <div
        v-if="log.filtered.value.length === 0"
        class="p-8 text-center text-muted"
      >
        {{ t("logs.empty") }}
      </div>
      <table v-else class="w-full border-collapse">
        <tbody>
          <tr
            v-for="entry in log.filtered.value"
            :key="entry.id"
            class="border-b border-line/50 align-top hover:bg-ink/4"
          >
            <td class="w-24 py-1.5 pr-3 pl-4 whitespace-nowrap text-muted">
              {{ d(entry.timestamp, "time") }}
            </td>
            <td
              class="w-14 py-1.5 pr-3 font-semibold"
              :class="levelClass(entry.level)"
            >
              {{ entry.level.toUpperCase() }}
            </td>
            <td class="w-24 py-1.5 pr-3 text-muted">{{ entry.source }}</td>
            <td class="w-20 py-1.5 pr-3 text-muted">
              {{ entry.operationId ?? "" }}
            </td>
            <td class="py-1.5 pr-4">{{ render(entry) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
