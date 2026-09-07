<script setup lang="ts">
import IconPhCheck from "~icons/ph/check";
import IconPhWarning from "~icons/ph/warning";
import IconPhX from "~icons/ph/x";
import IconPhStop from "~icons/ph/stop";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useGateway } from "../../../shared/gateway";

import type { OperationState } from "../../../shared/composables/useOperations";
import DeviceIllustration from "../../../shared/ui/DeviceIllustration.vue";
import StudioButton from "../../../shared/ui/StudioButton.vue";
import StudioCallout from "../../../shared/ui/StudioCallout.vue";
import StudioPanel from "../../../shared/ui/StudioPanel.vue";

const props = defineProps<{
  operation: OperationState | undefined;
  outcome: "success" | "failed" | "cancelled";
  attempt: number;
  workflow?: "jailbreak" | "appSync";
}>();
const emit = defineEmits<{
  retry: [];
  recheck: [];
  close: [];
  openStore: [];
  openLogs: [];
}>();
const { t, tm } = useI18n();

const error = computed(() => props.operation?.error);
const needsRestart = computed(
  () => error.value?.code === "appSyncRestartRequired",
);
const recoverySteps = computed(() => {
  const code = error.value?.code;
  if (!code) return [] as string[];
  const steps = tm(`preparation.errors.${code}.recovery`);
  return Array.isArray(steps) ? (steps as string[]) : [];
});
const failedStepId = computed(
  () => props.operation?.steps.find((step) => step.status === "failed")?.id,
);
const canRecheck = computed(
  () =>
    [
      "appSyncInstallFailed",
      "appSyncRestartRequired",
      "appSyncDependencies",
      "appSyncVerificationFailed",
      "sshAuthenticationFailed",
    ].includes(error.value?.code ?? "") ||
    error.value?.code === "verificationUnavailable" ||
    error.value?.code === "rebootTimeout" ||
    failedStepId.value === "verifyJailbreak",
);
const tone = computed(() =>
  props.outcome === "success"
    ? "success"
    : props.outcome === "failed" && !canRecheck.value
      ? "danger"
      : "warning",
);
const icon = computed(() =>
  props.outcome === "success"
    ? IconPhCheck
    : props.outcome === "failed"
      ? canRecheck.value
        ? IconPhWarning
        : IconPhX
      : IconPhStop,
);
</script>

<template>
  <div class="flex h-full min-h-0 flex-col items-center justify-center gap-4">
    <DeviceIllustration
      v-if="outcome === 'success'"
      :width="120"
      screen="home"
      shadow
    />
    <StudioPanel class="w-full max-w-[560px]">
      <div class="flex items-start gap-4">
        <span
          class="flex size-10 shrink-0 items-center justify-center rounded-full"
          :class="{
            'bg-success/12 text-success': tone === 'success',
            'bg-danger/12 text-danger': tone === 'danger',
            'bg-warning/14 text-warning': tone === 'warning',
          }"
        >
          <component :is="icon" width="20" height="20" />
        </span>
        <div class="min-w-0 flex-1">
          <h3 class="text-lg font-semibold">
            <template v-if="outcome === 'failed' && error">{{
              t(`preparation.errors.${error.code}.title`)
            }}</template>
            <template v-else>{{
              t(
                outcome === "success" && workflow === "appSync"
                  ? "preparation.appSync.successTitle"
                  : `preparation.result.${outcome}.title`,
              )
            }}</template>
          </h3>
          <p class="mt-1 text-sm text-muted">
            <template v-if="outcome === 'failed' && error">{{
              t(`preparation.errors.${error.code}.body`)
            }}</template>
            <template v-else>{{
              t(
                outcome === "success" && workflow === "appSync"
                  ? "preparation.appSync.successBody"
                  : `preparation.result.${outcome}.body`,
              )
            }}</template>
          </p>
          <StudioCallout v-if="needsRestart" tone="warning" strong class="mt-3">
            {{ t("preparation.appSync.restartInstructions") }}
          </StudioCallout>
          <p
            v-if="outcome === 'failed' && failedStepId && !needsRestart"
            class="mt-2 text-xs text-muted"
          >
            {{
              t("preparation.result.failedAt", {
                step: t(`preparation.steps.${failedStepId}.title`),
                attempt,
              })
            }}
          </p>
          <p
            v-if="
              error?.diagnostic &&
              ['exploitBootrom', 'bootRamdisk'].includes(failedStepId ?? '')
            "
            class="mt-2 text-sm text-danger"
          >
            {{
              t("preparation.usbDiagnostic", {
                stage: t(`preparation.usbStages.${error.diagnostic.stage}`),
                reason: t(`preparation.usbReasons.${error.diagnostic.reason}`),
              })
            }}
          </p>
          <div
            v-if="recoverySteps.length && !needsRestart"
            class="mt-3 rounded-control bg-ink/4 px-3 py-2"
          >
            <p class="text-xs font-medium">
              {{ t("preparation.result.recoveryTitle") }}
            </p>
            <ol class="mt-1 list-decimal space-y-0.5 pl-4 text-xs text-muted">
              <li v-for="(step, index) in recoverySteps" :key="index">
                {{ step }}
              </li>
            </ol>
          </div>
          <StudioCallout
            v-if="
              outcome === 'failed' &&
              error &&
              !error.recoverable &&
              !needsRestart
            "
            :tone="canRecheck ? 'warning' : 'danger'"
            class="mt-3"
          >
            {{
              t(
                canRecheck
                  ? "preparation.result.recheckHint"
                  : "preparation.result.notRecoverable",
              )
            }}
          </StudioCallout>
        </div>
      </div>
      <footer class="mt-4 flex flex-wrap items-center justify-between gap-2">
        <StudioButton variant="ghost" @click="emit('openLogs')">
          <IconPhFileText width="14" height="14" />
          {{ t("preparation.result.viewLogs") }}
        </StudioButton>
        <div class="flex gap-2">
          <StudioButton @click="emit('close')">
            {{ t("common.close") }}
          </StudioButton>
          <StudioButton
            v-if="outcome === 'failed' && canRecheck"
            variant="primary"
            @click="emit('recheck')"
          >
            <IconPhArrowsClockwise width="14" height="14" />
            {{
              t(
                needsRestart
                  ? "preparation.appSync.recheckAfterRestart"
                  : "preparation.result.recheck",
              )
            }}
          </StudioButton>
          <StudioButton
            v-else-if="
              outcome === 'success' && useGateway().capabilities.packages
            "
            variant="primary"
            @click="emit('openStore')"
          >
            <IconPhStorefront width="14" height="14" />
            {{ t("preparation.result.openStore") }}
          </StudioButton>
          <StudioButton
            v-else-if="outcome === 'cancelled' || error?.recoverable"
            variant="primary"
            @click="emit('retry')"
          >
            <IconPhArrowsClockwise width="14" height="14" />
            {{ t("preparation.result.retry") }}
          </StudioButton>
        </div>
      </footer>
    </StudioPanel>
  </div>
</template>
