<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";

import DemoPanel from "./app/DemoPanel.vue";
import NotificationStack from "./app/NotificationStack.vue";
import DeviceView from "./features/device/DeviceView.vue";
import LogsView from "./features/logs/LogsView.vue";
import PreparationWizard from "./features/preparation/PreparationWizard.vue";
import { usePreparation } from "./features/preparation/usePreparation";
import SettingsView from "./features/settings/SettingsView.vue";
import StoreView from "./features/store/StoreView.vue";
import { useDeviceSession } from "./shared/composables/useDeviceSession";
import { useOperationLog } from "./shared/composables/useOperationLog";
import { useOperations } from "./shared/composables/useOperations";
import AppIcon from "./shared/ui/AppIcon.vue";
import StatusPill from "./shared/ui/StatusPill.vue";

type View = "device" | "store" | "logs" | "settings";

const { t } = useI18n();
const view = ref<View>("device");
const demoOpen = ref(false);
const session = useDeviceSession();
const preparation = usePreparation();
const { active } = useOperations();

const navigation: Array<{ id: View; icon: string }> = [
  { id: "device", icon: "device" },
  { id: "store", icon: "store" },
  { id: "logs", icon: "logs" },
  { id: "settings", icon: "settings" },
];

function startPreparationFromStore(): void {
  if (session.device.value) void preparation.open(session.device.value.id);
}

onMounted(() => {
  void session.initialize();
  void useOperationLog().initialize();
});
</script>

<template>
  <div class="flex h-screen">
    <aside class="flex w-60 shrink-0 flex-col border-r border-line bg-surface">
      <div class="flex items-center gap-2 px-5 py-5">
        <span
          class="flex h-8 w-8 items-center justify-center rounded-lg bg-signal text-on-signal"
        >
          <AppIcon name="bolt" :size="18" />
        </span>
        <div>
          <div class="text-sm font-semibold">{{ t("app.name") }}</div>
          <div class="text-[11px] text-muted">{{ t("app.tagline") }}</div>
        </div>
      </div>

      <nav class="flex flex-col gap-1 px-3">
        <button
          v-for="item in navigation"
          :key="item.id"
          class="flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition"
          :class="
            view === item.id
              ? 'bg-signal/12 font-medium text-signal'
              : 'text-muted hover:bg-ink/5 hover:text-ink'
          "
          @click="view = item.id"
        >
          <AppIcon :name="item.icon" />
          {{ t(`nav.${item.id}`) }}
          <span
            v-if="item.id === 'logs' && active.length"
            class="ml-auto h-2 w-2 rounded-full bg-signal pulse"
          />
        </button>
      </nav>

      <div class="mt-auto flex flex-col gap-3 border-t border-line p-4">
        <div class="flex items-center gap-2 text-xs">
          <StatusPill v-if="session.device.value" tone="success" dot>{{
            session.device.value.modelIdentifier
          }}</StatusPill>
          <StatusPill v-else tone="neutral" dot>{{
            t("app.noDevice")
          }}</StatusPill>
          <StatusPill v-if="active.length" tone="signal">{{
            t("app.activeOperations", { count: active.length })
          }}</StatusPill>
        </div>
        <button
          class="btn btn-ghost justify-start px-2 text-xs"
          @click="demoOpen = !demoOpen"
        >
          <AppIcon name="lab" :size="14" />
          {{ demoOpen ? t("demo.hide") : t("demo.show") }}
        </button>
        <div
          v-if="demoOpen"
          class="rise rounded-xl border border-dashed border-line p-3"
        >
          <DemoPanel />
        </div>
      </div>
    </aside>

    <main class="scroll-thin min-w-0 flex-1 overflow-y-auto p-8">
      <DeviceView v-if="view === 'device'" @open-store="view = 'store'" />
      <StoreView
        v-else-if="view === 'store'"
        @prepare="startPreparationFromStore"
      />
      <LogsView v-else-if="view === 'logs'" />
      <SettingsView v-else />
    </main>

    <PreparationWizard
      @open-store="view = 'store'"
      @open-logs="view = 'logs'"
    />
    <NotificationStack />
  </div>
</template>
