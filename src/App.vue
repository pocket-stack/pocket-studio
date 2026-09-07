<script setup lang="ts">
import IconPhDeviceMobile from "~icons/ph/device-mobile-fill";
import IconPhSquaresFour from "~icons/ph/squares-four-fill";
import IconPhFileText from "~icons/ph/file-text-fill";
import IconPhStorefront from "~icons/ph/storefront-fill";
import IconPhCaretDown from "~icons/ph/caret-down";
import IconPhCaretUp from "~icons/ph/caret-up";
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
import PackagePlanDialog from "./features/store/PackagePlanDialog.vue";
import PreparationWizard from "./features/preparation/PreparationWizard.vue";
import { usePreparation } from "./features/preparation/usePreparation";
import SettingsView from "./features/settings/SettingsView.vue";
import StoreView from "./features/store/StoreView.vue";
import InstalledView from "./features/store/InstalledView.vue";
import { useStore } from "./features/store/useStore";
import { packageText } from "./features/store/packageContent";
import { useDeviceSession } from "./shared/composables/useDeviceSession";
import { useLogMessage } from "./shared/composables/useLogMessage";
import { useOperationLog } from "./shared/composables/useOperationLog";
import {
  operationProgress,
  operationStepCeiling,
  useOperations,
} from "./shared/composables/useOperations";
import { useGateway } from "./shared/gateway";
import ProgressBar from "./shared/ui/ProgressBar.vue";
import StudioDialog from "./shared/ui/StudioDialog.vue";
import StudioInput from "./shared/ui/StudioInput.vue";

const { t, d, locale } = useI18n();
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
const current = computed(
  () =>
    active.value.find((operation) => operation.currentStepId) ??
    active.value[0],
);
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
  current.value ? operationProgress(current.value) : 0,
);
const ceiling = computed(() =>
  current.value ? operationStepCeiling(current.value) : 0,
);
const stepPosition = computed(() => {
  const operation = current.value;
  if (!operation?.currentStepId) return null;
  const index = operation.steps.findIndex(
    (step) => step.id === operation.currentStepId,
  );
  return index < 0
    ? null
    : t("studio.stepOf", { index: index + 1, total: operation.steps.length });
});
const lcdTitle = computed(() => {
  const operation = current.value;
  const entry =
    operation &&
    store.catalog.value.find((item) => item.id === operation.subject);
  if (operation?.packagePlan)
    return t("store.actions.activity", {
      action: t(`store.actions.kind.${operation.packagePlan.action}`),
      name:
        operation.packagePlan.names[locale.value] ??
        operation.packagePlan.names.en,
      device: operation.packagePlan.deviceName,
    });
  if (operation)
    return operation.kind === "preparation"
      ? t("studio.preparing")
      : t("studio.installing", {
          name: operation.packagePlan
            ? `${operation.packagePlan.names[locale.value] ?? operation.packagePlan.names.en} · ${operation.packagePlan.deviceName}`
            : entry
              ? packageText(entry, "name", locale.value, t)
              : (operation.subject ?? t("store.title")),
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
  { id: "device", icon: IconPhDeviceMobile },
  { id: "installed", icon: IconPhSquaresFour },
  { id: "store", icon: IconPhStorefront },
  { id: "logs", icon: IconPhFileText },
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
      class="flex h-[52px] shrink-0 items-center gap-3 border-b border-line bg-chrome px-3"
    >
      <div
        class="flex shrink-0 items-center gap-0.5"
        role="group"
        :aria-label="t('studio.historyNavigation')"
      >
        <button
          v-for="direction in ['back', 'forward'] as const"
          :key="direction"
          class="inline-flex size-7 items-center justify-center rounded-control text-muted transition-colors enabled:hover:bg-ink/6 enabled:hover:text-ink disabled:opacity-30"
          :disabled="
            direction === 'back'
              ? !navigationHistory.canGoBack.value
              : !navigationHistory.canGoForward.value
          "
          :title="
            t(
              direction === 'back'
                ? 'studio.navigateBack'
                : 'studio.navigateForward',
            )
          "
          :aria-label="
            t(
              direction === 'back'
                ? 'studio.navigateBack'
                : 'studio.navigateForward',
            )
          "
          @click="
            direction === 'back'
              ? navigationHistory.back()
              : navigationHistory.forward()
          "
        >
          <IconPhCaretRight
            width="15"
            height="15"
            :class="{ 'rotate-180': direction === 'back' }"
          />
        </button>
      </div>
      <nav
        class="flex shrink-0 items-center gap-px rounded-control bg-ink/6 p-0.5"
        :aria-label="t('studio.navigation')"
      >
        <button
          v-for="item in navigation"
          :key="item.id"
          class="inline-flex h-[26px] items-center gap-1.5 rounded-[5px] px-2.5 text-sm text-muted transition-[background-color,color,box-shadow] duration-150 enabled:hover:text-ink disabled:cursor-not-allowed disabled:opacity-40 aria-[current=page]:bg-raised aria-[current=page]:text-ink aria-[current=page]:shadow-control"
          :disabled="!navigationHistory.enabled.value"
          :title="t(`studio.nav.${item.id}`)"
          :aria-current="view === item.id ? 'page' : undefined"
          @click="showPage(item.id)"
        >
          <component :is="item.icon" width="16" height="16" />
          <span class="max-[1100px]:sr-only">{{
            t(`studio.nav.${item.id}`)
          }}</span>
        </button>
      </nav>
      <button
        class="relative mx-auto flex h-[40px] w-[520px] min-w-[200px] flex-col justify-center overflow-hidden rounded-control bg-lcd px-3 text-left shadow-inset transition-colors hover:bg-lcd/80"
        :aria-label="t('studio.activity')"
        @click="showActivity"
      >
        <div
          class="flex items-baseline justify-between gap-3 text-sm leading-[16px]"
        >
          <span class="min-w-0 truncate font-semibold">{{ lcdTitle }}</span
          ><span v-if="current" class="shrink-0 text-xs text-muted tabular-nums"
            >{{ progress }}%</span
          >
        </div>
        <div
          class="flex items-baseline justify-between gap-3 text-xs leading-[14px] text-muted"
        >
          <span class="min-w-0 truncate">{{ lcdSubtitle }}</span
          ><span v-if="stepPosition" class="shrink-0 tabular-nums">{{
            stepPosition
          }}</span>
        </div>
        <ProgressBar
          v-if="current"
          class="mt-1"
          :percent="progress"
          :trickle-to="ceiling"
          active
          compact
        />
      </button>
      <div class="flex shrink-0 items-center gap-2.5">
        <details ref="deviceMenu" class="group/device-menu relative">
          <summary
            class="flex h-8 min-w-[112px] list-none items-center gap-2 rounded-control bg-raised px-2.5 text-muted shadow-control transition-colors hover:bg-track group-open/device-menu:bg-track [&::-webkit-details-marker]:hidden"
          >
            <IconPhDeviceMobile width="18" height="18" /><span
              class="max-[900px]:hidden"
              ><b
                class="block text-sm leading-[15px] font-semibold whitespace-nowrap text-ink"
                >{{
                  session.device.value
                    ? session.device.value.marketingName
                    : t("app.noDevice")
                }}</b
              ><small
                class="flex items-center gap-1 text-2xs leading-[13px] whitespace-nowrap"
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
            ><IconPhCaretDown width="12" height="12" />
          </summary>
          <div
            class="absolute top-[38px] right-0 z-20 w-[248px] rounded-panel bg-raised p-1.5 shadow-overlay"
          >
            <p
              class="px-2 py-1.5 text-2xs font-semibold tracking-wide text-muted"
            >
              {{ t("studio.connectedDevices") }}
            </p>
            <button
              v-for="connected in session.devices.value"
              :key="connected.id"
              class="flex w-full items-center gap-2.5 rounded-control px-2 py-1.5 text-left text-sm hover:bg-track disabled:opacity-40"
              :disabled="preparing"
              :aria-pressed="session.device.value?.id === connected.id"
              @click="session.select(connected.id)"
            >
              <IconPhDeviceMobile width="16" height="16" />
              <span class="min-w-0 flex-1 truncate">{{
                connected.marketingName
              }}</span>
              <IconPhCheck
                v-if="session.device.value?.id === connected.id"
                class="text-signal"
                width="14"
                height="14"
              />
            </button>
            <p
              v-if="!session.devices.value.length"
              class="px-2 py-1.5 text-sm text-muted"
            >
              {{ t("app.noDevice") }}
            </p>
            <div class="mx-1 my-1 h-px bg-ink/8" />
            <button
              class="flex w-full items-center gap-2.5 rounded-control px-2 py-1.5 text-sm hover:bg-track disabled:opacity-40"
              :disabled="session.scanning.value"
              @click="detect"
            >
              <IconPhArrowsClockwise width="14" height="14" />{{
                t("studio.detectDevice")
              }}</button
            ><button
              v-if="gateway.capabilities.demo"
              class="flex w-full items-center gap-2.5 rounded-control px-2 py-1.5 text-sm hover:bg-track disabled:opacity-40"
              :disabled="!session.device.value || !!active.length"
              @click="eject"
            >
              <IconPhEject width="14" height="14" />{{ t("studio.eject") }}
            </button>
          </div>
        </details>
        <StudioInput
          v-model="store.query.value"
          type="search"
          class="w-[180px] max-[1100px]:w-[140px]"
          :disabled="preparing"
          :placeholder="t('studio.search')"
          :label="t('studio.search')"
          @input="search"
          @keydown.enter="search"
        >
          <template #icon
            ><IconPhMagnifyingGlass width="14" height="14"
          /></template>
        </StudioInput>
      </div>
    </header>
    <div class="flex min-h-0 flex-1">
      <aside
        v-if="view === 'device' && session.device.value"
        class="flex w-[208px] shrink-0 flex-col bg-sidebar pt-2 max-[1100px]:w-[184px]"
      >
        <p
          class="px-4 pt-2 pb-1 text-2xs font-semibold tracking-[0.06em] text-muted uppercase"
        >
          {{ t("studio.settingsGroup") }}
        </p>
        <nav
          class="flex flex-col gap-px px-2"
          :aria-label="t('studio.deviceNavigation')"
        >
          <button
            v-for="section in ['summary', 'conditions', 'environment'] as const"
            :key="section"
            :disabled="preparing && section !== 'environment'"
            class="group/nav-item flex w-full items-center gap-2.5 rounded-control px-2.5 py-[5px] text-left text-base text-ink transition-colors hover:bg-ink/6 disabled:cursor-not-allowed disabled:opacity-40 aria-[current=page]:bg-signal aria-[current=page]:text-on-signal aria-[current=page]:hover:bg-signal"
            :aria-current="deviceSection === section ? 'page' : undefined"
            @click="showDevice(section)"
          >
            {{ t(`studio.sections.${section}`)
            }}<i
              v-if="section === 'conditions' && session.needsPreparation.value"
              class="ml-auto inline-block size-[5px] shrink-0 rounded-full bg-warning group-aria-[current=page]/nav-item:bg-on-signal"
            />
          </button>
        </nav>
        <p
          class="px-4 pt-5 pb-1 text-2xs font-semibold tracking-[0.06em] text-muted uppercase"
        >
          {{ t("studio.onDevice") }}
        </p>
        <div class="flex flex-col gap-px px-2">
          <button
            class="flex w-full items-center gap-2.5 rounded-control px-2.5 py-[5px] text-left text-base text-ink transition-colors hover:bg-ink/6 disabled:cursor-not-allowed disabled:opacity-40"
            :disabled="preparing"
            @click="showPage('installed')"
          >
            {{ t("studio.installedApps")
            }}<span class="ml-auto text-xs text-muted">{{
              gateway.capabilities.installed
                ? store.installed.value.length
                : "—"
            }}</span>
          </button>
          <button
            class="flex w-full items-center gap-2.5 rounded-control px-2.5 py-[5px] text-left text-base text-ink transition-colors hover:bg-ink/6 disabled:cursor-not-allowed disabled:opacity-40"
            :disabled="preparing"
            @click="showPage('logs')"
          >
            {{ t("nav.logs") }}
          </button>
        </div>
        <div class="mt-auto px-2 pb-3">
          <div
            class="mx-2 mb-2 flex items-center gap-2 py-2.5 text-xs text-muted"
          >
            <IconPhUsb width="14" height="14" /><span>{{
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
            class="flex w-full items-center gap-2.5 rounded-control px-2.5 py-[5px] text-left text-base text-ink transition-colors hover:bg-ink/6"
            @click="settingsOpen = true"
          >
            <IconPhGear width="15" height="15" />{{ t("studio.preferences") }}
          </button>
        </div>
      </aside>
      <main
        ref="mainContent"
        class="min-w-0 flex-1 overflow-y-auto p-5 [scrollbar-width:thin] [scrollbar-color:var(--color-line)_transparent]"
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
          @prepare="showDevice('environment')"
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
      class="max-h-[170px] shrink-0 overflow-hidden border-t border-line bg-surface px-5 py-3 text-xs"
    >
      <div class="flex items-center justify-between">
        <b class="font-semibold">{{ t("studio.recentActivity") }}</b
        ><button
          class="inline-flex items-center gap-1 text-xs text-signal hover:underline hover:underline-offset-2"
          @click="
            showPage('logs');
            logsExpanded = false;
          "
        >
          {{ t("studio.allLogs") }}<IconPhArrowRight width="12" height="12" />
        </button>
      </div>
      <div
        v-for="entry in log.entries.value.slice(-5)"
        :key="entry.id"
        class="mt-1.5 flex gap-3 text-2xs"
      >
        <time class="font-mono text-muted">{{
          d(entry.timestamp, "time")
        }}</time
        ><span
          class="w-10 font-mono"
          :class="entry.level === 'error' ? 'text-danger' : 'text-muted'"
          >{{ entry.level.toUpperCase() }}</span
        ><span class="truncate">{{ renderLog(entry) }}</span>
      </div>
    </section>
    <footer
      class="flex h-[30px] shrink-0 items-center gap-3 border-t border-line bg-chrome px-3 text-xs text-muted"
    >
      <button
        class="flex min-w-0 flex-1 items-center gap-2 text-left"
        :aria-expanded="logsExpanded"
        @click="logsExpanded = !logsExpanded"
      >
        <IconPhTerminalWindow width="13" height="13" /><b
          class="font-medium whitespace-nowrap text-ink"
          >{{ t("nav.logs") }}</b
        ><time v-if="latestLog" class="font-mono text-2xs">{{
          d(latestLog.timestamp, "time")
        }}</time
        ><span class="truncate">{{ latestMessage }}</span
        ><component
          :is="logsExpanded ? IconPhCaretDown : IconPhCaretUp"
          width="12"
          height="12"
        /></button
      ><button
        class="inline-flex shrink-0 items-center gap-1 whitespace-nowrap enabled:hover:text-ink disabled:cursor-not-allowed disabled:opacity-45"
        :disabled="!navigationHistory.enabled.value"
        @click="showPage('installed')"
      >
        <IconPhDownloadSimple width="13" height="13" />{{ t("studio.queue") }}
        {{ active.length + store.queuedIds.value.length }}</button
      ><button
        v-if="gateway.capabilities.demo"
        class="inline-flex items-center gap-1 whitespace-nowrap hover:text-ink"
        @click="demoOpen = true"
      >
        <IconPhFlask width="13" height="13" />{{ t("demo.title") }}</button
      ><button
        class="inline-flex items-center justify-center rounded p-1 text-muted hover:bg-ink/6 hover:text-ink"
        :aria-label="t('studio.preferences')"
        @click="settingsOpen = true"
      >
        <IconPhGear width="14" height="14" />
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
  <PackagePlanDialog />
  <PreparationWizard />
  <NotificationStack />
</template>
