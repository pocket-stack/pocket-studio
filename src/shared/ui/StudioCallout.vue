<script setup lang="ts">
import IconPhInfo from "~icons/ph/info";
import IconPhWarning from "~icons/ph/warning";
import IconPhWarningOctagon from "~icons/ph/warning-octagon";
import IconPhCheckCircle from "~icons/ph/check-circle";
import { computed } from "vue";

/** Compact single-tone notice. Only `strong` notices shout. */
const props = withDefaults(
  defineProps<{
    tone?: "info" | "warning" | "danger" | "success" | "neutral";
    title?: string;
    strong?: boolean;
  }>(),
  { tone: "info", title: "" },
);

const icon = computed(
  () =>
    ({
      info: IconPhInfo,
      neutral: IconPhInfo,
      warning: IconPhWarning,
      danger: IconPhWarningOctagon,
      success: IconPhCheckCircle,
    })[props.tone],
);
const toneClass = computed(
  () =>
    ({
      info: "bg-info/8 [--callout-ink:var(--color-info)]",
      neutral: "bg-ink/5 [--callout-ink:var(--color-muted)]",
      warning: "bg-warning/10 [--callout-ink:var(--color-warning)]",
      danger: "bg-danger/8 [--callout-ink:var(--color-danger)]",
      success: "bg-success/8 [--callout-ink:var(--color-success)]",
    })[props.tone],
);
</script>

<template>
  <div
    :role="tone === 'danger' ? 'alert' : 'status'"
    class="flex items-start gap-2 rounded-control px-2.5 py-1.5 text-sm leading-[18px] text-ink"
    :class="[toneClass, strong && 'py-2.5 font-medium']"
  >
    <component
      :is="icon"
      width="15"
      height="15"
      class="mt-px shrink-0 text-(--callout-ink)"
    />
    <p class="min-w-0 flex-1">
      <b v-if="title" class="mr-1.5 font-semibold">{{ title }}</b
      ><slot />
    </p>
    <span v-if="$slots.action" class="ml-1 shrink-0 self-center"
      ><slot name="action"
    /></span>
  </div>
</template>
