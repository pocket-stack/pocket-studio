<script setup lang="ts">
const model = defineModel<string | number | null>({ required: true });
withDefaults(
  defineProps<{ label?: string; size?: "sm" | "md"; disabled?: boolean }>(),
  { size: "md", label: "" },
);
</script>

<template>
  <span class="relative inline-flex">
    <select
      :value="model"
      :aria-label="label"
      :disabled="disabled"
      class="w-full appearance-none rounded-control bg-raised pr-7 pl-2.5 text-ink shadow-control transition-colors enabled:hover:bg-track disabled:opacity-45"
      :class="size === 'sm' ? 'h-6 text-xs' : 'h-7 text-sm'"
      @change="model = ($event.target as HTMLSelectElement).value"
    >
      <slot />
    </select>
    <IconPhCaretDown
      class="pointer-events-none absolute top-1/2 right-2 -translate-y-1/2 text-muted"
      width="12"
      height="12"
    />
  </span>
</template>
