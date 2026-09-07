<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { useGateway } from "../../shared/gateway";
import DeviceIllustration from "../../shared/ui/DeviceIllustration.vue";
import StatusPill from "../../shared/ui/StatusPill.vue";
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
const storage = computed(() => [
  { key: "system", size: 4.7, color: "#8792a2" },
  { key: "media", size: 6.8, color: "#6597d7" },
  { key: "apps", size: 1.1 + appSize.value, color: "#8db6b0" },
  {
    key: "free",
    size: Math.max(0, (device.value?.storageGb ?? 32) - 12.6 - appSize.value),
    color: "var(--ps-track)",
  },
]);
function startPreparation(): void {
  if (device.value && gateway.capabilities.preparation)
    void preparation.open(device.value.id);
}
</script>

<template>
  <section
    v-if="issues.length || lastError"
    role="status"
    class="mb-4 rounded-lg border border-warning/35 bg-warning/5 p-3 text-[12px]"
  >
    <h2 class="mb-1 font-medium">{{ t("connection.diagnosticsTitle") }}</h2>
    <p v-if="lastError" class="text-muted">{{ t("connection.scanFailed") }}</p>
    <p
      v-for="item in issues.filter(
        (item) => !item.deviceId || !device || item.deviceId === device.id,
      )"
      :key="`${item.code}-${item.deviceId}`"
      class="leading-6 text-muted"
    >
      {{ t(`connection.issues.${item.code}`) }}
    </p>
  </section>
  <PreparationWorkspace
    v-if="section === 'environment' && preparation.stage.value !== 'closed'"
    @open-store="emit('openStore')"
    @open-logs="emit('openLogs')"
    @show-conditions="emit('showConditions')"
  />
  <div
    v-else-if="!device"
    class="flex min-h-full flex-col items-center justify-center gap-[19px] p-8 text-center motion-safe:animate-rise"
  >
    <div class="relative mb-[7px] opacity-80">
      <DeviceIllustration :width="135" screen="off" cable /><span
        class="absolute top-[40%] -right-2.5 grid size-[46px] place-items-center rounded-full bg-raised text-signal shadow-[0_3px_20px_#00000012]"
        ><IconStudioUsb width="23" height="23"
      /></span>
    </div>
    <p class="text-[10px] tracking-[0.04em] text-muted">
      {{ t("studio.waitingForDevice") }}
    </p>
    <h1 class="text-[24px] font-semibold tracking-[-0.5px]">
      {{ t("device.empty.title") }}
    </h1>
    <p class="max-w-[520px] text-[12px] leading-[1.9] text-muted">
      {{
        t(
          gateway.capabilities.demo
            ? "device.empty.body"
            : "connection.emptyBody",
        )
      }}
    </p>
    <div class="flex flex-col gap-2.5 text-left text-[11px] text-muted">
      <span
        ><b class="mr-2.5 text-signal">1</b>{{ t("device.empty.step1") }}</span
      ><span
        ><b class="mr-2.5 text-signal">2</b>{{ t("device.empty.step3") }}</span
      >
    </div>
    <div class="flex gap-3">
      <button
        class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
        :disabled="checking"
        @click="
          gateway.capabilities.demo ? gateway.demo.attachDevice() : refresh()
        "
      >
        <IconStudioUsb width="15" height="15" />{{
          t(gateway.capabilities.demo ? "demo.attach" : "studio.detectDevice")
        }}</button
      ><button
        class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
        @click="emit('openStore')"
      >
        {{ t("studio.browseStore") }}
      </button>
    </div>
  </div>
  <div
    v-else
    :key="section"
    class="mx-auto flex min-h-full max-w-none flex-col pb-0 motion-safe:animate-rise"
  >
    <template v-if="section === 'summary'">
      <DeviceInfoCard :device="device" />
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
      <section
        v-if="gateway.capabilities.demo"
        class="mt-auto pt-6 text-[11px]"
      >
        <div class="flex items-center justify-between first:hidden">
          <h3 class="font-medium">{{ t("studio.storage") }}</h3>
          <span>{{
            t("studio.storageAvailable", {
              free: storage[3]?.size.toFixed(1),
              total: device.storageGb,
            })
          }}</span>
        </div>
        <div
          class="mt-[9px] flex h-[22px] overflow-hidden rounded-md border border-line first:hidden"
          :aria-label="
            t('studio.storageAvailable', {
              free: storage[3]?.size.toFixed(1),
              total: device.storageGb,
            })
          "
        >
          <span
            v-for="segment in storage"
            :key="segment.key"
            class="w-(--segment-width) border-r-2 border-canvas bg-(--segment-color) transition-[width] duration-600 last:border-0"
            :style="{
              '--segment-width': `${(segment.size / (device.storageGb || 1)) * 100}%`,
              '--segment-color': segment.color,
            }"
            :title="`${t(`studio.storageTypes.${segment.key}`)} ${segment.size.toFixed(1)} GB`"
          />
        </div>
        <div
          class="mt-1.5 flex flex-wrap gap-[18px] text-[10px] text-muted max-[800px]:gap-2.5 first:hidden"
        >
          <span
            v-for="segment in storage"
            :key="segment.key"
            class="flex items-center gap-[5px]"
            ><i
              class="size-1.5 rounded-[2px] bg-(--segment-color)"
              :style="{ '--segment-color': segment.color }"
            />{{ t(`studio.storageTypes.${segment.key}`)
            }}<b class="ml-0.5 font-normal"
              >{{ segment.size.toFixed(1) }} GB</b
            ></span
          ><small class="ml-auto max-[800px]:w-full">{{
            t("studio.sampleStorage")
          }}</small>
        </div>
      </section>
      <section
        v-if="
          !gateway.capabilities.demo &&
          device.storageTotalBytes &&
          device.storageFreeBytes != null
        "
        class="mt-auto pt-6 text-[11px] text-muted"
      >
        <div class="mb-2 flex justify-between">
          <span>{{ t("connection.storageTitle") }}</span
          ><span>{{
            t("studio.storageAvailable", {
              free: (device.storageFreeBytes / 1e9).toFixed(1),
              total: (device.storageTotalBytes / 1e9).toFixed(1),
            })
          }}</span>
        </div>
        <div
          class="h-[22px] overflow-hidden rounded-md border border-line bg-track"
        >
          <span
            class="block h-full w-(--used-width) bg-signal/65"
            :style="{
              '--used-width': `${Math.min(100, Math.max(0, (1 - device.storageFreeBytes / device.storageTotalBytes) * 100))}%`,
            }"
          />
        </div>
      </section>
    </template>
    <template v-else-if="section === 'conditions'"
      ><div class="px-0 pt-0 pb-3.5">
        <h1 class="text-[22px] font-semibold tracking-[-0.6px]">
          {{ t("studio.sections.conditions") }}
        </h1>
        <p class="mt-[7px] text-[12px] text-muted">
          {{ t("readiness.subtitle") }}
        </p>
      </div>
      <ReadinessPanel
        :report="readiness"
        :device="device"
        :checking="checking"
        @recheck="checkReadiness"
        @prepare="startPreparation"
        @open-store="emit('openStore')"
    /></template>
    <section v-else-if="!gateway.capabilities.preparation">
      <h1 class="mb-4 text-[22px] font-semibold">
        {{ t("studio.sections.environment") }}
      </h1>
      <ReadinessPanel
        :report="readiness"
        :device="device"
        :checking="checking"
        @recheck="checkReadiness"
      />
    </section>
    <PreparationWorkspace
      v-else-if="preparation.stage.value !== 'closed'"
      @open-store="emit('openStore')"
      @open-logs="emit('openLogs')"
      @show-conditions="emit('showConditions')"
    />
    <template v-else>
      <div class="px-0 pt-0 pb-3.5">
        <h1 class="text-[22px] font-semibold tracking-[-0.6px]">
          {{ t("studio.sections.environment") }}
        </h1>
        <p class="mt-[7px] text-[12px] text-muted">
          {{ t("studio.environmentIntro") }}
        </p>
      </div>
      <section
        class="max-w-[640px] p-4 rounded-lg border border-line bg-surface"
      >
        <div class="flex gap-4">
          <span class="hidden"
            ><IconStudioPocket width="30" height="30"
          /></span>
          <div class="flex-1">
            <div class="flex items-center gap-3">
              <h2 class="text-[15px] font-semibold">
                {{ t("studio.environmentName") }}
              </h2>
              <StatusPill :tone="isReady ? 'success' : 'warning'" dot>{{
                t(isReady ? "studio.allReady" : "studio.notInstalled")
              }}</StatusPill>
            </div>
            <p class="mt-2 text-sm text-muted">
              {{ t("studio.environmentDescription") }}
            </p>
          </div>
        </div>
        <div class="hidden">
          <div>
            <span>{{ t("studio.targetDevice") }}</span
            ><b>{{ t("studio.deviceName") }}</b>
          </div>
          <div>
            <span>{{ t("device.fields.os") }}</span
            ><b>iOS {{ device.osVersion }}</b>
          </div>
          <div>
            <span>{{ t("studio.method") }}</span
            ><b>{{ t("studio.ramdiskMethod") }}</b>
          </div>
        </div>
        <div
          class="flex flex-wrap items-center justify-between gap-5 border-0 pt-3.5"
        >
          <p
            class="flex flex-1 basis-full items-start gap-[7px] text-[11px] text-muted"
          >
            <IconStudioInfo width="16" height="16" />{{
              t("studio.environmentRisk")
            }}
          </p>
          <button
            v-if="!isReady"
            class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
            :disabled="checking"
            @click="startPreparation"
          >
            {{ t("studio.beginPreparation")
            }}<IconStudioArrowRight width="15" height="15" /></button
          ><button
            v-else
            class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
            @click="emit('openStore')"
          >
            {{ t("device.next.action") }}
          </button>
        </div>
      </section>
      <section class="hidden">
        <h3>{{ t("studio.howItWorks") }}</h3>
        <div>
          <article
            v-for="(step, index) in ['confirm', 'guide', 'prepare', 'verify']"
            :key="step"
          >
            <span>{{ String(index + 1).padStart(2, "0") }}</span>
            <h4>{{ t(`studio.roadmap.${step}.title`) }}</h4>
            <p>{{ t(`studio.roadmap.${step}.body`) }}</p>
          </article>
        </div>
      </section>
      <div class="hidden">
        <IconStudioExternal width="15" height="15" /><span>{{
          t("studio.reference")
        }}</span
        ><a
          href="https://github.com/LukeZGD/Legacy-iOS-Kit/wiki/Jailbreaking-with-Legacy-iOS-Kit"
          target="_blank"
          rel="noreferrer"
          >{{ t("studio.upstreamGuide") }}</a
        ><a
          href="https://github.com/HalfSweet/Legacy-iOS-Kit-rs"
          target="_blank"
          rel="noreferrer"
          >Legacy-iOS-Kit-rs</a
        >
      </div>
    </template>
  </div>
</template>
