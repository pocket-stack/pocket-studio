<script setup lang="ts">
import { computed } from "vue";

export type PillTone =
  "success" | "danger" | "warning" | "info" | "neutral" | "signal";

const props = defineProps<{ tone: PillTone; dot?: boolean }>();

const classes = computed(
  () =>
    ({
      success: "text-success bg-success/12",
      danger: "text-danger bg-danger/12",
      warning: "text-warning bg-warning/14",
      info: "text-info bg-info/12",
      signal: "text-signal bg-signal/12",
      neutral: "text-muted bg-ink/6",
    })[props.tone],
);
</script>

<template>
  <span
    class="inline-flex items-center gap-1.5 rounded-full px-2 py-px text-xs font-medium whitespace-nowrap"
    :class="classes"
  >
    <span v-if="dot" class="size-1.5 rounded-full bg-current" />
    <slot />
  </span>
</template>
