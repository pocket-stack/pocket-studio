<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import type { DeviceSummary } from "../../shared/gateway";
import DeviceIllustration from "../../shared/ui/DeviceIllustration.vue";
import AppIcon from "../../shared/ui/AppIcon.vue";
import StatusPill from "../../shared/ui/StatusPill.vue";

const props = defineProps<{ device: DeviceSummary }>();
const { t } = useI18n();

const modeTone = computed(() => {
  switch (props.device.mode) {
    case "dfu":
      return "warning";
    case "recovery":
      return "info";
    default:
      return "success";
  }
});

const screen = computed(() => {
  switch (props.device.mode) {
    case "dfu":
      return "off";
    case "recovery":
      return "recovery";
    default:
      return "home";
  }
});

const rows = computed(() => [
  {
    label: t("device.fields.model"),
    value: `${props.device.modelIdentifier} · ${props.device.boardConfig}`,
  },
  { label: t("device.fields.chip"), value: props.device.chip },
  {
    label: t("device.fields.os"),
    value: `iOS ${props.device.osVersion} (${props.device.buildNumber})`,
  },
  { label: t("device.fields.storage"), value: `${props.device.storageGb} GB` },
  {
    label: t("device.fields.udid"),
    value: props.device.udidMasked,
    mono: true,
  },
  {
    label: t("device.fields.ecid"),
    value: props.device.ecidMasked,
    mono: true,
  },
  {
    label: t("device.fields.serial"),
    value: props.device.serialMasked,
    mono: true,
  },
]);
</script>

<template>
  <section class="card flex gap-6 p-6">
    <div class="hidden shrink-0 md:block">
      <DeviceIllustration :width="120" :screen="screen" cable />
    </div>
    <div class="min-w-0 flex-1">
      <div class="flex flex-wrap items-center gap-2">
        <h2 class="text-xl font-semibold">{{ device.marketingName }}</h2>
        <StatusPill :tone="modeTone" dot>{{
          t(`device.mode.${device.mode}`)
        }}</StatusPill>
        <StatusPill tone="neutral">
          <AppIcon name="usb" :size="12" />
          {{ t(`device.transport.${device.transport}`) }}
        </StatusPill>
      </div>
      <p class="mt-1 text-sm text-muted">
        {{ t("device.identifiedAs", { platform: "iOS" }) }}
      </p>
      <dl class="mt-4 grid grid-cols-1 gap-x-8 gap-y-2 text-sm sm:grid-cols-2">
        <div
          v-for="row in rows"
          :key="row.label"
          class="flex justify-between gap-4 border-b border-line/60 py-1.5"
        >
          <dt class="text-muted">{{ row.label }}</dt>
          <dd
            class="truncate text-right"
            :class="row.mono ? 'font-mono text-xs' : ''"
          >
            {{ row.value }}
          </dd>
        </div>
        <div class="flex justify-between gap-4 border-b border-line/60 py-1.5">
          <dt class="text-muted">{{ t("device.fields.battery") }}</dt>
          <dd class="flex items-center gap-2">
            <span class="h-1.5 w-16 overflow-hidden rounded-full bg-ink/10">
              <span
                class="block h-full rounded-full"
                :class="
                  device.batteryPercent >= 50 ? 'bg-success' : 'bg-warning'
                "
                :style="{ width: `${device.batteryPercent}%` }"
              />
            </span>
            {{ device.batteryPercent }}%
          </dd>
        </div>
      </dl>
      <p class="mt-3 text-xs text-muted">{{ t("device.maskedNote") }}</p>
    </div>
  </section>
</template>
