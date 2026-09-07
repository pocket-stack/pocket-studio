<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";

import type { PreparationPlan } from "../../../shared/gateway";
import ForcedReading from "../../../shared/ui/ForcedReading.vue";

const props = defineProps<{
  plan: PreparationPlan;
  startError: string | null;
}>();
const emit = defineEmits<{ accept: [readingSeconds: number]; back: [] }>();
const { t, tm } = useI18n();

const readingReady = ref(false);
const elapsed = ref(0);
const sections = computed(
  () =>
    tm(
      props.plan.workflow === "appSync"
        ? "preparation.appSync.disclaimerSections"
        : "preparation.disclaimer.sections",
    ) as Array<{
      heading: string;
      body: string;
    }>,
);

function onReady(seconds: number): void {
  readingReady.value = true;
  elapsed.value = seconds;
}
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-4">
    <div class="flex items-center justify-between">
      <div>
        <h3 class="text-base font-semibold hidden">
          {{ t("preparation.disclaimer.title") }}
        </h3>
        <p class="text-xs text-muted">
          {{
            t("preparation.disclaimer.version", {
              version: plan.disclaimerVersion,
            })
          }}
        </p>
      </div>
      <IconStudioShield class="text-muted" width="22" height="22" />
    </div>

    <ForcedReading
      :minimum-seconds="plan.minimumReadingSeconds.disclaimer"
      @ready="onReady"
    >
      <div class="space-y-4 text-sm">
        <p class="font-medium">
          {{
            t(
              plan.workflow === "appSync"
                ? "preparation.appSync.preamble"
                : "preparation.disclaimer.preamble",
            )
          }}
        </p>
        <section v-for="(section, index) in sections" :key="index">
          <h4 class="font-semibold">{{ index + 1 }}. {{ section.heading }}</h4>
          <p class="mt-1 text-muted">{{ section.body }}</p>
        </section>
        <p class="pt-2 text-center text-xs text-muted">
          {{ t("preparation.disclaimer.end") }}
        </p>
      </div>
    </ForcedReading>

    <p v-if="startError" class="text-xs text-danger">
      {{
        t(
          `preparation.startErrors.${startError}`,
          t("preparation.startErrors.unknown"),
        )
      }}
    </p>

    <footer class="flex items-center justify-between">
      <button
        class="inline-flex items-center justify-center gap-2 rounded-md font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-transparent text-muted enabled:hover:bg-ink/6 enabled:hover:text-ink px-3 py-1.5 text-[12px]"
        @click="emit('back')"
      >
        <IconStudioArrowLeft width="16" height="16" />
        {{ t("common.back") }}
      </button>
      <button
        class="inline-flex items-center justify-center gap-2 rounded-md font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06] px-3 py-1.5 text-[12px]"
        :disabled="!readingReady"
        @click="emit('accept', elapsed)"
      >
        <IconStudioBolt width="16" height="16" />
        {{ t("preparation.disclaimer.acceptAndStart") }}
      </button>
    </footer>
  </div>
</template>
