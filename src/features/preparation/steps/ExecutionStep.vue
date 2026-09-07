<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";

import {
  operationProgress,
  type OperationState,
} from "../../../shared/composables/useOperations";
import ProgressBar from "../../../shared/ui/ProgressBar.vue";
import StepList from "../../../shared/ui/StepList.vue";

const props = defineProps<{ operation: OperationState }>();
const emit = defineEmits<{ cancel: [] }>();
const { t } = useI18n();

const now = ref(Date.now());
let timer: number | undefined;
onMounted(() => {
  timer = window.setInterval(() => (now.value = Date.now()), 1000);
});
onBeforeUnmount(() => timer && window.clearInterval(timer));

const elapsed = computed(() =>
  Math.floor((now.value - props.operation.startedAt) / 1000),
);
const percent = computed(() => operationProgress(props.operation));
const currentStep = computed(() =>
  props.operation.steps.find(
    (step) => step.id === props.operation.currentStepId,
  ),
);
const canCancel = computed(() => currentStep.value?.cancellable === true);
const passedPointOfNoReturn = computed(() =>
  props.operation.steps.some(
    (step) => step.pointOfNoReturn && step.status !== "pending",
  ),
);

function formatElapsed(seconds: number): string {
  const minutes = Math.floor(seconds / 60);
  return `${String(minutes).padStart(2, "0")}:${String(seconds % 60).padStart(2, "0")}`;
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-4">
    <div class="p-5 rounded-lg border border-line bg-surface">
      <div class="flex items-center justify-between gap-4">
        <div class="min-w-0">
          <h3 class="text-base font-semibold">
            {{
              currentStep
                ? t(`preparation.steps.${currentStep.id}.title`)
                : t("preparation.execution.finishing")
            }}
          </h3>
          <p class="mt-1 text-sm text-muted">
            {{
              currentStep ? t(`preparation.steps.${currentStep.id}.detail`) : ""
            }}
          </p>
        </div>
        <div class="text-right">
          <div class="font-mono text-2xl font-semibold tabular-nums">
            {{ percent }}%
          </div>
          <div class="text-xs text-muted">
            {{
              t("preparation.execution.elapsed", {
                time: formatElapsed(elapsed),
              })
            }}
          </div>
        </div>
      </div>
      <ProgressBar class="mt-4" :percent="percent" active />
    </div>

    <div
      class="flex items-start gap-3 rounded-xl border p-4 text-sm"
      :class="
        passedPointOfNoReturn
          ? 'border-danger/50 bg-danger/8'
          : 'border-warning/40 bg-warning/8'
      "
    >
      <IconStudioWarning
        class="mt-0.5"
        :class="passedPointOfNoReturn ? 'text-danger' : 'text-warning'"
      />
      <div>
        <p class="font-medium">
          {{ t("preparation.execution.keepConnected.title") }}
        </p>
        <p class="mt-0.5 text-muted">
          {{
            passedPointOfNoReturn
              ? t("preparation.execution.keepConnected.afterPointOfNoReturn")
              : t("preparation.execution.keepConnected.body")
          }}
        </p>
      </div>
    </div>

    <div
      class="min-h-0 flex-1 overflow-y-auto p-3 [scrollbar-width:thin] [scrollbar-color:var(--color-line)_transparent] rounded-lg border border-line bg-surface"
    >
      <StepList :steps="operation.steps" label-prefix="preparation.steps" />
    </div>

    <footer class="flex items-center justify-between">
      <button
        class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-danger bg-transparent text-danger enabled:hover:bg-danger/10"
        :disabled="!canCancel"
        @click="emit('cancel')"
      >
        <IconStudioStop width="16" height="16" />
        {{ t("preparation.execution.cancel") }}
      </button>
      <span class="text-xs text-muted">
        {{
          canCancel
            ? t("preparation.execution.cancelHint")
            : t("preparation.execution.cannotCancelNow")
        }}
      </span>
    </footer>
  </div>
</template>
