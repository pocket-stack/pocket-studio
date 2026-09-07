<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger" | "link";
    size?: "sm" | "md";
    type?: "button" | "submit";
    disabled?: boolean;
    loading?: boolean;
  }>(),
  { variant: "secondary", size: "md", type: "button" },
);

const classes = computed(() => [
  "inline-flex select-none items-center justify-center gap-1.5 whitespace-nowrap rounded-control font-medium transition-[background-color,color,box-shadow,transform] duration-150 disabled:cursor-not-allowed disabled:opacity-45 enabled:active:translate-y-px",
  props.variant === "link"
    ? "h-auto px-0 text-sm font-normal"
    : props.size === "sm"
      ? "h-6 px-2.5 text-xs"
      : "h-7 px-3 text-sm",
  {
    primary:
      "bg-signal text-on-signal shadow-control enabled:hover:bg-signal-strong",
    secondary: "bg-raised text-ink shadow-control enabled:hover:bg-track",
    ghost:
      "bg-transparent text-muted enabled:hover:bg-ink/6 enabled:hover:text-ink",
    danger: "bg-danger/10 text-danger enabled:hover:bg-danger/16",
    link: "text-signal underline-offset-2 enabled:hover:underline",
  }[props.variant],
]);
</script>

<template>
  <button :type="type" :disabled="disabled || loading" :class="classes">
    <IconSvgSpinners90Ring v-if="loading" width="14" height="14" />
    <slot />
  </button>
</template>
