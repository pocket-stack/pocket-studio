<script setup lang="ts">
import { watch } from "vue";
import { useI18n } from "vue-i18n";

import ProgressBar from "./ProgressBar.vue";
import { useReadingTimer } from "./useReadingTimer";

/**
 * Forced reading: a scrollable text area (the one place a scrollbar is
 * allowed) gated by a minimum reading time that keeps counting in the
 * background.
 */
const props = defineProps<{ minimumSeconds: number }>();
const emit = defineEmits<{ ready: [elapsedSeconds: number] }>();

const { t } = useI18n();
const { elapsed, remaining, satisfied } = useReadingTimer(
  () => props.minimumSeconds,
);
watch(
  satisfied,
  (value) => {
    if (value) emit("ready", elapsed.value);
  },
  { immediate: true },
);
defineExpose({ elapsed });
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-3">
    <div
      class="min-h-0 flex-1 overflow-y-auto rounded-control bg-ink/4 px-3 py-2 [scrollbar-color:var(--color-line)_transparent] [scrollbar-width:thin]"
      data-scroll-ok
      tabindex="0"
      :aria-label="t('reading.content')"
    >
      <slot />
    </div>
    <footer
      v-if="minimumSeconds > 0"
      class="flex items-center gap-3 text-xs text-muted"
    >
      <div class="w-28">
        <ProgressBar
          :percent="(elapsed / minimumSeconds) * 100"
          compact
          :tone="satisfied ? 'success' : 'signal'"
        />
      </div>
      <span v-if="!satisfied" class="flex items-center gap-1">
        <IconPhClock width="13" height="13" />
        {{ t("reading.remaining", { seconds: remaining }) }}
      </span>
      <span v-else class="flex items-center gap-1 text-success">
        <IconPhCheck width="13" height="13" />{{ t("reading.timeMet") }}
      </span>
    </footer>
  </div>
</template>
