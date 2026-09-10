<script setup lang="ts">
import { useGateway } from "../../shared/gateway";
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "./useStore";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { useElementSize } from "../../shared/composables/useElementSize";
import {
  operationProgress,
  operationStepCeiling,
  useOperations,
} from "../../shared/composables/useOperations";
import { formatBytes } from "./compatibility";
import ProgressBar from "../../shared/ui/ProgressBar.vue";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioPanel from "../../shared/ui/StudioPanel.vue";
import StudioSelect from "../../shared/ui/StudioSelect.vue";
import PackageArtwork from "./PackageArtwork.vue";
import { packageText } from "./packageContent";

const ROW_HEIGHT = 44;
const HEAD_HEIGHT = 28;

const emit = defineEmits<{ openStore: []; detail: [id: string] }>();
const { t, locale } = useI18n();
const store = useStore();
const gateway = useGateway();
const { active } = useOperations();
const { device } = useDeviceSession();
const sort = ref("name");
const removingId = ref<string | null>(null);
const page = ref(0);
const table = ref<HTMLElement | null>(null);
const { height } = useElementSize(table);
const rowsPerPage = computed(() =>
  Math.max(
    1,
    Math.floor(
      (height.value - HEAD_HEIGHT) /
        (device.value?.platform === "3ds" ? 92 : ROW_HEIGHT),
    ),
  ),
);
const installed = computed(() =>
  store.installed.value
    .flatMap((record) => {
      const item = store.allPackages.value.find(
        (item) => item.entry.id === record.packageId,
      );
      return item ? [{ ...item, installed: record }] : [];
    })
    .sort((a, b) =>
      sort.value === "recent"
        ? (b.installed?.installedAt ?? 0) - (a.installed?.installedAt ?? 0)
        : packageText(a.entry, "name", locale.value, t).localeCompare(
            packageText(b.entry, "name", locale.value, t),
          ),
    ),
);
const pageCount = computed(() =>
  Math.max(1, Math.ceil(installed.value.length / rowsPerPage.value)),
);
const pageRows = computed(() =>
  installed.value.slice(
    page.value * rowsPerPage.value,
    (page.value + 1) * rowsPerPage.value,
  ),
);
watch(pageCount, (count) => {
  page.value = Math.min(page.value, count - 1);
});
const tasks = computed(() =>
  store.allPackages.value
    .filter(
      (item) =>
        item.queuePosition ||
        (item.operation && item.operation.status !== "finished"),
    )
    .slice(0, 3),
);
</script>
<template>
  <section
    v-if="!gateway.capabilities.installed"
    class="flex h-full flex-col items-center justify-center gap-3 text-center"
  >
    <IconPhSquaresFour width="28" height="28" class="text-muted" />
    <h1 class="text-xl font-semibold">{{ t("studio.installedApps") }}</h1>
    <p class="max-w-[460px] text-sm text-muted">
      {{ t("connection.installedUnavailable") }}
    </p>
  </section>
  <div
    v-else
    class="flex h-full min-h-0 flex-col gap-3 motion-safe:animate-rise"
  >
    <header class="flex items-center gap-3">
      <div class="min-w-0 flex-1">
        <h1 class="text-xl font-semibold">{{ t("studio.installedApps") }}</h1>
      </div>
      <label class="flex items-center gap-1.5 text-xs text-muted"
        >{{ t("studio.sort")
        }}<StudioSelect v-model="sort" size="sm" :label="t('studio.sort')">
          <option value="name">{{ t("studio.sortName") }}</option>
          <option v-if="gateway.flavor === 'browser'" value="recent">
            {{ t("studio.sortRecent") }}
          </option>
        </StudioSelect></label
      >
      <StudioButton
        size="sm"
        variant="ghost"
        :disabled="!device"
        :loading="store.installedLoading.value"
        @click="store.refreshInstalled()"
      >
        <IconPhArrowsClockwise
          v-if="!store.installedLoading.value"
          width="13"
          height="13"
        />{{ t("store.installed.refresh") }}
      </StudioButton>
      <StudioButton size="sm" variant="primary" @click="emit('openStore')">
        <IconPhStorefront width="13" height="13" />{{ t("studio.browseStore") }}
      </StudioButton>
    </header>
    <StudioCallout v-if="store.installedIssue.value" tone="warning">
      {{ t("store.installed.readFailed") }}
    </StudioCallout>
    <StudioPanel
      v-if="gateway.flavor === 'browser' && tasks.length"
      :padded="false"
      class="px-4 py-2"
    >
      <div class="flex items-center justify-between text-xs">
        <h2 class="font-semibold">{{ t("studio.installTasks") }}</h2>
        <span class="text-2xs text-muted">{{
          t("studio.taskCount", { count: tasks.length })
        }}</span>
      </div>
      <div
        v-for="item in tasks"
        :key="item.entry.id"
        class="mt-1.5 flex items-center gap-3"
      >
        <PackageArtwork
          :entry="item.entry"
          :package-id="item.entry.id"
          :size="24"
          class="rounded-[18%]"
        />
        <button
          class="w-[160px] truncate text-left text-xs font-medium hover:text-signal"
          @click="emit('detail', item.entry.id)"
        >
          {{ packageText(item.entry, "name", locale, t) }}
        </button>
        <ProgressBar
          class="max-w-56 flex-1"
          :percent="
            item.operation && !item.queuePosition
              ? operationProgress(item.operation)
              : 0
          "
          :trickle-to="
            item.operation ? operationStepCeiling(item.operation) : 0
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
        />
        <span class="w-16 text-2xs text-muted tabular-nums">{{
          item.queuePosition
            ? t("studio.queued", { position: item.queuePosition })
            : item.operation?.status === "running"
              ? `${operationProgress(item.operation!)}%`
              : t(`studio.taskStatus.${item.operation?.status}`)
        }}</span>
        <StudioButton
          size="sm"
          variant="link"
          class="ml-auto"
          @click="emit('detail', item.entry.id)"
        >
          {{ t("studio.viewDetails")
          }}<IconPhCaretRight width="11" height="11" />
        </StudioButton>
      </div>
    </StudioPanel>
    <StudioPanel
      :padded="false"
      class="flex min-h-0 flex-1 flex-col overflow-hidden"
    >
      <div
        v-if="!installed.length"
        class="flex flex-1 flex-col items-center justify-center gap-3 p-6 text-center"
      >
        <span
          class="grid size-14 place-items-center rounded-[18px] bg-ink/5 text-muted"
          ><IconPhSquaresFour width="26" height="26"
        /></span>
        <h2 class="text-lg font-semibold">
          {{
            t(
              !device
                ? "device.empty.title"
                : store.installedLoading.value
                  ? "store.installed.loading"
                  : store.installedIssue.value
                    ? "store.installed.unavailable"
                    : "studio.noInstalledTitle",
            )
          }}
        </h2>
        <p class="max-w-[380px] text-sm text-muted">
          {{
            t(
              !device
                ? "studio.noDeviceInstalled"
                : store.installedIssue.value
                  ? "store.installed.readFailed"
                  : "studio.noInstalledBody",
            )
          }}
        </p>
        <StudioButton variant="primary" @click="emit('openStore')">
          {{ t("studio.findApps") }}<IconPhArrowRight width="14" height="14" />
        </StudioButton>
      </div>
      <template v-else>
        <div ref="table" class="min-h-0 flex-1 overflow-hidden">
          <table class="w-full table-fixed border-collapse text-left text-sm">
            <colgroup>
              <col />
              <col
                :class="device?.platform === '3ds' ? 'w-[220px]' : 'w-[150px]'"
              />
              <col v-if="device?.platform !== '3ds'" class="w-[90px]" />
              <col v-if="device?.platform !== '3ds'" class="w-[110px]" />
              <col
                :class="device?.platform === '3ds' ? 'w-[240px]' : 'w-[300px]'"
              />
            </colgroup>
            <thead>
              <tr class="h-7 bg-ink/4 text-2xs font-medium text-muted">
                <th class="px-4 font-medium">{{ t("studio.appName") }}</th>
                <th class="px-2 font-medium">
                  {{ t("store.detail.version") }}
                </th>
                <th v-if="device?.platform !== '3ds'" class="px-2 font-medium">
                  {{ t("store.installed.packageSize") }}
                </th>
                <th v-if="device?.platform !== '3ds'" class="px-2 font-medium">
                  {{ t("store.installed.revision") }}
                </th>
                <th class="px-2 font-medium">
                  <span class="sr-only">{{ t("studio.actions") }}</span>
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="item in pageRows"
                :key="item.installed.installationId"
                class="even:bg-ink/3 hover:bg-ink/6"
                :class="item.installed.managed ? 'h-[92px]' : 'h-11'"
              >
                <td class="px-4">
                  <button
                    class="flex min-w-0 items-center gap-2.5 text-left"
                    @click="emit('detail', item.entry.id)"
                  >
                    <PackageArtwork
                      :entry="item.entry"
                      :package-id="item.entry.id"
                      :size="28"
                      class="rounded-[18%]"
                    /><span class="min-w-0"
                      ><b class="block truncate font-medium">{{
                        packageText(item.entry, "name", locale, t)
                      }}</b
                      ><small class="block truncate text-2xs text-muted">{{
                        t(`store.category.${item.entry.category}`)
                      }}</small></span
                    >
                  </button>
                </td>
                <td class="px-2 text-xs">
                  {{
                    item.installed?.version ||
                    t("store.installed.unknownVersion")
                  }}<small
                    v-if="item.installed?.native?.buildNumber"
                    class="block text-2xs text-muted"
                    >{{
                      t("store.installed.build", {
                        value: item.installed.native.buildNumber,
                      })
                    }}</small
                  >
                  <div
                    v-if="item.installed.managed"
                    class="mt-1 text-2xs text-muted"
                  >
                    {{
                      t(`threeDs.delivery.${item.installed.managed.delivery}`)
                    }}
                    · {{ item.installed.managed.format.toUpperCase() }} <br />{{
                      t("threeDs.nativeVersion")
                    }}: {{ item.installed.managed.nativeVersion ?? "—" }} ·
                    Runtime {{ item.installed.managed.runtimeVersion ?? "—" }}
                    <br />{{
                      t(
                        item.installed.managed.unavailable
                          ? "threeDs.unavailable"
                          : `threeDs.health.${item.installed.managed.health}`,
                      )
                    }}
                  </div>
                </td>
                <td v-if="device?.platform !== '3ds'" class="px-2 text-xs">
                  {{ formatBytes(item.entry.sizeBytes) }}
                </td>
                <td v-if="device?.platform !== '3ds'" class="px-2 text-xs">
                  {{
                    item.installed?.revision ??
                    t("store.installed.unknownRevision")
                  }}
                </td>
                <td class="px-2">
                  <div class="flex flex-wrap items-center justify-end gap-1.5">
                    <StudioButton
                      v-if="removingId !== item.installed.installationId"
                      size="sm"
                      variant="link"
                      @click="emit('detail', item.entry.id)"
                    >
                      {{ t("studio.viewDetails") }}
                    </StudioButton>
                    <StudioButton
                      v-if="
                        removingId !== item.installed.installationId &&
                        gateway.capabilities.packages &&
                        (gateway.flavor === 'tauri' ||
                          device?.platform === '3ds' ||
                          item.installed?.version !== item.entry.version)
                      "
                      size="sm"
                      :disabled="
                        !!item.queuePosition ||
                        item.operation?.status === 'running' ||
                        item.pending
                      "
                      :loading="item.pending"
                      @click="
                        store.install(
                          item.entry.id,
                          item.installed.installationId,
                        )
                      "
                    >
                      {{
                        t(
                          item.installed.managed
                            ? item.installed.releaseId ===
                              item.entry.details?.releaseId
                              ? "store.detail.reinstall"
                              : "store.detail.update"
                            : gateway.flavor === "tauri" &&
                                (!item.installed.revision ||
                                  item.installed.artifactId ===
                                    item.entry.details?.artifactId)
                              ? "store.detail.reinstall"
                              : "store.detail.update",
                        )
                      }}
                    </StudioButton>
                    <StudioButton
                      v-if="
                        gateway.capabilities.packages &&
                        removingId !== item.installed.installationId
                      "
                      size="sm"
                      variant="ghost"
                      :disabled="
                        !!active.length ||
                        !!store.queuedIds.value.length ||
                        item.pending
                      "
                      @click="removingId = item.installed.installationId"
                    >
                      {{ t("studio.uninstall") }}
                    </StudioButton>
                    <template v-else-if="gateway.capabilities.packages">
                      <span
                        class="text-2xs"
                        :class="
                          item.installed.managed ? 'text-muted' : 'text-danger'
                        "
                        >{{
                          t(
                            item.installed.managed
                              ? "threeDs.keepsData"
                              : "store.actions.deleteHint",
                          )
                        }}</span
                      >
                      <StudioButton
                        size="sm"
                        variant="danger"
                        @click="
                          store.uninstall(
                            item.entry.id,
                            item.installed.installationId,
                          );
                          removingId = null;
                        "
                      >
                        {{ t("studio.confirmUninstall") }}
                      </StudioButton>
                      <StudioButton
                        v-if="item.installed.managed"
                        size="sm"
                        variant="danger"
                        @click="
                          store.uninstall(
                            item.entry.id,
                            item.installed.installationId,
                            true,
                          );
                          removingId = null;
                        "
                      >
                        {{ t("threeDs.removeWithData") }}
                      </StudioButton>
                      <StudioButton
                        size="sm"
                        variant="ghost"
                        @click="removingId = null"
                      >
                        {{ t("common.cancel") }}
                      </StudioButton>
                    </template>
                  </div>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <footer
          v-if="pageCount > 1"
          class="flex h-8 shrink-0 items-center justify-end gap-1 bg-ink/3 px-2 text-xs text-muted"
        >
          <span class="tabular-nums">{{
            t("common.pageOf", { page: page + 1, total: pageCount })
          }}</span>
          <StudioButton
            size="sm"
            variant="ghost"
            :disabled="page <= 0"
            :aria-label="t('common.previous')"
            @click="page -= 1"
          >
            <IconPhCaretLeft width="13" height="13" />
          </StudioButton>
          <StudioButton
            size="sm"
            variant="ghost"
            :disabled="page >= pageCount - 1"
            :aria-label="t('common.next')"
            @click="page += 1"
          >
            <IconPhCaretRight width="13" height="13" />
          </StudioButton>
        </footer>
      </template>
    </StudioPanel>
  </div>
</template>
