<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import ProgressBar from "./ProgressBar.vue";

/** A minimum elapsed-time gate; background time counts toward confirmation. */
const props = defineProps<{ minimumSeconds: number }>();
const emit = defineEmits<{ ready: [elapsedSeconds: number] }>();

const { t } = useI18n();
const elapsed = ref(0);
let timer: number | undefined;
let startedAt = 0;

const remaining = computed(() =>
  Math.max(0, props.minimumSeconds - elapsed.value),
);
const timeSatisfied = computed(() => remaining.value === 0);
function tick(): void {
  elapsed.value = Math.min(
    props.minimumSeconds,
    Math.floor((performance.now() - startedAt) / 1000),
  );
  if (timeSatisfied.value && timer !== undefined) {
    window.clearInterval(timer);
    timer = undefined;
  }
}

onMounted(() => {
  if (props.minimumSeconds <= 0) {
    emit("ready", 0);
    return;
  }
  startedAt = performance.now();
  timer = window.setInterval(tick, 100);
  // Recompute from elapsed time if a background webview throttled callbacks.
  window.addEventListener("focus", tick);
  document.addEventListener("visibilitychange", tick);
});

onBeforeUnmount(() => {
  if (timer !== undefined) window.clearInterval(timer);
  window.removeEventListener("focus", tick);
  document.removeEventListener("visibilitychange", tick);
});

watch(timeSatisfied, (value) => {
  if (value) emit("ready", elapsed.value);
});

defineExpose({ elapsed });
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-3">
    <div
      class="min-h-0 flex-1 overflow-y-auto leading-relaxed [scrollbar-width:thin] [scrollbar-color:var(--color-line)_transparent] border border-line rounded bg-canvas px-3.5 py-0"
      tabindex="0"
      :aria-label="t('reading.content')"
    >
      <slot />
    </div>
    <div
      v-if="minimumSeconds > 0"
      class="flex items-center gap-3 text-xs text-muted"
    >
      <div class="w-40">
        <ProgressBar
          :percent="(elapsed / minimumSeconds) * 100"
          compact
          :tone="timeSatisfied ? 'success' : 'signal'"
        />
      </div>
      <span v-if="!timeSatisfied" class="flex items-center gap-1">
        <IconStudioClock width="14" height="14" />
        {{ t("reading.remaining", { seconds: remaining }) }}
      </span>
      <span v-else class="flex items-center gap-1 text-success">
        <IconStudioCheck width="14" height="14" />
        {{ t("reading.timeMet") }}
      </span>
    </div>
  </div>
</template>
