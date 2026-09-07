<script setup lang="ts">
import { useI18n } from "vue-i18n";

import type { StepState } from "../composables/useOperations";
import ProgressBar from "./ProgressBar.vue";

defineProps<{ steps: readonly StepState[]; labelPrefix: string }>();
const { t } = useI18n();
</script>

<template>
  <ol class="flex flex-col gap-0.5">
    <li
      v-for="(step, index) in steps"
      :key="step.id"
      class="flex items-start gap-2.5 rounded-control px-2 py-1.5 transition-colors"
      :class="{
        'bg-signal/8': step.status === 'running',
        'opacity-50': step.status === 'pending' || step.status === 'skipped',
      }"
    >
      <span
        class="mt-px flex size-5 shrink-0 items-center justify-center rounded-full text-2xs font-semibold"
        :class="{
          'bg-ink/8 text-muted':
            step.status === 'pending' || step.status === 'skipped',
          'bg-signal text-on-signal': step.status === 'running',
          'bg-success text-white': step.status === 'done',
          'bg-danger text-white': step.status === 'failed',
          'bg-warning/15 text-warning': step.status === 'cancelled',
        }"
      >
        <IconPhCheck v-if="step.status === 'done'" width="12" height="12" />
        <IconPhX v-else-if="step.status === 'failed'" width="12" height="12" />
        <IconSvgSpinners90Ring
          v-else-if="step.status === 'running'"
          width="11"
          height="11"
          class="text-on-signal"
        />
        <span v-else>{{ index + 1 }}</span>
      </span>
      <div class="min-w-0 flex-1">
        <div class="flex items-center justify-between gap-2">
          <span class="text-sm font-medium">{{
            t(`${labelPrefix}.${step.id}.title`)
          }}</span>
          <span class="flex items-center gap-2 text-xs text-muted">
            <span
              v-if="step.pointOfNoReturn"
              class="text-danger"
              :title="t('operation.pointOfNoReturn')"
            >
              <IconPhWarning width="13" height="13" />
            </span>
            <span
              v-if="!step.cancellable && step.status !== 'done'"
              :title="t('operation.notCancellable')"
            >
              <IconPhShieldCheck width="13" height="13" />
            </span>
            <span v-if="step.status === 'running'">{{
              step.indeterminate
                ? t("store.actions.working")
                : `${step.percent}%`
            }}</span>
            <span v-else-if="step.status === 'pending'"
              >~{{ step.estimatedSeconds }}s</span
            >
          </span>
        </div>
        <p class="text-xs text-muted">
          {{ t(`${labelPrefix}.${step.id}.detail`) }}
        </p>
        <ProgressBar
          v-if="step.status === 'running' && !step.indeterminate"
          class="mt-1.5"
          :percent="step.percent"
          active
          compact
        />
      </div>
    </li>
  </ol>
</template>
