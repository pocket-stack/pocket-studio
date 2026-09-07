<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { usePreparation } from "./usePreparation";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import {
  operationProgress,
  operationStepCeiling,
} from "../../shared/composables/useOperations";
import ProgressBar from "../../shared/ui/ProgressBar.vue";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioPanel from "../../shared/ui/StudioPanel.vue";
import DfuGuideStep from "./steps/DfuGuideStep.vue";
import ExecutionScene from "./steps/ExecutionScene.vue";
import ResultStep from "./steps/ResultStep.vue";
import OverviewStep from "./steps/OverviewStep.vue";
const emit = defineEmits<{ openStore: []; openLogs: []; showConditions: [] }>();
const { t } = useI18n();
const preparation = usePreparation();
const operation = computed(() => preparation.operation.value);
const currentStep = computed(() =>
  operation.value?.steps.find(
    (step) => step.id === operation.value?.currentStepId,
  ),
);
const canCancel = computed(() => currentStep.value?.cancellable ?? false);
const isPreflight = computed(() =>
  ["overview", "risks", "disclaimer"].includes(preparation.stage.value),
);
const progress = computed(() =>
  operation.value ? operationProgress(operation.value) : 0,
);
const ceiling = computed(() =>
  operation.value ? operationStepCeiling(operation.value) : 0,
);
const passedPointOfNoReturn = computed(
  () =>
    operation.value?.steps.some(
      (step) => step.pointOfNoReturn && step.status !== "pending",
    ) ?? false,
);
const groupedSteps = computed(() => {
  const steps = operation.value?.steps ?? [];
  const groups = [
    { key: "resources", ids: ["fetchResources", "buildRamdisk"] },
    {
      key: preparation.plan.value?.entryMode === "dfu" ? "exploit" : "dfu",
      ids: ["enterDfu", "exploitBootrom"],
    },
    {
      key: "write",
      ids: ["bootRamdisk", "mountFilesystem", "installUntether"],
    },
    { key: "verify", ids: ["rebootDevice", "verifyJailbreak"] },
    {
      key: "appSync",
      ids: [
        "connectAppSync",
        "installAppSync",
        "activateAppSync",
        "verifyAppSync",
      ],
    },
  ];
  return [
    { key: "connect", status: "done", percent: 100 },
    { key: "consent", status: "done", percent: 100 },
    ...groups
      .filter((group) =>
        group.ids.some((id) => steps.some((step) => step.id === id)),
      )
      .map((group) => {
        const members = steps.filter((step) => group.ids.includes(step.id));
        const total = members.reduce(
          (sum, step) => sum + step.estimatedSeconds,
          0,
        );
        const percent = total
          ? Math.round(
              members.reduce(
                (sum, step) => sum + step.estimatedSeconds * step.percent,
                0,
              ) / total,
            )
          : 0;
        const status = members.some((step) => step.status === "failed")
          ? group.key === "appSync" &&
            operation.value?.error?.code === "appSyncRestartRequired"
            ? "restartRequired"
            : "failed"
          : members.some((step) => step.status === "cancelled")
            ? "cancelled"
            : members.length && members.every((step) => step.status === "done")
              ? "done"
              : members.some(
                    (step) =>
                      step.status === "running" || step.status === "done",
                  )
                ? "running"
                : "pending";
        return { key: group.key, status, percent };
      }),
  ];
});
function openStore(): void {
  preparation.close();
  emit("openStore");
}
function recheck(): void {
  preparation.close();
  emit("showConditions");
  void useDeviceSession().refresh();
}
</script>
<template>
  <div v-if="preparation.planError.value" class="flex flex-col gap-3">
    <StudioCallout tone="danger" :title="t('preparation.planFailed')">
      {{ t(`preparation.startErrors.${preparation.planError.value}`) }}
    </StudioCallout>
    <StudioButton class="self-start" @click="preparation.close">
      {{ t("common.close") }}
    </StudioButton>
  </div>
  <div
    v-else-if="!preparation.plan.value"
    class="flex h-full items-center justify-center gap-2 text-sm text-muted"
  >
    <IconSvgSpinnersRingResize width="16" height="16" />
    {{ t("preparation.planning") }}
  </div>
  <OverviewStep
    v-else-if="isPreflight"
    :plan="preparation.plan.value"
    :confirmed="preparation.confirmedPrerequisites.value"
    :all-confirmed="preparation.allPrerequisitesConfirmed.value"
    :ssh-password="preparation.sshPassword.value"
    @update:ssh-password="preparation.setSshPassword"
    @toggle="preparation.togglePrerequisite"
    @next="preparation.proceedToRisks"
    @cancel="preparation.close"
  />
  <div v-else class="flex h-full min-h-0 gap-4">
    <section class="flex min-h-0 min-w-0 flex-1 flex-col">
      <DfuGuideStep
        v-if="preparation.stage.value === 'awaitingDfu'"
        @cancel="preparation.cancel"
      />
      <template
        v-else-if="['starting', 'running'].includes(preparation.stage.value)"
      >
        <header class="flex items-center justify-between gap-3">
          <h1 class="text-xl font-semibold">
            {{
              currentStep
                ? t(`preparation.steps.${currentStep.id}.title`)
                : t("preparation.starting")
            }}
          </h1>
          <span
            class="rounded-full bg-info/12 px-2.5 py-px text-xs font-semibold text-info"
            >{{ t("preparation.execution.mode.auto") }}</span
          >
        </header>
        <div
          class="flex min-h-0 flex-1 flex-col items-center justify-center gap-4"
        >
          <ExecutionScene :step-id="currentStep?.id" :width="124" />
          <p class="max-w-[420px] text-center text-sm">
            {{
              currentStep
                ? t(`preparation.steps.${currentStep.id}.detail`)
                : t("preparation.starting")
            }}
          </p>
          <div class="flex w-full max-w-[340px] items-center gap-3">
            <ProgressBar
              v-if="operation"
              :percent="progress"
              :trickle-to="ceiling"
              active
            />
            <span class="text-xs text-muted tabular-nums">{{ progress }}%</span>
          </div>
          <StudioCallout
            :tone="passedPointOfNoReturn ? 'danger' : 'warning'"
            :title="t('preparation.execution.keepConnected.title')"
            class="max-w-[460px]"
          >
            {{
              passedPointOfNoReturn
                ? t("preparation.execution.keepConnected.afterPointOfNoReturn")
                : t("preparation.execution.keepConnected.body")
            }}
          </StudioCallout>
        </div>
      </template>
      <ResultStep
        v-else
        :operation="operation"
        :outcome="
          preparation.stage.value === 'success'
            ? 'success'
            : preparation.stage.value === 'failed'
              ? 'failed'
              : 'cancelled'
        "
        :attempt="preparation.attempt.value"
        :workflow="preparation.plan.value?.workflow"
        @retry="preparation.retry"
        @recheck="recheck"
        @close="preparation.close"
        @open-store="openStore"
        @open-logs="emit('openLogs')"
      />
    </section>
    <StudioPanel
      as="aside"
      :padded="false"
      class="flex w-[268px] shrink-0 flex-col"
    >
      <h2 class="px-4 pt-3 pb-1 text-sm font-semibold">
        {{ t("preparation.overview.stepsHeading") }}
      </h2>
      <ol class="flex-1 px-2">
        <li
          v-for="(step, index) in groupedSteps"
          :key="step.key"
          :data-status="step.status"
          class="flex items-center gap-2.5 rounded-control px-2 py-1.5 text-sm data-[status=failed]:text-danger data-[status=pending]:text-muted data-[status=restartRequired]:text-warning data-[status=running]:bg-signal/8"
        >
          <span
            class="grid size-4 shrink-0 place-items-center text-2xs text-muted"
          >
            <IconPhCheckCircleFill
              v-if="step.status === 'done'"
              width="15"
              height="15"
              class="text-success"
            />
            <IconPhWarningFill
              v-else-if="step.status === 'restartRequired'"
              width="15"
              height="15"
              class="text-warning"
            />
            <IconPhXCircleFill
              v-else-if="step.status === 'failed'"
              width="15"
              height="15"
              class="text-danger"
            />
            <IconSvgSpinners90Ring
              v-else-if="step.status === 'running'"
              width="13"
              height="13"
              class="text-signal"
            />
            <span v-else>{{ index + 1 }}</span>
          </span>
          <div class="min-w-0 flex-1">
            <span class="block truncate">{{
              t(`studio.flowSteps.${step.key}`)
            }}</span>
            <span
              v-if="step.status === 'running' && currentStep"
              class="block truncate text-2xs text-muted"
              >{{ t(`preparation.steps.${currentStep.id}.title`) }}</span
            >
          </div>
          <small class="shrink-0 text-2xs text-muted tabular-nums">{{
            step.status === "running"
              ? `${step.percent}%`
              : t(`studio.flowStatus.${step.status}`)
          }}</small>
        </li>
      </ol>
      <div class="flex flex-col gap-2.5 px-4 pt-2 pb-3">
        <div class="flex items-center gap-2">
          <ProgressBar
            :percent="progress"
            :trickle-to="ceiling"
            :active="operation?.status === 'running'"
            compact
          />
          <span class="text-2xs text-muted tabular-nums">{{ progress }}%</span>
        </div>
        <StudioButton
          v-if="operation?.status === 'running'"
          variant="danger"
          size="sm"
          class="self-start"
          :disabled="!canCancel"
          @click="preparation.cancel"
        >
          <IconPhStop width="12" height="12" />{{ t("common.cancel") }}
        </StudioButton>
      </div>
    </StudioPanel>
  </div>
</template>
