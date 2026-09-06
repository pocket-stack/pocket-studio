<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import type { OperationState } from "../../../shared/composables/useOperations";
import AppIcon from "../../../shared/ui/AppIcon.vue";
import DeviceIllustration from "../../../shared/ui/DeviceIllustration.vue";

const props = defineProps<{
  operation: OperationState | undefined;
  outcome: "success" | "failed" | "cancelled";
  attempt: number;
}>();
const emit = defineEmits<{
  retry: [];
  close: [];
  openStore: [];
  openLogs: [];
}>();
const { t, tm } = useI18n();

const error = computed(() => props.operation?.error);
const recoverySteps = computed(() => {
  const code = error.value?.code;
  if (!code) return [] as string[];
  const steps = tm(`preparation.errors.${code}.recovery`);
  return Array.isArray(steps) ? (steps as string[]) : [];
});
const failedStepId = computed(
  () => props.operation?.steps.find((step) => step.status === "failed")?.id,
);
</script>

<template>
  <div
    :data-outcome="outcome"
    class="flex min-h-0 flex-1 flex-col gap-4 group/result"
  >
    <DeviceIllustration
      v-if="outcome === 'success'"
      :width="190"
      screen="home"
      class="mx-auto my-5"
    />
    <section
      class="flex items-start gap-4 p-6 rounded-lg border border-line bg-surface group-data-[outcome=success]/result:border-0 group-data-[outcome=success]/result:bg-transparent group-data-[outcome=success]/result:px-5 group-data-[outcome=success]/result:py-3"
    >
      <span
        class="flex h-12 w-12 shrink-0 items-center justify-center rounded-full"
        :class="{
          'bg-success/15 text-success': outcome === 'success',
          'bg-danger/15 text-danger': outcome === 'failed',
          'bg-warning/15 text-warning': outcome === 'cancelled',
        }"
      >
        <AppIcon
          :name="
            outcome === 'success'
              ? 'check'
              : outcome === 'failed'
                ? 'cross'
                : 'stop'
          "
          :size="26"
        />
      </span>
      <div class="min-w-0 flex-1">
        <h3 class="text-lg font-semibold">
          <template v-if="outcome === 'failed' && error">{{
            t(`preparation.errors.${error.code}.title`)
          }}</template>
          <template v-else>{{
            t(`preparation.result.${outcome}.title`)
          }}</template>
        </h3>
        <p class="mt-1 text-sm text-muted">
          <template v-if="outcome === 'failed' && error">{{
            t(`preparation.errors.${error.code}.body`)
          }}</template>
          <template v-else>{{
            t(`preparation.result.${outcome}.body`)
          }}</template>
        </p>
        <p
          v-if="outcome === 'failed' && failedStepId"
          class="mt-2 text-xs text-muted"
        >
          {{
            t("preparation.result.failedAt", {
              step: t(`preparation.steps.${failedStepId}.title`),
              attempt,
            })
          }}
        </p>
        <div
          v-if="recoverySteps.length"
          class="mt-4 rounded-lg border border-line bg-raised p-4"
        >
          <p class="text-sm font-medium">
            {{ t("preparation.result.recoveryTitle") }}
          </p>
          <ol class="mt-2 list-decimal space-y-1 pl-5 text-sm text-muted">
            <li v-for="(step, index) in recoverySteps" :key="index">
              {{ step }}
            </li>
          </ol>
        </div>
        <div
          v-if="outcome === 'failed' && error && !error.recoverable"
          class="mt-3 flex items-start gap-2 text-xs text-danger"
        >
          <AppIcon name="warning" :size="14" class="mt-0.5" />
          {{ t("preparation.result.notRecoverable") }}
        </div>
      </div>
    </section>

    <footer
      class="flex flex-wrap items-center justify-between gap-2 group-data-[outcome=success]/result:mt-[15px] group-data-[outcome=success]/result:justify-center"
    >
      <div class="flex gap-2">
        <button
          class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-transparent text-muted enabled:hover:bg-ink/6 enabled:hover:text-ink"
          @click="emit('openLogs')"
        >
          <AppIcon name="logs" :size="16" />
          {{ t("preparation.result.viewLogs") }}
        </button>
      </div>
      <div class="flex gap-2">
        <button
          class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
          @click="emit('close')"
        >
          {{ t("common.close") }}
        </button>
        <button
          v-if="outcome === 'success'"
          class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
          @click="emit('openStore')"
        >
          <AppIcon name="store" :size="16" />
          {{ t("preparation.result.openStore") }}
        </button>
        <button
          v-else-if="outcome === 'cancelled' || error?.recoverable"
          class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
          @click="emit('retry')"
        >
          <AppIcon name="refresh" :size="16" />
          {{ t("preparation.result.retry") }}
        </button>
      </div>
    </footer>
  </div>
</template>
