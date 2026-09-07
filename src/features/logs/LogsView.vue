<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useLogMessage } from "../../shared/composables/useLogMessage";
import { useOperationLog } from "../../shared/composables/useOperationLog";
import { notify } from "../../shared/composables/useNotifications";
import type { LogEntry, LogLevel, LogSource } from "../../shared/gateway";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioDialog from "../../shared/ui/StudioDialog.vue";
import StudioInput from "../../shared/ui/StudioInput.vue";
import StudioPanel from "../../shared/ui/StudioPanel.vue";
import StudioSelect from "../../shared/ui/StudioSelect.vue";

const { t, d } = useI18n();
const log = useOperationLog();
const render = useLogMessage();
const levels: LogLevel[] = ["error", "warn", "info", "debug"];
const sources: Array<LogSource | "all"> = [
  "all",
  "device",
  "preparation",
  "store",
  "system",
];
const ROW_HEIGHT = 24;
const HEAD_HEIGHT = 26;

const search = ref("");
const follow = ref(true);
const selectedId = ref<string | null>(null);
const page = ref(1);
const rowsPerPage = ref(20);
const table = ref<HTMLElement | null>(null);

const visible = computed(() =>
  log.filtered.value.filter((entry) => {
    const needle = search.value.trim().toLowerCase();
    return (
      !needle ||
      render(entry).toLowerCase().includes(needle) ||
      entry.operationId?.toLowerCase().includes(needle)
    );
  }),
);
const pageCount = computed(() =>
  Math.max(1, Math.ceil(visible.value.length / rowsPerPage.value)),
);
// Pages anchor to the newest entries so the last page is always full.
const pageEntries = computed(() => {
  const end =
    visible.value.length - (pageCount.value - page.value) * rowsPerPage.value;
  return visible.value.slice(Math.max(0, end - rowsPerPage.value), end);
});
const selected = computed(
  () =>
    log.entries.value.find((entry) => entry.id === selectedId.value) ?? null,
);
const operationIds = computed(() =>
  [
    ...new Set(
      log.entries.value.flatMap((entry) =>
        entry.operationId ? [entry.operationId] : [],
      ),
    ),
  ].reverse(),
);
const sourceCounts = computed(() =>
  sources.map((source) => ({
    source,
    count: log.entries.value.filter(
      (entry) => source === "all" || entry.source === source,
    ).length,
  })),
);

function levelClass(level: LogLevel): string {
  return {
    error: "text-danger",
    warn: "text-warning",
    info: "text-info",
    debug: "text-muted",
  }[level];
}
function entryText(entry: LogEntry): string {
  return log.serializeLogs([entry]);
}
async function copy(text: string, count: number): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    notify("success", "notifications.logCopied", { count: String(count) });
  } catch {
    notify("error", "notifications.logCopyFailed");
  }
}
function fit(): void {
  const height = table.value?.clientHeight ?? 0;
  rowsPerPage.value = Math.max(
    5,
    Math.floor((height - HEAD_HEIGHT) / ROW_HEIGHT),
  );
}
function go(next: number): void {
  page.value = Math.min(pageCount.value, Math.max(1, next));
  follow.value = page.value === pageCount.value && follow.value;
}

let observer: ResizeObserver | undefined;
onMounted(() => {
  void log.initialize();
  fit();
  if (typeof ResizeObserver !== "undefined" && table.value) {
    observer = new ResizeObserver(fit);
    observer.observe(table.value);
  }
});
onBeforeUnmount(() => observer?.disconnect());
watch([pageCount, follow], ([count, following]) => {
  if (following) page.value = count;
  else page.value = Math.min(page.value, count);
});
watch(
  [search, () => log.sourceFilter.value, () => log.operationFilter.value],
  () => {
    page.value = follow.value ? pageCount.value : 1;
  },
);
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <header class="flex items-center gap-3">
      <div class="min-w-0 flex-1">
        <h1 class="text-xl font-semibold">{{ t("logs.title") }}</h1>
        <p class="truncate text-xs text-muted">{{ t("logs.subtitle") }}</p>
      </div>
      <StudioButton @click="copy(log.serializeLogs(visible), visible.length)">
        <IconPhCopy width="14" height="14" />{{ t("logs.copy") }}
      </StudioButton>
      <StudioButton
        variant="primary"
        :loading="log.exporting.value"
        @click="log.exportLogs"
      >
        <IconPhDownloadSimple width="14" height="14" />{{ t("logs.export") }}
      </StudioButton>
    </header>

    <div class="flex items-center gap-2">
      <StudioInput
        v-model="search"
        type="search"
        class="min-w-[180px] flex-1"
        :placeholder="t('studio.logSearch')"
        :label="t('studio.logSearch')"
      >
        <template #icon
          ><IconPhMagnifyingGlass width="14" height="14"
        /></template>
      </StudioInput>
      <StudioSelect
        v-model="log.sourceFilter.value"
        :label="t('studio.logSource')"
        class="w-[128px]"
      >
        <option v-for="source in sources" :key="source" :value="source">
          {{ t(`logs.source.${source}`) }}
        </option>
      </StudioSelect>
      <StudioSelect
        v-model="log.operationFilter.value"
        :label="t('studio.logOperation')"
        class="w-[168px]"
      >
        <option :value="null">{{ t("logs.allOperations") }}</option>
        <option v-for="id in operationIds" :key="id" :value="id">
          {{ id }}
        </option>
      </StudioSelect>
      <div
        class="flex items-center gap-px rounded-control bg-ink/6 p-0.5"
        role="group"
        :aria-label="t('studio.logLevel')"
      >
        <button
          v-for="level in levels"
          :key="level"
          class="h-6 rounded-[5px] px-2 font-mono text-2xs font-semibold transition-[background-color,opacity] aria-[pressed=false]:opacity-35 aria-[pressed=true]:bg-raised aria-[pressed=true]:shadow-control"
          :class="levelClass(level)"
          :aria-pressed="log.levelFilter.value.has(level)"
          @click="log.toggleLevel(level)"
        >
          {{ level.toUpperCase() }}
        </button>
      </div>
      <label class="ml-1 flex items-center gap-1.5 text-xs text-muted"
        ><input v-model="follow" type="checkbox" />{{ t("logs.follow") }}</label
      >
    </div>

    <div class="flex min-h-0 flex-1 gap-3">
      <StudioPanel
        class="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden"
        :padded="false"
      >
        <div ref="table" class="min-h-0 flex-1 overflow-hidden">
          <table class="w-full table-fixed border-collapse text-xs">
            <colgroup>
              <col class="w-[76px]" />
              <col class="w-[52px]" />
              <col class="w-[72px]" />
              <col />
            </colgroup>
            <thead>
              <tr class="h-[26px] bg-ink/4 text-2xs font-medium text-muted">
                <th class="px-3 text-left font-medium">
                  {{ t("studio.logTime") }}
                </th>
                <th class="px-1 text-left font-medium">
                  {{ t("studio.logLevel") }}
                </th>
                <th class="px-1 text-left font-medium">
                  {{ t("studio.logSource") }}
                </th>
                <th class="px-2 text-left font-medium">
                  {{ t("studio.logMessage") }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="entry in pageEntries"
                :key="entry.id"
                class="h-6 cursor-default even:bg-ink/3 hover:bg-ink/6 aria-[selected=true]:bg-signal/12"
                :aria-selected="selectedId === entry.id"
                :data-level="entry.level"
                @click="selectedId = selectedId === entry.id ? null : entry.id"
              >
                <td
                  class="px-3 font-mono text-2xs text-muted tabular-nums whitespace-nowrap"
                >
                  {{ d(entry.timestamp, "time") }}
                </td>
                <td
                  class="px-1 font-mono text-2xs font-semibold"
                  :class="levelClass(entry.level)"
                >
                  {{ entry.level.toUpperCase() }}
                </td>
                <td class="truncate px-1 text-muted">
                  {{ t(`logs.source.${entry.source}`) }}
                </td>
                <td class="truncate px-2">{{ render(entry) }}</td>
              </tr>
            </tbody>
          </table>
          <div
            v-if="!visible.length"
            class="flex h-full items-center justify-center gap-2 text-sm text-muted"
          >
            <IconPhFileText width="18" height="18" />{{ t("logs.empty") }}
          </div>
        </div>
        <footer
          class="flex h-8 shrink-0 items-center gap-2 bg-ink/3 px-2 text-xs text-muted"
        >
          <span class="mr-auto pl-1">{{
            t("logs.count", {
              shown: visible.length,
              total: log.entries.value.length,
            })
          }}</span>
          <span class="tabular-nums">{{
            t("studio.logPage", { page, total: pageCount })
          }}</span>
          <StudioButton
            size="sm"
            variant="ghost"
            :disabled="page <= 1"
            :aria-label="t('studio.logPrevious')"
            @click="go(page - 1)"
          >
            <IconPhCaretLeft width="13" height="13" />
          </StudioButton>
          <StudioButton
            size="sm"
            variant="ghost"
            :disabled="page >= pageCount"
            :aria-label="t('studio.logNext')"
            @click="go(page + 1)"
          >
            <IconPhCaretRight width="13" height="13" />
          </StudioButton>
          <StudioButton
            v-if="!follow && page < pageCount"
            size="sm"
            variant="link"
            @click="
              follow = true;
              go(pageCount);
            "
          >
            {{ t("studio.logLatest") }}
          </StudioButton>
        </footer>
      </StudioPanel>

      <StudioPanel
        class="flex w-[264px] shrink-0 flex-col gap-3 overflow-hidden text-xs"
        as="aside"
      >
        <template v-if="selected">
          <div class="flex items-center justify-between gap-2">
            <span
              class="font-mono text-2xs font-semibold"
              :class="levelClass(selected.level)"
              >{{ selected.level.toUpperCase() }}</span
            >
            <StudioButton
              size="sm"
              variant="ghost"
              @click="copy(entryText(selected), 1)"
            >
              <IconPhCopy width="12" height="12" />{{
                t("studio.logCopyEntry")
              }}
            </StudioButton>
          </div>
          <p class="text-sm leading-[18px] text-ink">{{ render(selected) }}</p>
          <dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-2xs">
            <dt class="text-muted">{{ t("studio.logTime") }}</dt>
            <dd class="truncate font-mono">
              {{ new Date(selected.timestamp).toISOString() }}
            </dd>
            <dt class="text-muted">{{ t("studio.logSource") }}</dt>
            <dd>{{ t(`logs.source.${selected.source}`) }}</dd>
            <dt class="text-muted">{{ t("studio.logEvent") }}</dt>
            <dd class="truncate font-mono">{{ selected.id }}</dd>
            <dt class="text-muted">{{ t("studio.logOperationId") }}</dt>
            <dd class="truncate font-mono">
              {{ selected.operationId ?? "—" }}
            </dd>
            <dt class="text-muted">code</dt>
            <dd class="truncate font-mono">{{ selected.code }}</dd>
          </dl>
          <div v-if="selected.params" class="min-h-0 flex-1 overflow-hidden">
            <p class="mb-1 text-2xs text-muted">{{ t("studio.logParams") }}</p>
            <pre
              class="max-h-full overflow-hidden rounded-control bg-ink/4 p-2 font-mono text-2xs leading-[14px] whitespace-pre-wrap"
              >{{ JSON.stringify(selected.params, null, 1) }}</pre>
          </div>
        </template>
        <template v-else>
          <p class="text-muted">{{ t("studio.logInspectorEmpty") }}</p>
          <p
            class="mt-2 text-2xs font-semibold tracking-wide text-muted uppercase"
          >
            {{ t("studio.logSourceCounts") }}
          </p>
          <button
            v-for="item in sourceCounts"
            :key="item.source"
            class="flex items-center justify-between rounded-control px-2 py-1 text-left hover:bg-ink/6 aria-[pressed=true]:bg-ink/6"
            :aria-pressed="log.sourceFilter.value === item.source"
            @click="log.sourceFilter.value = item.source"
          >
            {{ t(`logs.source.${item.source}`)
            }}<span class="text-muted tabular-nums">{{ item.count }}</span>
          </button>
          <p class="mt-auto flex items-center gap-1.5 text-2xs text-muted">
            <IconPhShieldCheck width="12" height="12" />{{
              t("studio.logLocal")
            }}
          </p>
          <p
            class="text-2xs leading-[14px]"
            :class="log.storageFailed.value ? 'text-warning' : 'text-muted'"
          >
            {{
              t(
                log.storageFailed.value
                  ? "studio.logStorageFailed"
                  : "studio.logPersistent",
              )
            }}
          </p>
        </template>
      </StudioPanel>
    </div>
  </div>
  <StudioDialog
    :open="log.exportText.value !== null"
    :title="t('studio.exportPreview')"
    compact
    @close="log.exportText.value = null"
  >
    <p class="text-sm text-muted">{{ t("studio.exportDescription") }}</p>
    <pre
      class="mt-3 line-clamp-6 rounded-control bg-ink/4 p-2 font-mono text-2xs leading-[14px] whitespace-pre-wrap text-muted"
      >{{ log.exportText.value }}</pre>
    <div
      class="mt-3 flex items-center justify-between gap-3 text-xs text-muted"
    >
      <span>{{
        t("studio.exportLines", {
          count: (log.exportText.value ?? "").split("\n").length,
        })
      }}</span>
      <StudioButton
        variant="primary"
        @click="copy(log.exportText.value ?? '', log.entries.value.length)"
      >
        <IconPhCopy width="14" height="14" />{{ t("logs.copy") }}
      </StudioButton>
    </div>
  </StudioDialog>
</template>
