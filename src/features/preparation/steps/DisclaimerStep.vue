<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";

import type { PreparationPlan } from "../../../shared/gateway";
import ReadingPager from "../../../shared/ui/ReadingPager.vue";
import StudioButton from "../../../shared/ui/StudioButton.vue";
import StudioCallout from "../../../shared/ui/StudioCallout.vue";

const SECTIONS_PER_PAGE = 3;

const props = defineProps<{
  plan: PreparationPlan;
  startError: string | null;
}>();
const emit = defineEmits<{ accept: [readingSeconds: number]; back: [] }>();
const { t, tm } = useI18n();

const sections = computed(
  () =>
    tm(
      props.plan.workflow === "appSync"
        ? "preparation.appSync.disclaimerSections"
        : "preparation.disclaimer.sections",
    ) as Array<{ heading: string; body: string }>,
);
const page = ref(0);
const pages = computed(() =>
  Math.max(1, Math.ceil(sections.value.length / SECTIONS_PER_PAGE)),
);
const pageSections = computed(() =>
  sections.value
    .map((section, index) => ({ ...section, index }))
    .slice(
      page.value * SECTIONS_PER_PAGE,
      (page.value + 1) * SECTIONS_PER_PAGE,
    ),
);
const readingReady = ref(false);
const elapsed = ref(0);

function onReady(seconds: number): void {
  readingReady.value = true;
  elapsed.value = seconds;
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-3">
    <div class="flex items-center justify-between gap-3 text-sm">
      <p class="text-muted">
        {{
          t(
            plan.workflow === "appSync"
              ? "preparation.appSync.preamble"
              : "preparation.disclaimer.preamble",
          )
        }}
      </p>
      <span class="shrink-0 text-xs text-muted">{{
        t("preparation.disclaimer.version", { version: plan.disclaimerVersion })
      }}</span>
    </div>
    <ReadingPager
      v-model:page="page"
      :pages="pages"
      :minimum-seconds="plan.minimumReadingSeconds.disclaimer"
      @ready="onReady"
    >
      <div class="flex flex-col gap-3 text-sm">
        <section v-for="section in pageSections" :key="section.index">
          <h4 class="font-semibold">
            {{ section.index + 1 }}. {{ section.heading }}
          </h4>
          <p class="mt-0.5 leading-[19px] text-muted">{{ section.body }}</p>
        </section>
        <p v-if="page === pages - 1" class="text-center text-xs text-muted">
          {{ t("preparation.disclaimer.end") }}
        </p>
      </div>
    </ReadingPager>
    <StudioCallout v-if="startError" tone="danger">
      {{
        t(
          `preparation.startErrors.${startError}`,
          t("preparation.startErrors.unknown"),
        )
      }}
    </StudioCallout>
    <footer class="flex items-center justify-between">
      <StudioButton variant="ghost" @click="emit('back')">
        <IconPhArrowLeft width="14" height="14" />{{ t("common.back") }}
      </StudioButton>
      <StudioButton
        variant="primary"
        :disabled="!readingReady"
        @click="emit('accept', elapsed)"
      >
        <IconPhLightning width="14" height="14" />
        {{ t("preparation.disclaimer.acceptAndStart") }}
      </StudioButton>
    </footer>
  </div>
</template>
