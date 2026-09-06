<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { DeviceSummary } from "../../shared/gateway";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import DeviceIllustration from "../../shared/ui/DeviceIllustration.vue";
import StatusPill from "../../shared/ui/StatusPill.vue";
const props = defineProps<{ device: DeviceSummary }>();
const { t } = useI18n();
const { isReady } = useDeviceSession();
const screen = computed(() =>
  props.device.mode === "dfu"
    ? "off"
    : props.device.mode === "recovery"
      ? "recovery"
      : "home",
);
const rows = computed(() => [
  { key: "storage", value: `${props.device.storageGb} GB` },
  { key: "battery", value: `${props.device.batteryPercent}%` },
  {
    key: "os",
    value: `iOS ${props.device.osVersion} (${props.device.buildNumber})`,
  },
  {
    key: "model",
    value: `${props.device.modelIdentifier} · ${props.device.boardConfig}`,
  },
  { key: "serial", value: props.device.serialMasked, mono: true },
  { key: "udid", value: props.device.udidMasked, mono: true },
]);
</script>
<template>
  <section
    class="mb-6 flex min-h-[236px] items-center gap-9 p-0 max-[850px]:gap-[22px] max-[800px]:items-start"
  >
    <div class="relative flex w-[118px] shrink-0 justify-center">
      <DeviceIllustration
        class="relative z-[1] h-[236px] w-[138px] drop-shadow-[2px_6px_5px_#0000000a]"
        :width="114"
        :screen="screen"
      />
      <div
        class="absolute bottom-[7px] h-[7px] w-[82px] rounded-[50%] bg-[#00000012] blur-[4px]"
      />
    </div>
    <div class="min-w-0 flex-1">
      <div class="m-0 flex items-center gap-3 max-[800px]:flex-wrap">
        <h1 class="text-[26px] leading-[1.3] font-semibold tracking-[-0.26px]">
          {{ t("studio.deviceName") }}
        </h1>
        <StatusPill :tone="isReady ? 'success' : 'warning'" dot>{{
          t(isReady ? "studio.pocketReady" : "studio.needsPreparation")
        }}</StatusPill>
      </div>
      <dl
        class="mt-3.5 grid grid-cols-[1fr_1fr_1.5fr] gap-x-8 gap-y-2 max-[1150px]:grid-cols-2 max-[1150px]:gap-x-3.5 max-[1150px]:gap-y-2.5 max-[850px]:grid-cols-1"
      >
        <div
          v-for="row in rows"
          :key="row.key"
          class="flex gap-[5px] text-[13px]"
        >
          <dt class="min-w-auto text-muted">
            {{ t(`device.fields.${row.key}`) }}
          </dt>
          <dd class="text-[13px]" :class="{ 'font-mono': row.mono }">
            {{ row.value }}
          </dd>
        </div>
      </dl>
    </div>
  </section>
</template>
