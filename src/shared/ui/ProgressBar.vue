<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  percent: number;
  tone?: "signal" | "success" | "danger" | "warning";
  active?: boolean;
  compact?: boolean;
}>();

const width = computed(() => `${Math.max(0, Math.min(100, props.percent))}%`);
const barClass = computed(() => {
  switch (props.tone) {
    case "success":
      return "bg-success";
    case "danger":
      return "bg-danger";
    case "warning":
      return "bg-warning";
    default:
      return "bg-signal";
  }
});
</script>

<template>
  <div
    class="w-full overflow-hidden rounded-full bg-ink/8"
    :class="compact ? 'h-1.5' : 'h-2.5'"
    role="progressbar"
    :aria-valuenow="Math.round(percent)"
    aria-valuemin="0"
    aria-valuemax="100"
  >
    <div
      class="h-full rounded-full transition-[width] duration-300 ease-out"
      :class="[barClass, active ? 'stripes' : '']"
      :style="{ width }"
    />
  </div>
</template>
