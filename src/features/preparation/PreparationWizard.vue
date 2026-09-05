<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import AppIcon from "../../shared/ui/AppIcon.vue";
import DfuGuideStep from "./steps/DfuGuideStep.vue";
import DisclaimerStep from "./steps/DisclaimerStep.vue";
import ExecutionStep from "./steps/ExecutionStep.vue";
import OverviewStep from "./steps/OverviewStep.vue";
import ResultStep from "./steps/ResultStep.vue";
import RiskStep from "./steps/RiskStep.vue";
import { usePreparation, type PreparationStage } from "./usePreparation";

const emit = defineEmits<{ openStore: []; openLogs: [] }>();
const { t } = useI18n();
const preparation = usePreparation();

const milestones: Array<{ id: string; stages: PreparationStage[] }> = [
  { id: "overview", stages: ["overview"] },
  { id: "risks", stages: ["risks"] },
  { id: "disclaimer", stages: ["disclaimer", "starting"] },
  { id: "dfu", stages: ["awaitingDfu"] },
  { id: "execute", stages: ["running"] },
  { id: "result", stages: ["success", "failed", "cancelled"] },
];

const activeIndex = computed(() =>
  milestones.findIndex((item) => item.stages.includes(preparation.stage.value)),
);

const closable = computed(() => {
  const stage = preparation.stage.value;
  return !["starting", "awaitingDfu", "running"].includes(stage);
});

function handleClose(): void {
  if (closable.value) preparation.close();
}

function openStore(): void {
  preparation.close();
  emit("openStore");
}

function openLogs(): void {
  preparation.close();
  emit("openLogs");
}
</script>

<template>
  <div
    v-if="preparation.stage.value !== 'closed'"
    class="fixed inset-0 z-40 flex items-center justify-center bg-ink/40 p-4 backdrop-blur-sm"
  >
    <div
      class="rise flex h-full max-h-[860px] w-full max-w-5xl flex-col overflow-hidden rounded-2xl border border-line bg-canvas shadow-2xl"
    >
      <header
        class="flex items-center gap-4 border-b border-line bg-surface px-6 py-4"
      >
        <div class="min-w-0">
          <h2 class="text-base font-semibold">{{ t("preparation.title") }}</h2>
          <p class="truncate text-xs text-muted">
            {{ t("preparation.subtitle") }}
          </p>
        </div>
        <ol class="ml-auto hidden items-center gap-1 md:flex">
          <li
            v-for="(milestone, index) in milestones"
            :key="milestone.id"
            class="flex items-center gap-1 text-xs"
            :class="
              index === activeIndex
                ? 'text-signal font-semibold'
                : index < activeIndex
                  ? 'text-success'
                  : 'text-muted'
            "
          >
            <span
              class="flex h-5 w-5 items-center justify-center rounded-full border text-[10px]"
              :class="
                index === activeIndex
                  ? 'border-signal'
                  : index < activeIndex
                    ? 'border-success'
                    : 'border-line'
              "
            >
              <AppIcon v-if="index < activeIndex" name="check" :size="11" />
              <span v-else>{{ index + 1 }}</span>
            </span>
            <span>{{ t(`preparation.milestones.${milestone.id}`) }}</span>
            <span
              v-if="index < milestones.length - 1"
              class="mx-1 h-px w-4 bg-line"
            />
          </li>
        </ol>
        <button
          class="btn btn-ghost -mr-2 px-2"
          :disabled="!closable"
          :title="t('common.close')"
          @click="handleClose"
        >
          <AppIcon name="cross" :size="18" />
        </button>
      </header>

      <div class="scroll-thin flex min-h-0 flex-1 flex-col overflow-y-auto p-6">
        <div v-if="preparation.planError.value" class="card p-6 text-sm">
          <p class="font-medium text-danger">
            {{ t("preparation.planFailed") }}
          </p>
          <button class="btn btn-secondary mt-4" @click="preparation.close()">
            {{ t("common.close") }}
          </button>
        </div>
        <div
          v-else-if="!preparation.plan.value"
          class="flex flex-1 items-center justify-center gap-3 text-sm text-muted"
        >
          <span
            class="spin block h-4 w-4 rounded-full border-2 border-signal border-t-transparent"
          />
          {{ t("preparation.planning") }}
        </div>
        <OverviewStep
          v-else-if="preparation.stage.value === 'overview'"
          :plan="preparation.plan.value"
          :confirmed="preparation.confirmedPrerequisites.value"
          :all-confirmed="preparation.allPrerequisitesConfirmed.value"
          @toggle="preparation.togglePrerequisite"
          @next="preparation.proceedToRisks"
          @cancel="preparation.close"
        />
        <RiskStep
          v-else-if="preparation.stage.value === 'risks'"
          :plan="preparation.plan.value"
          :acknowledged="preparation.acknowledgedRisks.value"
          :all-acknowledged="preparation.allRisksAcknowledged.value"
          @toggle="preparation.toggleRisk"
          @next="preparation.acknowledgeRisks"
          @back="preparation.backToOverview"
        />
        <DisclaimerStep
          v-else-if="preparation.stage.value === 'disclaimer'"
          :plan="preparation.plan.value"
          :start-error="preparation.startError.value"
          @accept="preparation.acceptDisclaimer"
          @back="preparation.backToRisks"
        />
        <div
          v-else-if="preparation.stage.value === 'starting'"
          class="flex flex-1 items-center justify-center gap-3 text-sm text-muted"
        >
          <span
            class="spin block h-4 w-4 rounded-full border-2 border-signal border-t-transparent"
          />
          {{ t("preparation.starting") }}
        </div>
        <DfuGuideStep
          v-else-if="preparation.stage.value === 'awaitingDfu'"
          @cancel="preparation.cancel"
        />
        <ExecutionStep
          v-else-if="
            preparation.stage.value === 'running' && preparation.operation.value
          "
          :operation="preparation.operation.value"
          @cancel="preparation.cancel"
        />
        <ResultStep
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
          @retry="preparation.retry"
          @close="preparation.close"
          @open-store="openStore"
          @open-logs="openLogs"
        />
      </div>
    </div>
  </div>
</template>
