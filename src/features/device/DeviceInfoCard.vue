<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { DeviceSummary } from "../../shared/gateway";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import DeviceIllustration from "../../shared/ui/DeviceIllustration.vue";
import StatusPill from "../../shared/ui/StatusPill.vue";
import StudioPanel from "../../shared/ui/StudioPanel.vue";

export interface StorageSegment {
  key: string;
  gigabytes: number;
  color: string;
}

const props = defineProps<{
  device: DeviceSummary;
  storage?: { total: number; free: number; segments: StorageSegment[] };
  sample?: boolean;
}>();
const { t } = useI18n();
const { isReady, readiness } = useDeviceSession();
const screen = computed(() =>
  props.device.mode === "dfu"
    ? "off"
    : props.device.mode === "recovery"
      ? "recovery"
      : "home",
);
const rows = computed(() => [
  {
    key: "storage",
    value:
      props.device.storageTotalBytes == null
        ? t("connection.unknownValue")
        : `${(props.device.storageTotalBytes / 1e9).toFixed(1)} GB`,
  },
  {
    key: "battery",
    value:
      props.device.batteryPercent == null
        ? t("connection.unknownValue")
        : `${props.device.batteryPercent}%`,
  },
  {
    key: "os",
    value: props.device.osVersion
      ? `iOS ${props.device.osVersion}${props.device.buildNumber ? ` (${props.device.buildNumber})` : ""}`
      : t("connection.unknownValue"),
  },
  {
    key: "model",
    value:
      [props.device.modelIdentifier, props.device.boardConfig]
        .filter(Boolean)
        .join(" · ") || t("connection.unknownValue"),
  },
  {
    key: "serial",
    value: props.device.serialMasked ?? t("connection.unknownValue"),
    mono: true,
  },
  {
    key: "udid",
    value: props.device.udidMasked ?? t("connection.unknownValue"),
    mono: true,
  },
]);
</script>
<template>
  <StudioPanel class="flex items-center gap-6 px-5 py-4">
    <DeviceIllustration class="shrink-0" :width="92" :screen="screen" shadow />
    <div class="min-w-0 flex-1">
      <div class="flex flex-wrap items-center gap-3">
        <h1 class="truncate text-2xl font-semibold tracking-tight">
          {{ device.marketingName }}
        </h1>
        <StatusPill :tone="isReady ? 'success' : 'warning'" dot>{{
          t(`readiness.status.${readiness?.status ?? "needsAttention"}`)
        }}</StatusPill>
      </div>
      <dl
        class="mt-3 grid grid-cols-3 gap-x-6 gap-y-1.5 text-sm max-[1100px]:grid-cols-2"
      >
        <div v-for="row in rows" :key="row.key" class="flex min-w-0 gap-1.5">
          <dt class="shrink-0 text-muted">
            {{ t(`device.fields.${row.key}`) }}
          </dt>
          <dd class="truncate" :class="{ 'font-mono': row.mono }">
            {{ row.value }}
          </dd>
        </div>
      </dl>
      <div v-if="storage" class="mt-4">
        <div
          class="flex h-2 overflow-hidden rounded-full bg-track"
          role="img"
          :aria-label="
            t('studio.storageAvailable', {
              free: storage.free.toFixed(1),
              total: storage.total,
            })
          "
        >
          <span
            v-for="segment in storage.segments"
            :key="segment.key"
            class="h-full w-(--segment-width) bg-(--segment-color) transition-[width] duration-500"
            :style="{
              '--segment-width': `${(segment.gigabytes / (storage.total || 1)) * 100}%`,
              '--segment-color': segment.color,
            }"
          />
        </div>
        <div
          class="mt-1.5 flex flex-wrap items-center gap-x-4 gap-y-1 text-2xs text-muted"
        >
          <span
            v-for="segment in storage.segments"
            :key="segment.key"
            class="flex items-center gap-1.5"
            ><i
              class="size-1.5 rounded-[2px] bg-(--segment-color)"
              :style="{ '--segment-color': segment.color }"
            />{{ t(`studio.storageTypes.${segment.key}`) }}
            <b class="font-medium text-ink"
              >{{ segment.gigabytes.toFixed(1) }} GB</b
            ></span
          >
          <span class="ml-auto">{{
            t("studio.storageAvailable", {
              free: storage.free.toFixed(1),
              total: storage.total,
            })
          }}</span>
          <span v-if="sample">{{ t("studio.sampleStorage") }}</span>
        </div>
      </div>
    </div>
  </StudioPanel>
</template>
