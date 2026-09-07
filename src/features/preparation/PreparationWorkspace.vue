<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { usePreparation } from "./usePreparation";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { useLogMessage } from "../../shared/composables/useLogMessage";
import { useOperationLog } from "../../shared/composables/useOperationLog";
import { operationProgress } from "../../shared/composables/useOperations";
import DeviceIllustration from "../../shared/ui/DeviceIllustration.vue";
import ProgressBar from "../../shared/ui/ProgressBar.vue";
import DfuGuideStep from "./steps/DfuGuideStep.vue";
import ResultStep from "./steps/ResultStep.vue";
import OverviewStep from "./steps/OverviewStep.vue";
import { useGateway } from "../../shared/gateway";
const emit = defineEmits<{ openStore: []; openLogs: []; showConditions: [] }>();
const { t, d } = useI18n();
const renderLog = useLogMessage();
const preparation = usePreparation();
const log = useOperationLog();
const currentStep = computed(() =>
  preparation.operation.value?.steps.find(
    (step) => step.id === preparation.operation.value?.currentStepId,
  ),
);
const canCancel = computed(() => currentStep.value?.cancellable ?? false);
const isPreflight = computed(() =>
  ["overview", "risks", "disclaimer"].includes(preparation.stage.value),
);
const groupedSteps = computed(() => {
  const steps = preparation.operation.value?.steps ?? [];
  const groups = [
    {
      key: "resources",
      ids: ["fetchResources", "buildRamdisk"],
    },
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
            preparation.operation.value?.error?.code ===
              "appSyncRestartRequired"
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
const flowLogs = computed(() =>
  log.entries.value
    .filter((entry) =>
      preparation.operation.value
        ? entry.operationId === preparation.operation.value.id
        : entry.source === "preparation",
    )
    .slice(-5),
);
function openStore(): void {
  preparation.close();
  emit("openStore");
}
function recheck(): void {
  preparation.close();
  emit("showConditions");
  void useDeviceSession().refresh();
}
function openLogs(): void {
  emit("openLogs");
}
</script>
<template>
  <p
    v-if="useGateway().capabilities.demo"
    class="mb-4 rounded-lg border border-line bg-raised p-3 text-sm text-muted"
  >
    {{ t("preparation.demoNotice") }}
  </p>
  <section v-if="preparation.planError.value" class="p-6">
    <p class="text-danger">{{ t("preparation.planFailed") }}</p>
    <p class="mt-2 text-sm text-muted">
      {{ t(`preparation.startErrors.${preparation.planError.value}`) }}
    </p>
    <button
      class="mt-4 inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
      @click="preparation.close"
    >
      {{ t("common.close") }}
    </button>
  </section>
  <div
    v-else-if="!preparation.plan.value"
    class="flex min-h-[250px] items-center justify-center gap-2.5 text-[12px] text-muted"
  >
    {{ t("preparation.planning") }}
  </div>
  <section v-else-if="isPreflight" class="max-w-[1000px]">
    <OverviewStep
      :plan="preparation.plan.value"
      :confirmed="preparation.confirmedPrerequisites.value"
      :all-confirmed="preparation.allPrerequisitesConfirmed.value"
      :ssh-password="preparation.sshPassword.value"
      @update:ssh-password="preparation.setSshPassword"
      @toggle="preparation.togglePrerequisite"
      @next="preparation.proceedToRisks"
      @cancel="preparation.close"
    />
  </section>
  <div
    v-else
    class="flex min-h-[580px] flex-1 gap-8 max-[1150px]:gap-5 max-[850px]:flex-col [@media(min-height:820px)]:min-h-[680px]"
  >
    <div class="min-w-0 flex-1">
      <h1
        v-if="preparation.stage.value === 'awaitingDfu'"
        class="mb-4 text-[22px] font-semibold"
      >
        {{ t("preparation.steps.enterDfu.title") }}
      </h1>
      <DfuGuideStep
        v-if="preparation.stage.value === 'awaitingDfu'"
        @cancel="preparation.cancel"
      /><template
        v-else-if="['starting', 'running'].includes(preparation.stage.value)"
        ><header class="flex items-center justify-between gap-[15px]">
          <h1 class="text-[22px] font-semibold">
            {{
              currentStep
                ? t(`preparation.steps.${currentStep.id}.title`)
                : t("preparation.starting")
            }}
          </h1>
          <span class="text-[13px] text-muted">{{
            preparation.operation.value
              ? `${operationProgress(preparation.operation.value)}%`
              : ""
          }}</span>
        </header>
        <div class="relative mx-auto mt-[30px] mb-[18px] flex justify-center">
          <DeviceIllustration
            :width="220"
            :screen="currentStep?.id === 'rebootDevice' ? 'apple' : 'off'"
            cable
          /><img
            v-if="currentStep?.id !== 'rebootDevice'"
            src="/vectors/indicators/device-spinner.svg"
            alt=""
            class="absolute top-[43%] left-[calc(50%-13px)] size-[26px] motion-safe:animate-studio-spin"
          />
        </div>
        <p class="mb-5 text-center text-[16px]">
          {{
            currentStep
              ? t(`preparation.steps.${currentStep.id}.detail`)
              : t("preparation.starting")
          }}
        </p>
        <ProgressBar
          v-if="preparation.operation.value"
          class="max-w-sm mx-auto"
          :percent="operationProgress(preparation.operation.value)"
          active
        />
        <p class="mt-[15px] text-center text-[12px] text-muted">
          {{ t("preparation.execution.keepConnected.title") }}
        </p></template
      ><ResultStep
        v-else
        :operation="preparation.operation.value"
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
        @open-logs="openLogs"
      />
    </div>
    <aside
      class="flex w-80 flex-col border-l border-line pl-[30px] max-[1150px]:w-[270px] max-[1150px]:pl-5 max-[850px]:w-full max-[850px]:border-l-0 max-[850px]:pl-0"
    >
      <h2 class="mb-3 text-[16px] font-semibold">
        {{ t("preparation.overview.stepsHeading") }}
      </h2>
      <ol class="mt-1.5">
        <li
          v-for="(step, index) in groupedSteps"
          :key="step.key"
          :data-status="step.status"
          class="group/flow-step flex items-start gap-2.5 border-b border-line px-1.5 py-3.5 text-[12px] data-[status=running]:rounded data-[status=running]:bg-track data-[status=pending]:text-muted data-[status=failed]:text-danger data-[status=restartRequired]:text-warning"
        >
          <span
            class="grid h-[18px] w-4 place-items-center text-[10px] text-muted group-data-[status=done]/flow-step:text-success"
            ><IconPhCheck
              v-if="step.status === 'done'"
              width="14"
              height="14"
            /><IconPhWarning
              v-else-if="step.status === 'restartRequired'"
              width="14"
              height="14"
              class="text-warning"
            /><IconPhX
              v-else-if="step.status === 'failed'"
              width="14"
              height="14"
            /><IconSvgSpinners90Ring
              v-else-if="step.status === 'running'"
              width="12"
              height="12"
              class="text-signal"
            /><span v-else>{{ index + 1 }}</span></span
          >
          <div class="flex-1">
            <span>{{ t(`studio.flowSteps.${step.key}`) }}</span>
            <p
              v-if="step.status === 'running' && currentStep"
              class="mt-[7px] text-[10px] leading-[1.7] text-muted"
            >
              {{ t(`preparation.steps.${currentStep.id}.title`) }}
            </p>
          </div>
          <small class="mt-px text-[10px] whitespace-nowrap text-muted">{{
            step.status === "running"
              ? `${step.percent}%`
              : t(`studio.flowStatus.${step.status}`)
          }}</small>
        </li>
      </ol>
      <div class="flex items-center gap-3 mt-5">
        <ProgressBar
          :percent="
            preparation.operation.value
              ? operationProgress(preparation.operation.value)
              : 0
          "
          compact
        /><span class="text-[10px] leading-[1.333333]"
          >{{
            preparation.operation.value
              ? operationProgress(preparation.operation.value)
              : 0
          }}%</span
        >
      </div>
      <div
        class="mt-auto mb-3.5 rounded-md border border-line p-2.5 font-mono text-[10px] leading-[1.8] text-muted max-[850px]:mt-5"
      >
        <p v-for="entry in flowLogs" :key="entry.id">
          <time>{{ d(entry.timestamp, "time") }}</time>
          {{ renderLog(entry) }}
        </p>
      </div>
      <button
        v-if="preparation.operation.value?.status === 'running'"
        class="self-start inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
        :disabled="!canCancel"
        @click="preparation.cancel"
      >
        {{ t("common.cancel") }}
      </button>
    </aside>
  </div>
</template>
