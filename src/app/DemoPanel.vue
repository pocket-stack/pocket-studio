<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";

import { useDeviceSession } from "../shared/composables/useDeviceSession";
import { useGateway, type StepId } from "../shared/gateway";
import AppIcon from "../shared/ui/AppIcon.vue";

/**
 * Demo controls stand in for physical actions (plugging a cable, a failing
 * exploit). They call the same gateway the rest of the UI uses.
 */
const { t } = useI18n();
const gateway = useGateway();
const { device } = useDeviceSession();

const failStep = ref<StepId | "">("");
const failureSteps: StepId[] = [
  "enterDfu",
  "exploitBootrom",
  "fetchResources",
  "bootRamdisk",
  "installUntether",
  "verifyJailbreak",
  "download",
  "verify",
  "install",
];

async function armFailure(): Promise<void> {
  await gateway.demo.failNextStep(
    failStep.value === "" ? null : failStep.value,
  );
}
</script>

<template>
  <div class="flex flex-col gap-3 text-xs">
    <p class="flex items-center gap-2 font-semibold text-muted">
      <AppIcon name="lab" :size="14" />
      {{ t("demo.title") }}
    </p>
    <p class="text-muted">{{ t("demo.hint") }}</p>
    <div class="grid grid-cols-2 gap-1.5">
      <button
        class="btn btn-secondary px-2 py-1 text-xs"
        :disabled="!!device"
        @click="gateway.demo.attachDevice()"
      >
        {{ t("demo.attach") }}
      </button>
      <button
        class="btn btn-secondary px-2 py-1 text-xs"
        :disabled="!device"
        @click="gateway.demo.detachDevice()"
      >
        {{ t("demo.detach") }}
      </button>
      <button
        class="btn btn-secondary px-2 py-1 text-xs"
        :disabled="!device"
        @click="gateway.demo.setDeviceMode('dfu')"
      >
        {{ t("demo.enterDfu") }}
      </button>
      <button
        class="btn btn-secondary px-2 py-1 text-xs"
        :disabled="!device"
        @click="gateway.demo.setDeviceMode('normal')"
      >
        {{ t("demo.exitDfu") }}
      </button>
      <button
        class="btn btn-secondary px-2 py-1 text-xs"
        @click="gateway.demo.setJailbroken(true)"
      >
        {{ t("demo.markJailbroken") }}
      </button>
      <button
        class="btn btn-secondary px-2 py-1 text-xs"
        @click="gateway.demo.setJailbroken(false)"
      >
        {{ t("demo.markStock") }}
      </button>
    </div>
    <label class="flex flex-col gap-1">
      <span class="text-muted">{{ t("demo.failNext") }}</span>
      <select
        v-model="failStep"
        class="field py-1 text-xs"
        @change="armFailure"
      >
        <option value="">{{ t("demo.noFailure") }}</option>
        <option v-for="step in failureSteps" :key="step" :value="step">
          {{ step }}
        </option>
      </select>
    </label>
  </div>
</template>
