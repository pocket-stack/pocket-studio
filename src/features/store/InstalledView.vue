<script setup lang="ts">
import { useGateway } from "../../shared/gateway";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "./useStore";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import {
  operationProgress,
  useOperations,
} from "../../shared/composables/useOperations";
import { formatBytes } from "./compatibility";
import ProgressBar from "../../shared/ui/ProgressBar.vue";
import PackageArtwork from "./PackageArtwork.vue";
const emit = defineEmits<{ openStore: []; detail: [id: string] }>();
const { t, d } = useI18n();
const store = useStore();
const gateway = useGateway();
const { active } = useOperations();
const { device, isReady } = useDeviceSession();
const sort = ref("name");
const removingId = ref<string | null>(null);
const installed = computed(() =>
  store.allPackages.value
    .filter((item) => item.installed)
    .sort((a, b) =>
      sort.value === "recent"
        ? (b.installed?.installedAt ?? 0) - (a.installed?.installedAt ?? 0)
        : t(`catalog.${a.entry.id}.name`).localeCompare(
            t(`catalog.${b.entry.id}.name`),
          ),
    ),
);
const tasks = computed(() =>
  store.allPackages.value.filter(
    (item) =>
      item.queuePosition ||
      (item.operation && item.operation.status !== "finished"),
  ),
);
const totalSize = computed(() =>
  installed.value.reduce((sum, item) => sum + item.entry.sizeBytes, 0),
);
</script>
<template>
  <section
    v-if="!gateway.capabilities.packages"
    class="flex min-h-full flex-col items-center justify-center gap-4 p-8 text-center"
  >
    <IconStudioGrid width="32" height="32" class="text-muted" />
    <h1 class="text-xl font-semibold">{{ t("studio.installedApps") }}</h1>
    <p class="max-w-[500px] text-sm leading-7 text-muted">
      {{ t("connection.installedUnavailable") }}
    </p>
  </section>
  <div
    v-else
    class="mx-auto min-h-full max-w-[1360px] motion-safe:animate-rise"
  >
    <header class="flex items-center gap-5 mb-[15px] justify-end">
      <div class="hidden">
        <p class="text-[10px] tracking-[0.04em] text-muted">
          {{ t("studio.onDevice") }}
        </p>
        <h1>{{ t("studio.installedApps") }}</h1>
      </div>
      <button
        class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
        @click="emit('openStore')"
      >
        <IconStudioStore width="14" height="14" />{{ t("studio.browseStore") }}
      </button>
    </header>
    <section
      v-if="tasks.length"
      class="mb-[26px] rounded-lg border border-line bg-surface px-5 py-4"
    >
      <header class="flex items-center justify-between text-[12px]">
        <h2 class="font-semibold">{{ t("studio.installTasks") }}</h2>
        <span class="text-[10px] text-muted">{{
          t("studio.taskCount", { count: tasks.length })
        }}</span>
      </header>
      <article
        v-for="item in tasks"
        :key="item.entry.id"
        class="mt-[15px] flex items-center gap-3.5 border-t border-line pt-[15px]"
      >
        <PackageArtwork :package-id="item.entry.id" :size="36" />
        <div class="min-w-0 flex-1">
          <button
            class="text-xs font-medium"
            @click="emit('detail', item.entry.id)"
          >
            {{ t(`catalog.${item.entry.id}.name`) }}
          </button>
          <div class="mt-1.5 flex items-center gap-3">
            <ProgressBar
              class="max-w-56 flex-1"
              :percent="
                item.operation && !item.queuePosition
                  ? operationProgress(item.operation)
                  : 0
              "
              :tone="
                item.operation?.status === 'failed'
                  ? 'danger'
                  : item.operation?.status === 'finished'
                    ? 'success'
                    : 'signal'
              "
              :active="item.operation?.status === 'running'"
              compact
            /><span class="text-[10px] text-muted">{{
              item.queuePosition
                ? t("studio.queued", { position: item.queuePosition })
                : item.operation?.status === "running"
                  ? `${operationProgress(item.operation!)}%`
                  : t(`studio.taskStatus.${item.operation?.status}`)
            }}</span>
          </div>
        </div>
        <button
          class="inline-flex items-center gap-[5px] text-[11px] text-signal hover:underline hover:underline-offset-[3px]"
          @click="emit('detail', item.entry.id)"
        >
          {{ t("studio.viewDetails")
          }}<IconStudioChevronRight width="12" height="12" />
        </button>
      </article>
    </section>
    <div class="flex gap-[30px]">
      <section class="min-w-0 flex-1">
        <div
          class="mb-[15px] flex items-center justify-between gap-[15px] text-[12px] text-muted"
        >
          <span>{{
            t("studio.installedSummary", {
              count: installed.length,
              size: formatBytes(totalSize),
            })
          }}</span
          ><label class="flex items-center gap-2"
            >{{ t("studio.sort")
            }}<select
              v-model="sort"
              class="rounded border border-line bg-surface px-2 py-1 text-ink"
            >
              <option value="name">{{ t("studio.sortName") }}</option>
              <option value="recent">{{ t("studio.sortRecent") }}</option>
            </select></label
          >
        </div>
        <div
          v-if="!installed.length"
          class="flex min-h-[380px] flex-col items-center justify-center gap-[17px] rounded-lg border border-line p-[30px]"
        >
          <span
            class="mb-2 grid size-[75px] place-items-center rounded-[18px] border border-line bg-surface text-[#9aabbc]"
            ><IconStudioGrid width="35" height="35"
          /></span>
          <h2 class="text-[17px] font-medium">
            {{ t(device ? "studio.noInstalledTitle" : "device.empty.title") }}
          </h2>
          <p
            class="max-w-[380px] text-center text-[12px] leading-[1.8] text-muted"
          >
            {{
              t(device ? "studio.noInstalledBody" : "studio.noDeviceInstalled")
            }}
          </p>
          <button
            class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
            @click="emit('openStore')"
          >
            {{ t("studio.findApps")
            }}<IconStudioArrowRight width="14" height="14" />
          </button>
        </div>
        <div v-else class="overflow-x-auto rounded-[7px] border border-line">
          <table class="w-full border-collapse text-left text-[12px]">
            <thead class="bg-surface text-muted">
              <tr>
                <th
                  class="px-[15px] py-2.5 text-[12px] font-normal whitespace-nowrap"
                >
                  {{ t("studio.appName") }}
                </th>
                <th
                  class="px-[15px] py-2.5 text-[12px] font-normal whitespace-nowrap"
                >
                  {{ t("store.detail.version") }}
                </th>
                <th
                  class="px-[15px] py-2.5 text-[12px] font-normal whitespace-nowrap"
                >
                  {{ t("store.detail.size") }}
                </th>
                <th
                  class="px-[15px] py-2.5 text-[12px] font-normal whitespace-nowrap"
                >
                  {{ t("studio.installedAt") }}
                </th>
                <th
                  class="px-[15px] py-2.5 text-[12px] font-normal whitespace-nowrap"
                >
                  <span class="sr-only">{{ t("studio.actions") }}</span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="item in installed" :key="item.entry.id">
                <td class="border-t border-line px-3 py-2.5 whitespace-nowrap">
                  <button
                    class="flex items-center gap-3 text-left"
                    @click="emit('detail', item.entry.id)"
                  >
                    <PackageArtwork
                      :package-id="item.entry.id"
                      :size="38"
                    /><span
                      ><b class="text-[12px] font-medium">{{
                        t(`catalog.${item.entry.id}.name`)
                      }}</b
                      ><small class="mt-1 block text-[10px] text-muted">{{
                        t(`store.category.${item.entry.category}`)
                      }}</small></span
                    >
                  </button>
                </td>
                <td class="border-t border-line px-3 py-2.5 whitespace-nowrap">
                  {{ item.installed?.version }}
                </td>
                <td class="border-t border-line px-3 py-2.5 whitespace-nowrap">
                  {{ formatBytes(item.entry.sizeBytes) }}
                </td>
                <td class="border-t border-line px-3 py-2.5 whitespace-nowrap">
                  {{ d(item.installed!.installedAt, "date") }}
                </td>
                <td class="border-t border-line px-3 py-2.5 whitespace-nowrap">
                  <button
                    class="inline-flex min-h-[22px] min-w-[45px] items-center justify-center rounded-[5px] border border-[#2f6fd6] bg-[#2f6fd6] px-2.5 py-px text-[12px] text-white enabled:hover:bg-[#255cad] enabled:hover:text-white disabled:opacity-55 m-0"
                    @click="emit('detail', item.entry.id)"
                  >
                    {{ t("studio.viewDetails") }}
                  </button>
                  <button
                    v-if="item.installed?.version !== item.entry.version"
                    class="ml-2 inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
                    :disabled="
                      !!item.queuePosition ||
                      item.operation?.status === 'running'
                    "
                    @click="store.install(item.entry.id)"
                  >
                    {{ t("store.detail.update") }}
                  </button>
                  <button
                    v-if="removingId !== item.entry.id"
                    class="ml-2 inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
                    :disabled="
                      !!active.length || !!store.queuedIds.value.length
                    "
                    @click="removingId = item.entry.id"
                  >
                    {{ t("studio.uninstall") }}
                  </button>
                  <template v-else
                    ><button
                      class="ml-2 inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-danger bg-transparent text-danger enabled:hover:bg-danger/10"
                      @click="
                        store.uninstall(item.entry.id);
                        removingId = null;
                      "
                    >
                      {{ t("studio.confirmUninstall") }}</button
                    ><button
                      class="ml-2 inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
                      @click="removingId = null"
                    >
                      {{ t("common.cancel") }}
                    </button></template
                  >
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </section>
      <aside
        class="w-[220px] shrink-0 border-l border-line pl-6 max-[1150px]:w-[190px] max-[800px]:hidden"
      >
        <IconStudioDevice class="mb-3.5 text-muted" width="24" height="24" />
        <h2 class="text-[14px] font-medium">{{ t("studio.deviceName") }}</h2>
        <p class="mt-[7px] text-[12px] text-muted">
          {{ device ? t("studio.usbConnected") : t("app.noDevice") }}
        </p>
        <hr class="mx-0 my-5 border-line" />
        <span class="mt-5 block text-[10px] text-muted">{{
          t("studio.sections.environment")
        }}</span
        ><b
          class="mt-[7px] block text-[11px] font-normal"
          :class="isReady ? 'text-success' : 'text-warning'"
          >{{ t(isReady ? "studio.allReady" : "studio.needsPreparation") }}</b
        ><span class="mt-5 block text-[10px] text-muted">{{
          t("studio.sources")
        }}</span
        ><b class="mt-[7px] block text-[11px] font-normal">{{
          t("studio.demoCatalog")
        }}</b>
        <p class="mt-[7px] text-[12px] text-muted">
          {{ t("studio.installedNotice") }}
        </p>
      </aside>
    </div>
  </div>
</template>
