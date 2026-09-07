<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";

import type { PreparationPlan, RiskSeverity } from "../../../shared/gateway";
import ForcedReading from "../../../shared/ui/ForcedReading.vue";
import StatusPill from "../../../shared/ui/StatusPill.vue";

const props = defineProps<{
  plan: PreparationPlan;
}>();
const emit = defineEmits<{
  next: [readingSeconds: number];
  back: [];
}>();
const { t } = useI18n();

const riskNamespace = computed(() =>
  props.plan.workflow === "appSync"
    ? "preparation.appSync.risks"
    : "preparation.risks.items",
);
const readingReady = ref(false);
const elapsed = ref(0);

function onReady(seconds: number): void {
  readingReady.value = true;
  elapsed.value = seconds;
}

function severityTone(severity: RiskSeverity): "danger" | "warning" | "info" {
  if (severity === "high") return "danger";
  if (severity === "medium") return "warning";
  return "info";
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-4">
    <div class="text-[12px]">
      <div class="flex items-start gap-3">
        <IconStudioWarning class="mt-0.5 text-danger hidden" />
        <div>
          <p class="font-semibold hidden m-0 text-[12px] leading-[1.428571]">
            {{ t("preparation.risks.banner.title") }}
          </p>
          <p class="text-muted m-0 text-[12px] leading-[1.428571]">
            {{
              t(
                plan.workflow === "appSync"
                  ? "preparation.appSync.intro"
                  : "preparation.risks.banner.body",
              )
            }}
          </p>
        </div>
      </div>
    </div>

    <ForcedReading
      :minimum-seconds="plan.minimumReadingSeconds.risks"
      @ready="onReady"
    >
      <div class="space-y-3">
        <article
          v-for="risk in plan.risks"
          :key="risk.id"
          class="border-0 border-b border-line bg-transparent px-0 py-3"
        >
          <div class="flex items-center gap-2">
            <StatusPill :tone="severityTone(risk.severity)">{{
              t(`preparation.risks.severity.${risk.severity}`)
            }}</StatusPill>
            <h4 class="text-sm font-semibold">
              {{ t(`${riskNamespace}.${risk.id}.title`) }}
            </h4>
          </div>
          <p class="mt-2 text-[12px] leading-[1.8]">
            {{ t(`${riskNamespace}.${risk.id}.body`) }}
          </p>
          <p class="mt-2 text-muted text-[12px] leading-[1.8]">
            <span class="font-medium">{{
              t("preparation.risks.mitigation")
            }}</span>
            {{ t(`${riskNamespace}.${risk.id}.mitigation`) }}
          </p>
        </article>
        <p class="pt-2 text-center text-xs text-muted">
          {{ t("preparation.risks.endOfList") }}
        </p>
      </div>
    </ForcedReading>

    <footer class="flex items-center justify-between">
      <button
        class="inline-flex items-center justify-center gap-2 rounded-md font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-transparent text-muted enabled:hover:bg-ink/6 enabled:hover:text-ink px-3 py-1.5 text-[12px]"
        @click="emit('back')"
      >
        <IconStudioArrowLeft width="16" height="16" />
        {{ t("common.back") }}
      </button>
      <div class="flex items-center gap-3">
        <button
          class="inline-flex items-center justify-center gap-2 rounded-md font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06] px-3 py-1.5 text-[12px]"
          :disabled="!readingReady"
          @click="emit('next', elapsed)"
        >
          {{ t("preparation.risks.continue") }}
          <IconStudioArrowRight width="16" height="16" />
        </button>
      </div>
    </footer>
  </div>
</template>
