<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";

import type { PreparationPlan, RiskSeverity } from "../../../shared/gateway";
import ReadingPager from "../../../shared/ui/ReadingPager.vue";
import StatusPill from "../../../shared/ui/StatusPill.vue";
import StudioButton from "../../../shared/ui/StudioButton.vue";

const RISKS_PER_PAGE = 2;

const props = defineProps<{ plan: PreparationPlan }>();
const emit = defineEmits<{ next: [readingSeconds: number]; back: [] }>();
const { t } = useI18n();

const riskNamespace = computed(() =>
  props.plan.workflow === "appSync"
    ? "preparation.appSync.risks"
    : "preparation.risks.items",
);
const page = ref(0);
const pages = computed(() =>
  Math.max(1, Math.ceil(props.plan.risks.length / RISKS_PER_PAGE)),
);
const pageRisks = computed(() =>
  props.plan.risks.slice(
    page.value * RISKS_PER_PAGE,
    (page.value + 1) * RISKS_PER_PAGE,
  ),
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
  <div class="flex min-h-0 flex-1 flex-col gap-3">
    <p class="text-sm text-muted">
      {{
        t(
          plan.workflow === "appSync"
            ? "preparation.appSync.intro"
            : "preparation.risks.banner.body",
        )
      }}
    </p>
    <ReadingPager
      v-model:page="page"
      :pages="pages"
      :minimum-seconds="plan.minimumReadingSeconds.risks"
      @ready="onReady"
    >
      <div class="flex flex-col gap-2.5">
        <article
          v-for="risk in pageRisks"
          :key="risk.id"
          class="rounded-panel bg-ink/4 px-4 py-3"
        >
          <div class="flex items-center gap-2">
            <StatusPill :tone="severityTone(risk.severity)">{{
              t(`preparation.risks.severity.${risk.severity}`)
            }}</StatusPill>
            <h4 class="text-base font-semibold">
              {{ t(`${riskNamespace}.${risk.id}.title`) }}
            </h4>
          </div>
          <p class="mt-1.5 text-sm leading-[19px]">
            {{ t(`${riskNamespace}.${risk.id}.body`) }}
          </p>
          <p class="mt-1.5 text-sm leading-[19px] text-muted">
            <span class="font-medium text-ink">{{
              t("preparation.risks.mitigation")
            }}</span>
            {{ t(`${riskNamespace}.${risk.id}.mitigation`) }}
          </p>
        </article>
        <p v-if="page === pages - 1" class="text-center text-xs text-muted">
          {{ t("preparation.risks.endOfList") }}
        </p>
      </div>
    </ReadingPager>
    <footer class="flex items-center justify-between">
      <StudioButton variant="ghost" @click="emit('back')">
        <IconPhArrowLeft width="14" height="14" />{{ t("common.back") }}
      </StudioButton>
      <StudioButton
        variant="primary"
        :disabled="!readingReady"
        @click="emit('next', elapsed)"
      >
        {{ t("preparation.risks.continue") }}
        <IconPhArrowRight width="14" height="14" />
      </StudioButton>
    </footer>
  </div>
</template>
