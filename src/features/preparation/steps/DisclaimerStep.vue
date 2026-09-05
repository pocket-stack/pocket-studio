<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";

import type { PreparationPlan } from "../../../shared/gateway";
import ForcedReading from "../../../shared/ui/ForcedReading.vue";
import AppIcon from "../../../shared/ui/AppIcon.vue";

defineProps<{ plan: PreparationPlan; startError: string | null }>();
const emit = defineEmits<{ accept: [readingSeconds: number]; back: [] }>();
const { t, tm } = useI18n();

const readingReady = ref(false);
const elapsed = ref(0);
const agreed = ref(false);
const typed = ref("");

const phrase = computed(() => t("preparation.disclaimer.typedPhrase"));
const typedMatches = computed(
  () => typed.value.trim().toUpperCase() === phrase.value.toUpperCase(),
);
const canAccept = computed(
  () => readingReady.value && agreed.value && typedMatches.value,
);
const sections = computed(
  () =>
    tm("preparation.disclaimer.sections") as Array<{
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
        <h3 class="text-base font-semibold">
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
      <AppIcon name="shield" class="text-muted" :size="22" />
    </div>

    <ForcedReading
      :minimum-seconds="plan.minimumReadingSeconds.disclaimer"
      @ready="onReady"
    >
      <div class="space-y-4 text-sm">
        <p class="font-medium">{{ t("preparation.disclaimer.preamble") }}</p>
        <section v-for="(section, index) in sections" :key="index">
          <h4 class="font-semibold">{{ index + 1 }}. {{ section.heading }}</h4>
          <p class="mt-1 text-muted">{{ section.body }}</p>
        </section>
        <p class="pt-2 text-center text-xs text-muted">
          {{ t("preparation.disclaimer.end") }}
        </p>
      </div>
    </ForcedReading>

    <div
      class="card flex flex-col gap-3 p-4"
      :class="readingReady ? '' : 'opacity-50'"
    >
      <label
        class="flex items-center gap-2 text-sm"
        :class="readingReady ? 'cursor-pointer' : 'cursor-not-allowed'"
      >
        <input
          v-model="agreed"
          type="checkbox"
          class="accent-signal"
          :disabled="!readingReady"
        />
        {{ t("preparation.disclaimer.agree") }}
      </label>
      <label class="flex flex-wrap items-center gap-2 text-sm">
        <span>{{ t("preparation.disclaimer.typeToConfirm") }}</span>
        <span class="kbd">{{ phrase }}</span>
        <input
          v-model="typed"
          type="text"
          class="field w-44 font-mono uppercase"
          :disabled="!readingReady"
          :placeholder="phrase"
          autocomplete="off"
          spellcheck="false"
        />
        <AppIcon
          v-if="typedMatches"
          name="check"
          class="text-success"
          :size="16"
        />
      </label>
      <p v-if="startError" class="text-xs text-danger">
        {{
          t(
            `preparation.startErrors.${startError}`,
            t("preparation.startErrors.unknown"),
          )
        }}
      </p>
    </div>

    <footer class="flex items-center justify-between">
      <button class="btn btn-ghost" @click="emit('back')">
        <AppIcon name="arrowLeft" :size="16" />
        {{ t("common.back") }}
      </button>
      <button
        class="btn btn-primary"
        :disabled="!canAccept"
        @click="emit('accept', elapsed)"
      >
        <AppIcon name="bolt" :size="16" />
        {{ t("preparation.disclaimer.acceptAndStart") }}
      </button>
    </footer>
  </div>
</template>
