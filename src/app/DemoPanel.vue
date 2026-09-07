<script setup lang="ts">
import { ref } from "vue";
import { useI18n } from "vue-i18n";

import { useDeviceSession } from "../shared/composables/useDeviceSession";
import { useGateway, type StepId } from "../shared/gateway";
import StudioButton from "../shared/ui/StudioButton.vue";
import StudioSelect from "../shared/ui/StudioSelect.vue";

/**
 * Demo controls stand in for physical actions (plugging a cable, a failing
 * exploit). They call the same gateway the rest of the UI uses.
 */
const { t } = useI18n();
const gateway = useGateway();
const { device, checkReadiness } = useDeviceSession();
async function setJailbroken(value: boolean): Promise<void> {
  await gateway.demo.setJailbroken(value);
  await checkReadiness();
}

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
  <div class="flex flex-col gap-3 text-sm">
    <p class="text-muted">{{ t("demo.hint") }}</p>
    <div class="grid grid-cols-2 gap-2">
      <StudioButton :disabled="!!device" @click="gateway.demo.attachDevice()">
        {{ t("demo.attach") }}
      </StudioButton>
      <StudioButton :disabled="!device" @click="gateway.demo.detachDevice()">
        {{ t("demo.detach") }}
      </StudioButton>
      <StudioButton
        :disabled="!device"
        @click="gateway.demo.setDeviceMode('normal')"
      >
        {{ t("demo.exitDfu") }}
      </StudioButton>
      <StudioButton @click="setJailbroken(true)">
        {{ t("demo.markJailbroken") }}
      </StudioButton>
      <StudioButton @click="setJailbroken(false)">
        {{ t("demo.markStock") }}
      </StudioButton>
    </div>
    <label class="flex flex-col gap-1">
      <span class="text-xs text-muted">{{ t("demo.failNext") }}</span>
      <StudioSelect
        v-model="failStep"
        :label="t('demo.failNext')"
        @change="armFailure"
      >
        <option value="">{{ t("demo.noFailure") }}</option>
        <option v-for="step in failureSteps" :key="step" :value="step">
          {{
            t(
              `${["download", "verify", "install"].includes(step) ? "store.steps" : "preparation.steps"}.${step}.title`,
            )
          }}
        </option>
      </StudioSelect>
    </label>
  </div>
</template>
