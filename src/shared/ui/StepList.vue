<script setup lang="ts">
import { useI18n } from "vue-i18n";

import type { StepState } from "../composables/useOperations";
import AppIcon from "./AppIcon.vue";
import ProgressBar from "./ProgressBar.vue";

defineProps<{ steps: readonly StepState[]; labelPrefix: string }>();
const { t } = useI18n();
</script>

<template>
  <ol class="flex flex-col gap-1.5">
    <li
      v-for="(step, index) in steps"
      :key="step.id"
      class="flex items-start gap-3 rounded-lg px-3 py-2 transition"
      :class="{
        'bg-signal/8': step.status === 'running',
        'opacity-55': step.status === 'pending' || step.status === 'skipped',
      }"
    >
      <span
        class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full border text-xs font-semibold"
        :class="{
          'border-line text-muted':
            step.status === 'pending' || step.status === 'skipped',
          'border-signal bg-signal text-on-signal': step.status === 'running',
          'border-success bg-success text-white': step.status === 'done',
          'border-danger bg-danger text-white': step.status === 'failed',
          'border-warning text-warning': step.status === 'cancelled',
        }"
      >
        <AppIcon v-if="step.status === 'done'" name="check" :size="14" />
        <AppIcon v-else-if="step.status === 'failed'" name="cross" :size="14" />
        <span
          v-else-if="step.status === 'running'"
          class="spin block h-3 w-3 rounded-full border-2 border-on-signal border-t-transparent"
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
              <AppIcon name="warning" :size="14" />
            </span>
            <span
              v-if="!step.cancellable && step.status !== 'done'"
              :title="t('operation.notCancellable')"
            >
              <AppIcon name="shield" :size="14" />
            </span>
            <span v-if="step.status === 'running'">{{ step.percent }}%</span>
            <span v-else-if="step.status === 'pending'"
              >~{{ step.estimatedSeconds }}s</span
            >
          </span>
        </div>
        <p class="mt-0.5 text-xs text-muted">
          {{ t(`${labelPrefix}.${step.id}.detail`) }}
        </p>
        <ProgressBar
          v-if="step.status === 'running'"
          class="mt-2"
          :percent="step.percent"
          active
          compact
        />
      </div>
    </li>
  </ol>
</template>
