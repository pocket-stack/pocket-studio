<script setup lang="ts">
import IconStudioDevice from "~icons/studio/device";
import IconStudioGrid from "~icons/studio/grid";
import IconStudioLogs from "~icons/studio/logs";
import IconStudioStore from "~icons/studio/store";
import IconStudioChevronDown from "~icons/studio/chevron-down";
import IconStudioChevronUp from "~icons/studio/chevron-up";
import { computed, nextTick, onMounted, ref, watch, type Component } from "vue";
import { useI18n } from "vue-i18n";
import {
  createStudioNavigation,
  type DeviceSection,
  type StudioView,
} from "./app/useStudioNavigation";
import DemoPanel from "./app/DemoPanel.vue";
import NotificationStack from "./app/NotificationStack.vue";
import DeviceView from "./features/device/DeviceView.vue";
import LogsView from "./features/logs/LogsView.vue";
import PreparationWizard from "./features/preparation/PreparationWizard.vue";
import { usePreparation } from "./features/preparation/usePreparation";
import SettingsView from "./features/settings/SettingsView.vue";
import StoreView from "./features/store/StoreView.vue";
import InstalledView from "./features/store/InstalledView.vue";
import { useStore } from "./features/store/useStore";
import { useDeviceSession } from "./shared/composables/useDeviceSession";
import { useLogMessage } from "./shared/composables/useLogMessage";
import { useOperationLog } from "./shared/composables/useOperationLog";
import {
  operationProgress,
  useOperations,
} from "./shared/composables/useOperations";
import { useGateway } from "./shared/gateway";
import StudioDialog from "./shared/ui/StudioDialog.vue";

const { t, d } = useI18n();
const renderLog = useLogMessage();
const lastDeviceSection = ref<DeviceSection>("summary");
const mainContent = ref<HTMLElement | null>(null);
const demoOpen = ref(false);
const settingsOpen = ref(false);
const deviceMenu = ref<HTMLDetailsElement | null>(null);
const logsExpanded = ref(false);
const session = useDeviceSession();
const preparation = usePreparation();
const store = useStore();
const log = useOperationLog();
const gateway = useGateway();
const { active } = useOperations();
const current = computed(() => active.value[0]);
const preparing = computed(
  () =>
    ["starting", "awaitingDfu", "running"].includes(preparation.stage.value) ||
    active.value.some((operation) => operation.kind === "preparation"),
);
const latestLog = computed(() => log.entries.value.at(-1));
const latestMessage = computed(() => {
  const entry = latestLog.value;
  return entry ? renderLog(entry) : t("studio.logIdle");
});
const progress = computed(() =>
  current.value ? operationProgress(current.value) : 100,
);
const lcdTitle = computed(() => {
  const operation = current.value;
  if (operation)
    return operation.kind === "preparation"
      ? t("studio.preparing")
      : t("studio.installing", {
          name: t(`catalog.${operation.subject}.name`),
        });
  return session.device.value
    ? session.device.value.marketingName
    : t("device.empty.title");
});
const lcdSubtitle = computed(() => {
  const operation = current.value;
  if (operation?.currentStepId)
    return t(
      `${operation.kind === "preparation" ? "preparation.steps" : "store.steps"}.${operation.currentStepId}.title`,
    );
  return session.checking.value
    ? t("readiness.checking")
    : session.device.value
      ? t(
          `readiness.status.${session.readiness.value?.status ?? "needsAttention"}`,
        )
      : t("studio.connectHint");
});
const modalOpen = computed(
  () =>
    preparation.consentVisible.value || settingsOpen.value || demoOpen.value,
);
const navigationHistory = createStudioNavigation(
  () => !preparing.value && !modalOpen.value,
);
const view = computed(() => navigationHistory.current.value.view);
const deviceSection = computed(() =>
  navigationHistory.current.value.view === "device"
    ? navigationHistory.current.value.section
    : lastDeviceSection.value,
);
watch(navigationHistory.current, async (location) => {
  if (location.view === "device") lastDeviceSection.value = location.section;
  if (location.view === "store") store.select(location.packageId);
  await nextTick();
  mainContent.value?.scrollTo({ top: 0 });
});
const navigation: Array<{ id: StudioView; icon: Component }> = [
  { id: "device", icon: IconStudioDevice },
  { id: "installed", icon: IconStudioGrid },
  { id: "logs", icon: IconStudioLogs },
  { id: "store", icon: IconStudioStore },
];
function showDevice(section: DeviceSection): void {
  navigationHistory.navigate({ view: "device", section });
}
function showPage(page: StudioView): void {
  if (page === "device") showDevice(lastDeviceSection.value);
  else if (page === "store") showPackage(store.selectedId.value);
  else navigationHistory.navigate({ view: page });
}
function showPackage(packageId: string | null): void {
  navigationHistory.navigate({ view: "store", packageId });
}
function startPreparation(): void {
  if (session.device.value && gateway.capabilities.preparation) {
    showDevice("environment");
    void preparation.open(session.device.value.id);
  }
}
function showActivity(): void {
  if (current.value?.kind === "preparation") showDevice("environment");
  else if (current.value?.subject) showPackage(current.value.subject);
  else showDevice("summary");
}
async function detect(): Promise<void> {
  if (deviceMenu.value) deviceMenu.value.open = false;
  if (gateway.capabilities.demo && !session.device.value)
    await gateway.demo.attachDevice();
  else await session.refresh();
}
async function eject(): Promise<void> {
  if (active.value.length || !gateway.capabilities.demo) return;
  if (deviceMenu.value) deviceMenu.value.open = false;
  await gateway.demo.detachDevice();
}
function search(): void {
  if (preparing.value) return;
  showPackage(null);
}
watch(preparation.stage, (stage) => {
  if (stage === "overview" || stage === "starting") showDevice("environment");
});
onMounted(async () => {
  await Promise.allSettled([
    log.initialize(),
    session.initialize(),
    store.initialize(),
  ]);
  if (gateway.capabilities.demo && !session.device.value)
    await gateway.demo.attachDevice();
});
</script>

<template>
  <div class="flex h-dvh flex-col" :inert="modalOpen">
    <header
      class="flex h-[52px] shrink-0 items-center justify-between gap-3 border-b border-line bg-chrome px-4 max-[600px]:gap-2 max-[600px]:px-2.5"
    >
      <div
        class="flex shrink-0 items-center gap-[9px] max-[800px]:gap-1.5 max-[600px]:gap-1"
      >
        <div
          class="flex items-center gap-0.5"
          role="group"
          :aria-label="t('studio.historyNavigation')"
        >
          <button
            class="relative inline-flex size-[30px] items-center justify-center rounded border border-transparent bg-transparent p-0 text-muted transition-[background-color,color] duration-150 enabled:hover:bg-track enabled:hover:text-ink disabled:cursor-not-allowed disabled:opacity-35 aria-[current=page]:border-signal/14 aria-[current=page]:bg-signal/13 aria-[current=page]:text-signal aria-[current=page]:enabled:hover:bg-signal/20 aria-[current=page]:enabled:hover:text-signal max-[800px]:w-7 max-[600px]:h-7 max-[600px]:w-6"
            :disabled="!navigationHistory.canGoBack.value"
            :title="t('studio.navigateBack')"
            :aria-label="t('studio.navigateBack')"
            @click="navigationHistory.back"
          >
            <IconStudioChevronRight
              width="16"
              height="16"
              class="rotate-180 max-[600px]:size-[17px]"
            />
          </button>
          <button
            class="relative inline-flex size-[30px] items-center justify-center rounded border border-transparent bg-transparent p-0 text-muted transition-[background-color,color] duration-150 enabled:hover:bg-track enabled:hover:text-ink disabled:cursor-not-allowed disabled:opacity-35 aria-[current=page]:border-signal/14 aria-[current=page]:bg-signal/13 aria-[current=page]:text-signal aria-[current=page]:enabled:hover:bg-signal/20 aria-[current=page]:enabled:hover:text-signal max-[800px]:w-7 max-[600px]:h-7 max-[600px]:w-6"
            :disabled="!navigationHistory.canGoForward.value"
            :title="t('studio.navigateForward')"
            :aria-label="t('studio.navigateForward')"
            @click="navigationHistory.forward"
          >
            <IconStudioChevronRight
              class="max-[600px]:size-[17px]"
              width="16"
              height="16"
            />
          </button>
        </div>
        <span class="h-5 w-px bg-line" aria-hidden="true" />
        <nav
          class="flex items-center gap-0.5"
          :aria-label="t('studio.navigation')"
        >
          <button
            v-for="item in navigation"
            :key="item.id"
            class="relative inline-flex size-[30px] items-center justify-center rounded border border-transparent bg-transparent p-0 text-muted transition-[background-color,color] duration-150 enabled:hover:bg-track enabled:hover:text-ink disabled:cursor-not-allowed disabled:opacity-35 aria-[current=page]:border-signal/14 aria-[current=page]:bg-signal/13 aria-[current=page]:text-signal aria-[current=page]:enabled:hover:bg-signal/20 aria-[current=page]:enabled:hover:text-signal max-[800px]:w-7 max-[600px]:h-7 max-[600px]:w-6"
            :disabled="!navigationHistory.enabled.value"
            :title="t(`studio.nav.${item.id}`)"
            :aria-label="t(`studio.nav.${item.id}`)"
            :aria-current="view === item.id ? 'page' : undefined"
            @click="showPage(item.id)"
          >
            <component
              :is="item.icon"
              class="max-[600px]:size-[17px]"
              width="20"
              height="20"
            />
          </button>
        </nav>
      </div>
      <button
        class="relative mx-auto flex h-[34px] w-[520px] min-w-[230px] max-w-[44%] flex-col justify-center overflow-hidden rounded-md border border-[#d0d0d0] bg-lcd px-3 pt-0 pb-[5px] text-left shadow-[inset_0_1px_3px_#00000005] hover:border-muted max-[850px]:max-w-none max-[800px]:min-w-[120px] max-[800px]:flex-1 max-[600px]:min-w-[90px]"
        :aria-label="t('studio.activity')"
        @click="showActivity"
      >
        <div
          class="flex items-center justify-between gap-3 text-[12px] leading-[1.35] max-[600px]:text-[10px]"
        >
          <span
            class="font-semibold first:shrink-0 first:truncate last:truncate last:text-right last:text-[11px] max-[600px]:last:hidden"
            >{{ lcdTitle }}</span
          ><span
            class="text-muted first:shrink-0 first:truncate last:truncate last:text-right last:text-[11px] max-[600px]:last:hidden"
            >{{ lcdSubtitle }}{{ current ? ` · ${progress}%` : "" }}</span
          >
        </div>

        <div class="static mt-1 h-[3px] rounded-[2px] bg-[#c4c4c4]">
          <span
            :data-running="!!current"
            class="block h-full w-(--progress-width) bg-[#719185] transition-[width] duration-250 data-[running=true]:bg-signal"
            :style="{ '--progress-width': `${progress}%` }"
          />
        </div>
      </button>
      <div class="flex shrink-0 items-center gap-3 max-[850px]:gap-2">
        <details ref="deviceMenu" class="group/device-menu relative">
          <summary
            class="flex h-[34px] min-w-[115px] list-none items-center gap-2 rounded-[5px] border border-[#d0d0d0] px-2.5 py-0 text-muted hover:bg-track group-open/device-menu:bg-track max-[850px]:min-w-auto [&::-webkit-details-marker]:hidden"
          >
            <IconStudioDevice width="21" height="21" /><span
              class="max-[800px]:hidden"
              ><b
                class="block text-[12px] font-semibold whitespace-nowrap text-ink"
                >{{
                  session.device.value
                    ? session.device.value.marketingName
                    : t("app.noDevice")
                }}</b
              ><small
                class="mt-0 flex items-center gap-[5px] text-[10px] whitespace-nowrap"
                ><i
                  class="inline-block size-[5px] shrink-0 rounded-full"
                  :class="session.device.value ? 'bg-success' : 'bg-muted'"
                />{{
                  session.device.value
                    ? session.device.value.mode === "normal"
                      ? t("studio.usbConnected")
                      : t(`device.mode.${session.device.value.mode}`)
                    : t("studio.disconnected")
                }}</small
              ></span
            ><IconStudioChevronDown width="13" height="13" />
          </summary>
          <div
            class="absolute top-[47px] right-0 z-20 w-[248px] rounded-lg border border-line bg-raised p-2 shadow-[0_12px_35px_#00000020]"
          >
            <p class="text-[10px] tracking-[0.04em] text-muted p-[7px]">
              {{ t("studio.connectedDevices") }}
            </p>
            <button
              v-for="connected in session.devices.value"
              :key="connected.id"
              class="flex w-full items-center gap-2.5 rounded p-[9px] text-left text-[12px] hover:bg-track disabled:opacity-40"
              :disabled="preparing"
              :aria-pressed="session.device.value?.id === connected.id"
              @click="session.select(connected.id)"
            >
              <IconStudioDevice />
              <span class="min-w-0 flex-1 truncate">{{
                connected.marketingName
              }}</span>
              <IconStudioCheck
                v-if="session.device.value?.id === connected.id"
                class="text-signal"
                width="15"
                height="15"
              />
            </button>
            <p
              v-if="!session.devices.value.length"
              class="p-[9px] text-xs text-muted"
            >
              {{ t("app.noDevice") }}
            </p>
            <hr class="border-line" />
            <button
              class="flex w-full items-center gap-2.5 rounded p-[9px] text-[12px] hover:bg-track disabled:opacity-40"
              :disabled="session.scanning.value"
              @click="detect"
            >
              <IconStudioRefresh width="15" height="15" />{{
                t("studio.detectDevice")
              }}</button
            ><button
              v-if="gateway.capabilities.demo"
              class="flex w-full items-center gap-2.5 rounded p-[9px] text-[12px] hover:bg-track disabled:opacity-40"
              :disabled="!session.device.value || !!active.length"
              @click="eject"
            >
              <IconStudioEject width="15" height="15" />{{ t("studio.eject") }}
            </button>
          </div>
        </details>
        <label
          class="flex h-8 w-[180px] items-center gap-[7px] rounded-2xl border border-line bg-raised px-2.5 py-0 text-muted focus-within:border-signal focus-within:ring-2 focus-within:ring-signal/15 max-[1150px]:w-[135px] max-[600px]:w-[100px]"
          ><IconStudioSearch width="15" height="15" /><input
            v-model="store.query.value"
            class="w-full min-w-0 text-[12px] text-ink outline-none"
            type="search"
            :disabled="preparing"
            :placeholder="t('studio.search')"
            :aria-label="t('studio.search')"
            @input="search"
            @keydown.enter="search"
        /></label>
      </div>
    </header>
    <div class="flex min-h-0 flex-1">
      <aside
        v-if="view === 'device' && session.device.value"
        class="flex w-[220px] shrink-0 flex-col border-r border-line bg-sidebar pt-3 max-[1150px]:w-[190px] max-[850px]:w-40 max-[600px]:hidden"
      >
        <p
          class="mb-1 border-y border-line bg-track px-3.5 py-[3px] text-[11px] font-medium tracking-[0.03em] text-muted"
        >
          {{ t("studio.settingsGroup") }}
        </p>
        <nav :aria-label="t('studio.deviceNavigation')">
          <button
            v-for="section in ['summary', 'conditions', 'environment'] as const"
            :key="section"
            :disabled="preparing && section !== 'environment'"
            class="group/nav-item m-0 flex w-full items-center gap-2.5 rounded-none px-3.5 py-1.5 text-left text-[13px] text-ink hover:bg-track hover:text-ink disabled:cursor-not-allowed disabled:opacity-45 aria-[current=page]:bg-[#2f6fd6] aria-[current=page]:font-normal aria-[current=page]:text-white aria-[current=page]:hover:bg-[#2f6fd6] aria-[current=page]:hover:text-white"
            :aria-current="deviceSection === section ? 'page' : undefined"
            @click="showDevice(section)"
          >
            {{ t(`studio.sections.${section}`)
            }}<i
              v-if="section === 'conditions' && session.needsPreparation.value"
              class="ml-auto bg-warning inline-block size-[5px] shrink-0 rounded-full group-aria-[current=page]/nav-item:bg-white"
            />
          </button>
        </nav>
        <p
          class="mt-6 mb-1 border-y border-line bg-track px-3.5 py-[3px] text-[11px] font-medium tracking-[0.03em] text-muted"
        >
          {{ t("studio.onDevice") }}
        </p>
        <button
          class="group/nav-item m-0 flex w-full items-center gap-2.5 rounded-none px-3.5 py-1.5 text-left text-[13px] text-ink hover:bg-track hover:text-ink disabled:cursor-not-allowed disabled:opacity-45 aria-[current=page]:bg-[#2f6fd6] aria-[current=page]:font-normal aria-[current=page]:text-white aria-[current=page]:hover:bg-[#2f6fd6] aria-[current=page]:hover:text-white"
          :disabled="preparing"
          @click="showPage('installed')"
        >
          {{ t("studio.installedApps")
          }}<span class="ml-auto text-[10px]">{{
            gateway.capabilities.packages ? store.installed.value.length : "—"
          }}</span>
        </button>
        <button
          class="group/nav-item m-0 flex w-full items-center gap-2.5 rounded-none px-3.5 py-1.5 text-left text-[13px] text-ink hover:bg-track hover:text-ink disabled:cursor-not-allowed disabled:opacity-45 aria-[current=page]:bg-[#2f6fd6] aria-[current=page]:font-normal aria-[current=page]:text-white aria-[current=page]:hover:bg-[#2f6fd6] aria-[current=page]:hover:text-white"
          :disabled="preparing"
          @click="showPage('logs')"
        >
          {{ t("nav.logs") }}
        </button>
        <div class="mt-auto pb-[15px]">
          <div
            class="mx-2 mt-0 mb-2.5 flex items-center gap-2 border-b border-line px-0.5 py-3.5 text-[11px] text-muted"
          >
            <IconStudioUsb width="14" height="14" /><span>{{
              session.device.value
                ? t("studio.usbConnected")
                : t("studio.disconnected")
            }}</span
            ><i
              class="ml-auto inline-block size-[5px] shrink-0 rounded-full"
              :class="session.device.value ? 'bg-success' : 'bg-muted'"
            />
          </div>
          <button
            class="group/nav-item m-0 flex w-full items-center gap-2.5 rounded-none px-3.5 py-1.5 text-left text-[13px] text-ink hover:bg-track hover:text-ink disabled:cursor-not-allowed disabled:opacity-45 aria-[current=page]:bg-[#2f6fd6] aria-[current=page]:font-normal aria-[current=page]:text-white aria-[current=page]:hover:bg-[#2f6fd6] aria-[current=page]:hover:text-white"
            @click="settingsOpen = true"
          >
            <IconStudioSettings width="16" height="16" />{{
              t("studio.preferences")
            }}
          </button>
        </div>
      </aside>
      <main
        ref="mainContent"
        class="min-w-0 flex-1 overflow-y-auto p-5 [scrollbar-width:thin] [scrollbar-color:var(--color-line)_transparent]"
        :class="{ 'pl-8 max-[600px]:pl-5': view === 'device' }"
      >
        <DeviceView
          v-if="view === 'device'"
          :section="deviceSection"
          @open-logs="showPage('logs')"
          @open-store="showPage('store')"
          @show-conditions="showDevice('conditions')"
        />
        <StoreView
          v-else-if="view === 'store'"
          @prepare="startPreparation"
          @open-package="showPackage"
          @open-installed="showPage('installed')"
          @open-environment="showDevice('environment')"
        />
        <InstalledView
          v-else-if="view === 'installed'"
          @open-store="showPage('store')"
          @detail="showPackage"
        />
        <LogsView v-else />
      </main>
    </div>
    <section
      v-if="logsExpanded"
      class="max-h-[190px] overflow-y-auto border-t border-line bg-surface px-[25px] py-[15px] text-[11px]"
    >
      <div class="flex items-center justify-between">
        <b>{{ t("studio.recentActivity") }}</b
        ><button
          class="inline-flex items-center gap-[5px] text-[11px] text-signal hover:underline hover:underline-offset-[3px]"
          @click="
            showPage('logs');
            logsExpanded = false;
          "
        >
          {{ t("studio.allLogs")
          }}<IconStudioArrowRight width="12" height="12" />
        </button>
      </div>
      <div
        v-for="entry in log.entries.value.slice(-5)"
        :key="entry.id"
        class="mt-[9px] flex gap-[15px] text-[10px]"
      >
        <time class="font-mono text-muted">{{
          d(entry.timestamp, "time")
        }}</time
        ><span
          :class="entry.level === 'error' ? 'text-danger' : 'text-muted'"
          >{{ entry.level.toUpperCase() }}</span
        ><span>{{ renderLog(entry) }}</span>
      </div>
    </section>
    <footer
      class="flex h-[34px] shrink-0 items-center gap-3 border-t border-line bg-chrome px-[15px] py-0 text-[11px] text-muted"
    >
      <button
        class="flex min-w-0 flex-1 items-center gap-[9px] text-left"
        :aria-expanded="logsExpanded"
        @click="logsExpanded = !logsExpanded"
      >
        <IconStudioTerminal width="14" height="14" /><b
          class="font-medium whitespace-nowrap text-ink"
          >{{ t("nav.logs") }}</b
        ><time
          v-if="latestLog"
          class="font-mono text-[10px] max-[600px]:hidden"
          >{{ d(latestLog.timestamp, "time") }}</time
        ><span class="truncate">{{ latestMessage }}</span
        ><component
          :is="logsExpanded ? IconStudioChevronDown : IconStudioChevronUp"
          width="13"
          height="13"
        /></button
      ><button
        class="inline-flex shrink-0 items-center gap-[5px] text-[10px] whitespace-nowrap text-muted enabled:hover:text-ink disabled:cursor-not-allowed disabled:opacity-45"
        :disabled="!navigationHistory.enabled.value"
        @click="showPage('installed')"
      >
        <IconStudioDownload width="13" height="13" />{{ t("studio.queue") }}
        {{ active.length + store.queuedIds.value.length }}</button
      ><button
        v-if="gateway.capabilities.demo"
        class="flex items-center gap-[5px] border-l border-line pl-3 whitespace-nowrap"
        @click="demoOpen = true"
      >
        <IconStudioLab width="13" height="13" />{{ t("demo.title") }}</button
      ><button
        class="inline-flex items-center justify-center rounded p-[5px] text-muted hover:bg-track hover:text-ink"
        :aria-label="t('studio.preferences')"
        @click="settingsOpen = true"
      >
        <IconStudioSettings width="14" height="14" />
      </button>
    </footer>
  </div>
  <StudioDialog
    :open="settingsOpen"
    :title="t('studio.preferences')"
    @close="settingsOpen = false"
    ><SettingsView
  /></StudioDialog>
  <StudioDialog
    v-if="gateway.capabilities.demo"
    :open="demoOpen"
    :title="t('demo.title')"
    compact
    @close="demoOpen = false"
    ><DemoPanel
  /></StudioDialog>
  <PreparationWizard />
  <NotificationStack />
</template>
