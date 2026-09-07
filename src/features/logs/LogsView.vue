<script setup lang="ts">
import IconPhCaretDown from "~icons/ph/caret-down";
import IconPhCaretRight from "~icons/ph/caret-right";
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useLogMessage } from "../../shared/composables/useLogMessage";
import { useOperationLog } from "../../shared/composables/useOperationLog";
import { notify } from "../../shared/composables/useNotifications";
import type { LogLevel, LogSource } from "../../shared/gateway";
import StudioDialog from "../../shared/ui/StudioDialog.vue";
const { t, d } = useI18n();
const log = useOperationLog();
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
const expandedId = ref<string | null>(null);
const search = ref("");
const visible = computed(() =>
  log.filtered.value.filter(
    (entry) =>
      render(entry).toLowerCase().includes(search.value.toLowerCase()) ||
      entry.operationId?.toLowerCase().includes(search.value.toLowerCase()),
  ),
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
const render = useLogMessage();
function levelClass(level: LogLevel): string {
  return level === "error"
    ? "text-danger"
    : level === "warn"
      ? "text-warning"
      : level === "info"
        ? "text-info"
        : "text-muted";
}
async function copy(text: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    notify("success", "notifications.logCopied", {
      count: String(visible.value.length),
    });
  } catch {
    notify("error", "notifications.logCopyFailed");
  }
}
watch(
  () => visible.value.length,
  async () => {
    if (!follow.value) return;
    await nextTick();
    list.value?.scrollTo({ top: list.value.scrollHeight });
  },
);
onMounted(() => void log.initialize());
</script>
<template>
  <div class="flex h-full gap-7">
    <div
      class="mx-auto flex h-full min-h-[470px] min-w-0 max-w-[1360px] flex-1 flex-col motion-safe:animate-rise"
    >
      <header class="flex items-center gap-5 mb-3.5 justify-end">
        <div class="first:hidden">
          <p class="text-[10px] tracking-[0.04em] text-muted">
            {{ t("studio.recentActivity") }}
          </p>
          <h1>{{ t("logs.title") }}</h1>
        </div>
        <div class="flex gap-2 first:hidden">
          <button
            class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
            @click="copy(log.serializeLogs(visible))"
          >
            <IconPhCopy width="14" height="14" />{{ t("logs.copy") }}</button
          ><button
            class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
            :disabled="log.exporting.value"
            @click="log.exportLogs"
          >
            <IconPhDownloadSimple width="14" height="14" />{{
              t("logs.export")
            }}
          </button>
        </div>
      </header>
      <div class="mb-4 flex gap-3 max-[1150px]:flex-wrap">
        <label
          class="flex flex-1 items-center gap-2 rounded-[5px] border border-line px-[11px] py-[7px] text-[11px] text-muted max-[1150px]:basis-full"
          ><IconPhMagnifyingGlass width="14" height="14" /><input
            v-model="search"
            class="w-full text-ink outline-none"
            :placeholder="t('studio.logSearch')"
            :aria-label="t('studio.logSearch')" /></label
        ><select
          v-model="log.sourceFilter.value"
          class="rounded-lg border border-line bg-raised px-3 py-2 text-sm text-ink focus:border-signal leading-[1.428571] max-w-[150px]"
          :aria-label="t('studio.logSource')"
        >
          <option v-for="source in sources" :key="source" :value="source">
            {{ t(`logs.source.${source}`) }}
          </option></select
        ><select
          v-model="log.operationFilter.value"
          class="rounded-lg border border-line bg-raised px-3 py-2 text-sm text-ink focus:border-signal leading-[1.428571] max-w-[150px]"
          :aria-label="t('studio.logOperation')"
        >
          <option :value="null">{{ t("logs.allOperations") }}</option>
          <option v-for="id in operationIds" :key="id" :value="id">
            {{ id }}
          </option>
        </select>
      </div>
      <div
        class="mb-[15px] flex items-center gap-[15px] text-[10px] text-muted"
      >
        <div class="flex gap-[5px]">
          <button
            v-for="level in levels"
            :key="level"
            class="rounded border border-line bg-surface px-2 py-[3px] text-[9px] aria-[pressed=false]:opacity-40 aria-[pressed=false]:line-through"
            :aria-pressed="log.levelFilter.value.has(level)"
            :class="levelClass(level)"
            @click="log.toggleLevel(level)"
          >
            {{ level.toUpperCase() }}
          </button>
        </div>
        <label class="ml-auto flex items-center gap-1.5"
          ><input v-model="follow" type="checkbox" class="accent-signal" />{{
            t("logs.follow")
          }}</label
        ><span class="max-[800px]:hidden">{{
          t("logs.count", {
            shown: visible.length,
            total: log.entries.value.length,
          })
        }}</span>
      </div>
      <div
        ref="list"
        class="min-h-0 flex-1 overflow-auto rounded-md border border-line [scrollbar-width:thin] [scrollbar-color:var(--color-line)_transparent]"
      >
        <table class="w-full border-collapse text-left text-[11px]">
          <thead>
            <tr class="hover:bg-surface data-[expanded=true]:bg-surface">
              <th
                class="sticky top-0 z-[1] bg-surface px-3.5 py-2.5 text-[12px] font-medium whitespace-nowrap text-muted max-[800px]:px-2.5"
              >
                {{ t("studio.logTime") }}
              </th>
              <th
                class="sticky top-0 z-[1] bg-surface px-3.5 py-2.5 text-[12px] font-medium whitespace-nowrap text-muted max-[800px]:px-2.5"
              >
                {{ t("studio.logLevel") }}
              </th>
              <th
                class="sticky top-0 z-[1] bg-surface px-3.5 py-2.5 text-[12px] font-medium whitespace-nowrap text-muted max-[800px]:px-2.5"
              >
                {{ t("studio.logSource") }}
              </th>
              <th
                class="sticky top-0 z-[1] bg-surface px-3.5 py-2.5 text-[12px] font-medium whitespace-nowrap text-muted max-[800px]:px-2.5"
              >
                {{ t("studio.logMessage") }}
              </th>
              <th
                class="sticky top-0 z-[1] bg-surface px-3.5 py-2.5 text-[12px] font-medium whitespace-nowrap text-muted max-[800px]:px-2.5"
              >
                <span class="sr-only">{{ t("studio.actions") }}</span>
              </th>
            </tr>
          </thead>
          <tbody>
            <template v-for="entry in visible" :key="entry.id"
              ><tr
                :data-expanded="expandedId === entry.id"
                class="hover:bg-surface data-[expanded=true]:bg-surface"
              >
                <td
                  class="border-t border-line px-3.5 py-2.5 text-[12px] first:w-[100px] first:font-mono first:text-[10px] first:text-muted nth-[2]:w-[70px] nth-[2]:text-[9px] nth-[3]:w-[95px] nth-[3]:text-[10px] nth-[3]:whitespace-nowrap nth-[3]:text-muted last:w-10 max-[800px]:px-2.5"
                >
                  <time :title="new Date(entry.timestamp).toISOString()">{{
                    d(entry.timestamp, "time")
                  }}</time>
                </td>
                <td
                  class="border-t border-line px-3.5 py-2.5 text-[12px] first:w-[100px] first:font-mono first:text-[10px] first:text-muted nth-[2]:w-[70px] nth-[2]:text-[9px] nth-[3]:w-[95px] nth-[3]:text-[10px] nth-[3]:whitespace-nowrap nth-[3]:text-muted last:w-10 max-[800px]:px-2.5"
                  :class="levelClass(entry.level)"
                >
                  <span
                    class="rounded-[3px] bg-current/7 px-1.5 py-[3px] font-mono"
                    >{{ entry.level.toUpperCase() }}</span
                  >
                </td>
                <td
                  class="border-t border-line px-3.5 py-2.5 text-[12px] first:w-[100px] first:font-mono first:text-[10px] first:text-muted nth-[2]:w-[70px] nth-[2]:text-[9px] nth-[3]:w-[95px] nth-[3]:text-[10px] nth-[3]:whitespace-nowrap nth-[3]:text-muted last:w-10 max-[800px]:px-2.5"
                >
                  {{ t(`logs.source.${entry.source}`) }}
                </td>
                <td
                  class="border-t border-line px-3.5 py-2.5 text-[12px] first:w-[100px] first:font-mono first:text-[10px] first:text-muted nth-[2]:w-[70px] nth-[2]:text-[9px] nth-[3]:w-[95px] nth-[3]:text-[10px] nth-[3]:whitespace-nowrap nth-[3]:text-muted last:w-10 max-[800px]:px-2.5"
                >
                  {{ render(entry) }}
                </td>
                <td
                  class="border-t border-line px-3.5 py-2.5 text-[12px] first:w-[100px] first:font-mono first:text-[10px] first:text-muted nth-[2]:w-[70px] nth-[2]:text-[9px] nth-[3]:w-[95px] nth-[3]:text-[10px] nth-[3]:whitespace-nowrap nth-[3]:text-muted last:w-10 max-[800px]:px-2.5"
                >
                  <button
                    class="inline-flex items-center justify-center rounded p-[5px] text-muted hover:bg-track hover:text-ink"
                    :aria-label="t('studio.logDetails')"
                    :aria-expanded="expandedId === entry.id"
                    @click="
                      expandedId = expandedId === entry.id ? null : entry.id
                    "
                  >
                    <component
                      :is="
                        expandedId === entry.id
                          ? IconPhCaretDown
                          : IconPhCaretRight
                      "
                      width="12"
                      height="12"
                    />
                  </button>
                </td>
              </tr>
              <tr
                v-if="expandedId === entry.id"
                class="hover:bg-surface data-[expanded=true]:bg-surface"
              >
                <td
                  colspan="5"
                  class="border-t border-line text-[12px] first:w-[100px] first:font-mono first:text-[10px] first:text-muted nth-[2]:w-[70px] nth-[2]:text-[9px] nth-[3]:w-[95px] nth-[3]:text-[10px] nth-[3]:whitespace-nowrap nth-[3]:text-muted last:w-10 bg-surface px-5 py-4 max-[800px]:px-5"
                >
                  <div class="flex flex-wrap gap-[18px] text-[10px]">
                    <span>{{ new Date(entry.timestamp).toISOString() }}</span
                    ><span>{{ entry.id }}</span
                    ><span>{{ entry.operationId ?? "—" }}</span
                    ><code>{{ entry.code }}</code>
                  </div>
                  <p class="mt-2.5 text-[11px] text-ink">{{ entry.message }}</p>
                  <pre
                    v-if="entry.params"
                    class="mt-2.5 text-[10px] whitespace-pre-wrap"
                    >{{ JSON.stringify(entry.params, null, 2) }}</pre>
                </td>
              </tr></template
            >
          </tbody>
        </table>
        <div
          v-if="!visible.length"
          class="flex min-h-[250px] items-center justify-center gap-2.5 text-[12px] text-muted"
        >
          <IconPhFileText width="24" height="24" />{{ t("logs.empty") }}
        </div>
      </div>
      <footer
        class="mt-[15px] flex items-center justify-between gap-[18px] text-[9px] text-muted max-[800px]:flex-wrap"
      >
        <span class="flex items-center gap-1.5 whitespace-nowrap"
          ><IconPhShieldCheck width="13" height="13" />{{
            t("studio.logLocal")
          }}</span
        >
        <p :class="{ 'text-warning': log.storageFailed.value }">
          {{
            t(
              log.storageFailed.value
                ? "studio.logStorageFailed"
                : "studio.logPersistent",
            )
          }}
        </p>
      </footer>
    </div>
    <aside
      class="w-[220px] shrink-0 border-l border-line pl-6 text-[12px] max-[1150px]:w-[190px] max-[700px]:hidden"
    >
      <h2 class="text-[18px] font-semibold">{{ t("logs.title") }}</h2>
      <h3 class="mt-5 mb-[7px] text-[12px] text-muted">
        {{ t("studio.logSource") }}
      </h3>
      <button
        v-for="source in sources"
        :key="source"
        :data-selected="log.sourceFilter.value === source"
        class="mb-0.5 flex w-full items-center justify-between rounded-[5px] px-2.5 py-[5px] hover:bg-track data-[selected=true]:bg-track"
        @click="log.sourceFilter.value = source"
      >
        {{ t(`logs.source.${source}`)
        }}<span class="text-muted">{{
          log.entries.value.filter(
            (entry) => source === "all" || entry.source === source,
          ).length
        }}</span>
      </button>
      <h3 class="mt-5 mb-[7px] text-[12px] text-muted">
        {{ t("studio.retention") }}
      </h3>
      <p>{{ t("studio.logLocal") }}</p>
    </aside>
  </div>
  <StudioDialog
    :open="log.exportText.value !== null"
    :title="t('studio.exportPreview')"
    @close="log.exportText.value = null"
    ><p class="text-xs text-muted mb-4">{{ t("studio.exportDescription") }}</p>
    <textarea
      class="w-full h-72 font-mono rounded-lg border border-line bg-raised px-3 py-2 text-sm text-ink focus:border-signal leading-[1.428571]"
      :aria-label="t('studio.exportPreview')"
      :value="log.exportText.value ?? ''"
      readonly
    /><button
      class="mt-4 inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
      @click="copy(log.exportText.value ?? '')"
    >
      <IconPhCopy width="14" height="14" />{{ t("logs.copy") }}
    </button></StudioDialog
  >
</template>
