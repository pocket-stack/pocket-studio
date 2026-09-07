<script setup lang="ts">
import { computed, useAttrs } from "vue";

defineOptions({ inheritAttrs: false });
const model = defineModel<string>({ default: "" });
withDefaults(
  defineProps<{
    type?: "text" | "search" | "password";
    placeholder?: string;
    label?: string;
    disabled?: boolean;
    size?: "sm" | "md";
  }>(),
  { type: "text", size: "md", placeholder: "", label: "" },
);
const attrs = useAttrs();
const rootClass = computed(() => attrs.class);
const inputAttrs = computed(() =>
  Object.fromEntries(Object.entries(attrs).filter(([key]) => key !== "class")),
);
</script>

<template>
  <label
    class="flex items-center gap-1.5 rounded-control bg-track/70 px-2.5 text-ink shadow-inset transition-[background-color,box-shadow] focus-within:bg-raised focus-within:shadow-[0_0_0_2px_var(--color-signal)] has-[input:disabled]:opacity-45"
    :class="[size === 'sm' ? 'h-6 text-xs' : 'h-7 text-sm', rootClass]"
  >
    <span v-if="$slots.icon" class="flex shrink-0 text-muted"
      ><slot name="icon"
    /></span>
    <input
      v-model="model"
      v-bind="inputAttrs"
      :type="type"
      :placeholder="placeholder"
      :aria-label="label"
      :disabled="disabled"
      class="min-w-0 flex-1 bg-transparent outline-none placeholder:text-muted"
    />
  </label>
</template>
