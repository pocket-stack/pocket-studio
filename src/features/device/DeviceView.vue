<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { useGateway } from "../../shared/gateway";
import DeviceIllustration from "../../shared/ui/DeviceIllustration.vue";
import StatusPill from "../../shared/ui/StatusPill.vue";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioPanel from "../../shared/ui/StudioPanel.vue";
import { usePreparation } from "../preparation/usePreparation";
import { useStore } from "../store/useStore";
import PreparationWorkspace from "../preparation/PreparationWorkspace.vue";
import DeviceInfoCard from "./DeviceInfoCard.vue";
import ReadinessPanel from "./ReadinessPanel.vue";

defineProps<{ section: "summary" | "conditions" | "environment" }>();
const emit = defineEmits<{ openStore: []; showConditions: []; openLogs: [] }>();
const { t } = useI18n();
const {
  device,
  readiness,
  checking,
  checkReadiness,
  isReady,
  issues,
  lastError,
  refresh,
} = useDeviceSession();
const preparation = usePreparation();
const gateway = useGateway();
const store = useStore();
const appSize = computed(
  () =>
    store.installed.value.reduce(
      (sum, item) =>
        sum +
        (store.catalog.value.find((entry) => entry.id === item.packageId)
          ?.sizeBytes ?? 0),
      0,
    ) / 1e9,
);
const storage = computed(() => {
  const current = device.value;
  if (!current) return undefined;
  if (gateway.capabilities.demo) {
    const total = current.storageGb ?? 32;
    const apps = 1.1 + appSize.value;
    const free = Math.max(0, total - 4.7 - 6.8 - apps);
    return {
      total,
      free,
      segments: [
        { key: "system", gigabytes: 4.7, color: "var(--color-muted)" },
        { key: "media", gigabytes: 6.8, color: "var(--color-signal)" },
        { key: "apps", gigabytes: apps, color: "var(--color-success)" },
      ],
    };
  }
  if (current.storageTotalBytes && current.storageFreeBytes != null) {
    const total = Number((current.storageTotalBytes / 1e9).toFixed(1));
    const free = current.storageFreeBytes / 1e9;
    return {
      total,
      free,
      segments: [
        {
          key: "used",
          gigabytes: Math.max(0, total - free),
          color: "var(--color-signal)",
        },
      ],
    };
  }
  return undefined;
});
const diagnostics = computed(() =>
  issues.value.filter(
    (item) =>
      !item.deviceId || !device.value || item.deviceId === device.value.id,
  ),
);
function startPreparation(): void {
  if (device.value && gateway.capabilities.preparation)
    void preparation.open(device.value.id);
}
</script>

<template>
  <PreparationWorkspace
    v-if="section === 'environment' && preparation.stage.value !== 'closed'"
    @open-store="emit('openStore')"
    @open-logs="emit('openLogs')"
    @show-conditions="emit('showConditions')"
  />
  <div
    v-else-if="!device"
    class="flex h-full flex-col items-center justify-center gap-4 text-center motion-safe:animate-rise"
  >
    <div class="relative mb-2">
      <DeviceIllustration :width="104" screen="off" cable shadow /><span
        class="absolute top-[42%] -right-4 grid size-10 place-items-center rounded-full bg-raised text-signal shadow-raised"
        ><IconPhUsb width="20" height="20"
      /></span>
    </div>
    <p class="text-2xs font-semibold tracking-[0.08em] text-muted uppercase">
      {{ t("studio.waitingForDevice") }}
    </p>
    <h1 class="text-2xl font-semibold tracking-tight">
      {{ t("device.empty.title") }}
    </h1>
    <p class="max-w-[480px] text-sm text-muted">
      {{
        t(
          gateway.capabilities.demo
            ? "device.empty.body"
            : "connection.emptyBody",
        )
      }}
    </p>
    <ol class="flex flex-col gap-1 text-left text-sm text-muted">
      <li v-for="(step, index) in ['step1', 'step3']" :key="step">
        <b class="mr-2 font-semibold text-signal">{{ index + 1 }}</b
        >{{ t(`device.empty.${step}`) }}
      </li>
    </ol>
    <StudioCallout v-if="diagnostics.length || lastError" tone="warning">
      <span v-if="lastError">{{ t("connection.scanFailed") }} </span
      ><span v-for="item in diagnostics" :key="`${item.code}-${item.deviceId}`"
        >{{ t(`connection.issues.${item.code}`) }}
      </span>
    </StudioCallout>
    <div class="mt-2 flex gap-2">
      <StudioButton
        variant="primary"
        :disabled="checking"
        @click="
          gateway.capabilities.demo ? gateway.demo.attachDevice() : refresh()
        "
      >
        <IconPhUsb width="15" height="15" />{{
          t(gateway.capabilities.demo ? "demo.attach" : "studio.detectDevice")
        }}
      </StudioButton>
      <StudioButton
        v-if="gateway.capabilities.demo"
        :disabled="checking"
        @click="gateway.demo.attachDevice('n3dsll')"
      >
        <IconPhWifiHigh width="15" height="15" />{{ t("demo.attach3ds") }}
      </StudioButton>
      <StudioButton @click="emit('openStore')">
        {{ t("studio.browseStore") }}
      </StudioButton>
    </div>
  </div>
  <div
    v-else
    :key="section"
    class="flex h-full min-h-0 flex-col gap-4 motion-safe:animate-rise"
  >
    <StudioCallout v-if="diagnostics.length || lastError" tone="warning">
      <span v-if="lastError">{{ t("connection.scanFailed") }} </span
      ><span v-for="item in diagnostics" :key="`${item.code}-${item.deviceId}`"
        >{{ t(`connection.issues.${item.code}`) }}
      </span>
    </StudioCallout>
    <template v-if="section === 'summary'">
      <DeviceInfoCard
        :device="device"
        :storage="storage"
        :sample="gateway.capabilities.demo"
      />
      <ReadinessPanel
        :report="readiness"
        :device="device"
        :checking="checking"
        compact
        @recheck="checkReadiness"
        @prepare="startPreparation"
        @details="emit('showConditions')"
        @open-store="emit('openStore')"
      />
    </template>
    <template v-else-if="section === 'conditions'">
      <header>
        <h1 class="text-xl font-semibold">
          {{ t("studio.sections.conditions") }}
        </h1>
        <p class="text-xs text-muted">{{ t("readiness.subtitle") }}</p>
      </header>
      <ReadinessPanel
        :report="readiness"
        :device="device"
        :checking="checking"
        @recheck="checkReadiness"
        @prepare="startPreparation"
        @open-store="emit('openStore')"
      />
    </template>
    <template v-else>
      <header>
        <h1 class="text-xl font-semibold">
          {{ t("studio.sections.environment") }}
        </h1>
        <p class="text-xs text-muted">{{ t("studio.environmentIntro") }}</p>
      </header>
      <ReadinessPanel
        v-if="!gateway.capabilities.preparation"
        :report="readiness"
        :device="device"
        :checking="checking"
        @recheck="checkReadiness"
      />
      <!-- The 3DS arrives with custom firmware already installed; Studio only
           verifies it and points at the community guide otherwise. -->
      <StudioPanel v-else-if="device.platform === '3ds'" class="max-w-[620px]">
        <div class="flex items-center gap-3">
          <h2 class="text-lg font-semibold">{{ t("studio.cfw.title") }}</h2>
          <StatusPill :tone="isReady ? 'success' : 'warning'" dot>{{
            t(isReady ? "studio.allReady" : "studio.cfw.missing")
          }}</StatusPill>
        </div>
        <p class="mt-2 text-sm text-muted">{{ t("studio.cfw.description") }}</p>
        <StudioCallout v-if="!isReady" tone="warning" class="mt-3">
          {{ t("studio.cfw.notice") }}
        </StudioCallout>
        <div class="mt-4 flex justify-end gap-2">
          <StudioButton :disabled="checking" @click="checkReadiness">
            <IconSvgSpinners90Ring v-if="checking" width="13" height="13" />
            <IconPhArrowsClockwise v-else width="13" height="13" />{{
              t("readiness.recheck")
            }}
          </StudioButton>
          <StudioButton
            v-if="isReady"
            variant="primary"
            @click="emit('openStore')"
          >
            {{ t("device.next.action") }}
          </StudioButton>
        </div>
      </StudioPanel>
      <StudioPanel v-else class="max-w-[620px]">
        <div class="flex items-center gap-3">
          <h2 class="text-lg font-semibold">
            {{ t("studio.environmentName") }}
          </h2>
          <StatusPill :tone="isReady ? 'success' : 'warning'" dot>{{
            t(isReady ? "studio.allReady" : "studio.notInstalled")
          }}</StatusPill>
        </div>
        <p class="mt-2 text-sm text-muted">
          {{ t("studio.environmentDescription") }}
        </p>
        <StudioCallout tone="neutral" class="mt-3">
          {{ t("studio.environmentRisk") }}
        </StudioCallout>
        <div class="mt-4 flex justify-end">
          <StudioButton
            v-if="!isReady"
            variant="primary"
            :disabled="checking"
            @click="startPreparation"
          >
            {{ t("studio.beginPreparation")
            }}<IconPhArrowRight width="14" height="14" />
          </StudioButton>
          <StudioButton v-else variant="primary" @click="emit('openStore')">
            {{ t("device.next.action") }}
          </StudioButton>
        </div>
      </StudioPanel>
    </template>
  </div>
</template>
