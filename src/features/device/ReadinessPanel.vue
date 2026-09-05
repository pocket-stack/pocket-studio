<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import type { CheckStatus, ReadinessReport } from "../../shared/gateway";
import AppIcon from "../../shared/ui/AppIcon.vue";
import StatusPill from "../../shared/ui/StatusPill.vue";

const props = defineProps<{
  report: ReadinessReport | null;
  checking: boolean;
}>();
const emit = defineEmits<{ recheck: []; prepare: [] }>();
const { t, d } = useI18n();

const tone = computed(() => {
  switch (props.report?.status) {
    case "ready":
      return "success";
    case "needsPreparation":
      return "warning";
    case "unsupported":
      return "danger";
    default:
      return "neutral";
  }
});

function checkIcon(status: CheckStatus): { name: string; cls: string } {
  switch (status) {
    case "pass":
      return { name: "check", cls: "text-success bg-success/12" };
    case "fail":
      return { name: "cross", cls: "text-danger bg-danger/12" };
    case "warn":
      return { name: "warning", cls: "text-warning bg-warning/12" };
    default:
      return { name: "info", cls: "text-muted bg-ink/8" };
  }
}
</script>

<template>
  <section class="card p-6">
    <div class="flex items-start justify-between gap-4">
      <div>
        <h3 class="text-base font-semibold">{{ t("readiness.title") }}</h3>
        <p class="mt-1 text-sm text-muted">{{ t("readiness.subtitle") }}</p>
      </div>
      <button
        class="btn btn-ghost"
        :disabled="checking"
        @click="emit('recheck')"
      >
        <AppIcon name="refresh" :size="16" :class="checking ? 'spin' : ''" />
        {{ t("readiness.recheck") }}
      </button>
    </div>

    <div
      v-if="checking && !report"
      class="mt-5 flex items-center gap-3 text-sm text-muted"
    >
      <span
        class="spin block h-4 w-4 rounded-full border-2 border-signal border-t-transparent"
      />
      {{ t("readiness.checking") }}
    </div>

    <template v-else-if="report">
      <div
        class="mt-5 flex flex-wrap items-center gap-3 rounded-xl border border-line bg-raised p-4"
      >
        <StatusPill :tone="tone" dot>{{
          t(`readiness.status.${report.status}`)
        }}</StatusPill>
        <p class="flex-1 text-sm">
          {{ t(`readiness.summary.${report.status}`) }}
        </p>
        <span class="text-xs text-muted">{{
          t("readiness.checkedAt", { time: d(report.checkedAt, "time") })
        }}</span>
      </div>

      <ul class="mt-4 grid gap-2 sm:grid-cols-2">
        <li
          v-for="check in report.checks"
          :key="check.id"
          class="flex items-start gap-3 rounded-lg px-2 py-1.5"
        >
          <span
            class="mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-full"
            :class="checkIcon(check.status).cls"
          >
            <AppIcon :name="checkIcon(check.status).name" :size="14" />
          </span>
          <div class="min-w-0">
            <div class="flex items-center gap-2 text-sm">
              <span class="font-medium">{{
                t(`readiness.checks.${check.id}.title`)
              }}</span>
              <span v-if="check.value" class="font-mono text-xs text-muted">{{
                check.value
              }}</span>
            </div>
            <p class="text-xs text-muted">
              {{ t(`readiness.checks.${check.id}.${check.status}`) }}
            </p>
          </div>
        </li>
      </ul>

      <div
        v-if="report.status === 'needsPreparation'"
        class="mt-5 rounded-xl border border-warning/40 bg-warning/8 p-4"
      >
        <div class="flex items-start gap-3">
          <AppIcon name="warning" class="mt-0.5 text-warning" />
          <div class="flex-1">
            <p class="text-sm font-medium">
              {{ t("readiness.guidance.jailbreak.title") }}
            </p>
            <p class="mt-1 text-sm text-muted">
              {{ t("readiness.guidance.jailbreak.body") }}
            </p>
            <ul class="mt-2 list-disc pl-5 text-xs text-muted">
              <li>{{ t("readiness.guidance.jailbreak.point1") }}</li>
              <li>{{ t("readiness.guidance.jailbreak.point2") }}</li>
              <li>{{ t("readiness.guidance.jailbreak.point3") }}</li>
            </ul>
          </div>
        </div>
        <div class="mt-4 flex justify-end">
          <button class="btn btn-primary" @click="emit('prepare')">
            <AppIcon name="bolt" :size="16" />
            {{ t("readiness.guidance.jailbreak.action") }}
          </button>
        </div>
      </div>

      <div
        v-else-if="report.status === 'ready'"
        class="mt-5 rounded-xl border border-success/40 bg-success/8 p-4 text-sm"
      >
        <div class="flex items-start gap-3">
          <AppIcon name="check" class="mt-0.5 text-success" />
          <p>{{ t("readiness.guidance.ready") }}</p>
        </div>
      </div>
    </template>
  </section>
</template>
