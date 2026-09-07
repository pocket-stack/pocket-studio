<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";

import type { PreparationPlan, RiskSeverity } from "../../../shared/gateway";
import ReadingArea from "../../../shared/ui/ReadingArea.vue";
import StudioButton from "../../../shared/ui/StudioButton.vue";

const props = defineProps<{ plan: PreparationPlan }>();
const emit = defineEmits<{ next: [readingSeconds: number]; back: [] }>();
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
const severityClass: Record<RiskSeverity, string> = {
  high: "before:bg-danger text-danger",
  medium: "before:bg-warning text-warning",
  low: "before:bg-info text-info",
};
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
    <ReadingArea
      :minimum-seconds="plan.minimumReadingSeconds.risks"
      @ready="onReady"
    >
      <ul class="flex flex-col gap-1">
        <li
          v-for="risk in plan.risks"
          :key="risk.id"
          class="relative py-1.5 pl-3 before:absolute before:top-2 before:bottom-2 before:left-0 before:w-[3px] before:rounded-full"
          :class="severityClass[risk.severity]"
        >
          <div class="flex items-baseline gap-2">
            <h4 class="text-sm font-semibold text-ink">
              {{ t(`${riskNamespace}.${risk.id}.title`) }}
            </h4>
            <span class="text-2xs font-semibold">{{
              t(`preparation.risks.severity.${risk.severity}`)
            }}</span>
          </div>
          <p class="text-xs leading-[17px] text-ink">
            {{ t(`${riskNamespace}.${risk.id}.body`) }}
          </p>
          <p class="text-xs leading-[17px] text-muted">
            <b class="font-medium">{{ t("preparation.risks.mitigation") }}</b>
            {{ t(`${riskNamespace}.${risk.id}.mitigation`) }}
          </p>
        </li>
      </ul>
      <p class="py-2 text-center text-2xs text-muted">
        {{ t("preparation.risks.endOfList") }}
      </p>
    </ReadingArea>
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
