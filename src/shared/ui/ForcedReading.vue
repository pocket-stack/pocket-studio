<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import AppIcon from "./AppIcon.vue";
import ProgressBar from "./ProgressBar.vue";

/**
 * Gate that only reports `ready` once the reader has both spent the minimum
 * time on the content and scrolled it to the end. Time is counted only while
 * the window is focused, so backgrounding the app does not satisfy the gate.
 */
const props = defineProps<{ minimumSeconds: number }>();
const emit = defineEmits<{ ready: [elapsedSeconds: number] }>();

const { t } = useI18n();
const container = ref<HTMLElement | null>(null);
const elapsed = ref(0);
const scrolledToEnd = ref(false);
let timer: number | undefined;
let resizeObserver: ResizeObserver | undefined;

const remaining = computed(() =>
  Math.max(0, props.minimumSeconds - elapsed.value),
);
const timeSatisfied = computed(() => remaining.value === 0);
const ready = computed(() => timeSatisfied.value && scrolledToEnd.value);

function measureScroll(): void {
  const element = container.value;
  if (!element) return;
  const atEnd =
    element.scrollHeight - element.scrollTop - element.clientHeight < 12;
  if (atEnd) scrolledToEnd.value = true;
}

function tick(): void {
  if (
    document.visibilityState === "visible" &&
    document.hasFocus() &&
    !timeSatisfied.value
  )
    elapsed.value += 1;
}

onMounted(() => {
  timer = window.setInterval(tick, 1000);
  requestAnimationFrame(measureScroll);
  resizeObserver = new ResizeObserver(measureScroll);
  if (container.value) resizeObserver.observe(container.value);
});

onBeforeUnmount(() => {
  if (timer) window.clearInterval(timer);
  resizeObserver?.disconnect();
});

watch(ready, (value) => {
  if (value) emit("ready", elapsed.value);
});

defineExpose({ elapsed });
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-3">
    <div
      ref="container"
      class="min-h-0 flex-1 overflow-y-auto leading-relaxed [scrollbar-width:thin] [scrollbar-color:var(--color-line)_transparent] border border-line rounded bg-canvas px-3.5 py-0"
      tabindex="0"
      :aria-label="t('reading.scrollToEnd')"
      @scroll="measureScroll"
    >
      <slot />
    </div>
    <div class="flex items-center gap-3 text-xs text-muted">
      <div class="w-40">
        <ProgressBar
          :percent="(elapsed / minimumSeconds) * 100"
          compact
          :tone="timeSatisfied ? 'success' : 'signal'"
        />
      </div>
      <span v-if="!timeSatisfied" class="flex items-center gap-1">
        <AppIcon name="clock" :size="14" />
        {{ t("reading.remaining", { seconds: remaining }) }}
      </span>
      <span v-else class="flex items-center gap-1 text-success">
        <AppIcon name="check" :size="14" />
        {{ t("reading.timeMet") }}
      </span>
      <span class="text-line">·</span>
      <span v-if="!scrolledToEnd">{{ t("reading.scrollToEnd") }}</span>
      <span v-else class="flex items-center gap-1 text-success">
        <AppIcon name="check" :size="14" />
        {{ t("reading.scrolled") }}
      </span>
    </div>
  </div>
</template>
