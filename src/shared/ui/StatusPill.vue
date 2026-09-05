<script setup lang="ts">
import { computed } from "vue";

export type PillTone =
  "success" | "danger" | "warning" | "info" | "neutral" | "signal";

const props = defineProps<{ tone: PillTone; dot?: boolean }>();

const classes = computed(() => {
  switch (props.tone) {
    case "success":
      return "text-success bg-success/10 border-success/30";
    case "danger":
      return "text-danger bg-danger/10 border-danger/30";
    case "warning":
      return "text-warning bg-warning/10 border-warning/30";
    case "info":
      return "text-info bg-info/10 border-info/30";
    case "signal":
      return "text-signal bg-signal/10 border-signal/30";
    default:
      return "text-muted bg-ink/5 border-line";
  }
});
</script>

<template>
  <span
    class="inline-flex items-center gap-1.5 rounded-full border px-2.5 py-0.5 text-xs font-medium whitespace-nowrap"
    :class="classes"
  >
    <span v-if="dot" class="h-1.5 w-1.5 rounded-full bg-current" />
    <slot />
  </span>
</template>
