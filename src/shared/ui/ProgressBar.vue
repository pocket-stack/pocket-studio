<script setup lang="ts">
import { useSpring, useTransform } from "motion-v";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";

import { prefersReducedMotion } from "./motion";

/**
 * Progress that never stalls: the reported percent is chased by a spring on a
 * compositor-only transform, and while a step is `active` the bar creeps
 * toward `trickleTo` (the percent the running step would end at) so long
 * steps keep moving between sparse progress events.
 */
const props = withDefaults(
  defineProps<{
    percent: number;
    tone?: "signal" | "success" | "danger" | "warning";
    active?: boolean;
    indeterminate?: boolean;
    compact?: boolean;
    trickleTo?: number;
  }>(),
  { tone: "signal", trickleTo: 0 },
);

const reduceMotion = prefersReducedMotion();
const clamp = (value: number) => Math.max(0, Math.min(100, value));
const creep = ref(clamp(props.percent));
const shown = computed(() => Math.max(clamp(props.percent), creep.value));
const spring = useSpring(shown.value, {
  stiffness: 110,
  damping: 26,
  mass: 0.9,
});
const x = useTransform(spring, (value) => `translateX(${value - 100}%)`);
const bar = ref<HTMLElement | null>(null);
let trickleTimer: number | undefined;
let unsubscribe: (() => void) | undefined;

// Write the transform straight to the element: no re-render per frame.
function paint(transform: string): void {
  const element = bar.value;
  if (element?.style) element.style.transform = transform;
}

function stopTrickle(): void {
  if (trickleTimer !== undefined) window.clearInterval(trickleTimer);
  trickleTimer = undefined;
}
function syncTrickle(): void {
  stopTrickle();
  const ceiling = Math.min(99, clamp(props.trickleTo));
  if (!props.active || props.indeterminate || ceiling <= shown.value) return;
  trickleTimer = window.setInterval(() => {
    const gap = ceiling - creep.value;
    if (gap <= 0.2) return stopTrickle();
    // Asymptotic approach: never claims the step finished.
    creep.value = Math.min(ceiling, creep.value + gap * 0.035);
  }, 250);
}

watch(
  () => clamp(props.percent),
  (percent, previous) => {
    if (percent < previous) creep.value = percent;
    else creep.value = Math.max(creep.value, percent);
    syncTrickle();
  },
);
watch(() => [props.active, props.indeterminate, props.trickleTo], syncTrickle);
watch(shown, (value) => {
  if (reduceMotion) spring.jump(value);
  else spring.set(value);
});
onMounted(() => {
  spring.jump(shown.value);
  paint(x.get());
  unsubscribe = x.on("change", paint);
  syncTrickle();
});
onBeforeUnmount(() => {
  stopTrickle();
  unsubscribe?.();
});

const barClass = computed(
  () =>
    ({
      success: "bg-success",
      danger: "bg-danger",
      warning: "bg-warning",
      signal: "bg-signal",
    })[props.tone],
);
</script>

<template>
  <div
    class="relative w-full overflow-hidden rounded-full bg-ink/10"
    :class="compact ? 'h-1' : 'h-2'"
    role="progressbar"
    :aria-valuenow="indeterminate ? undefined : Math.round(percent)"
    aria-valuemin="0"
    aria-valuemax="100"
    :aria-busy="indeterminate || undefined"
  >
    <div
      v-if="indeterminate"
      class="absolute inset-y-0 w-1/3 rounded-full motion-safe:animate-indeterminate"
      :class="barClass"
    />
    <div
      v-else
      ref="bar"
      class="absolute inset-0 overflow-hidden rounded-full will-change-transform"
      :class="barClass"
    >
      <span
        v-if="active"
        aria-hidden="true"
        class="absolute inset-y-0 -left-7 w-[calc(100%+28px)] bg-[repeating-linear-gradient(-45deg,transparent_0_7px,rgb(255_255_255/0.28)_7px_14px)] motion-safe:animate-stripes"
      />
    </div>
  </div>
</template>
