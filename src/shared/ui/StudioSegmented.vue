<script setup lang="ts">
import type { Component } from "vue";

const model = defineModel<string>({ required: true });
withDefaults(
  defineProps<{
    options: ReadonlyArray<{ value: string; label: string; icon?: Component }>;
    label?: string;
    size?: "sm" | "md";
  }>(),
  { label: "", size: "md" },
);
</script>

<template>
  <div
    class="inline-flex items-center gap-px rounded-control bg-ink/6 p-0.5"
    role="radiogroup"
    :aria-label="label"
  >
    <button
      v-for="option in options"
      :key="option.value"
      type="button"
      role="radio"
      class="inline-flex items-center gap-1.5 rounded-[5px] font-medium whitespace-nowrap text-muted transition-[background-color,color,box-shadow] duration-150 hover:text-ink aria-checked:bg-raised aria-checked:text-ink aria-checked:shadow-control"
      :class="size === 'sm' ? 'h-6 px-2 text-xs' : 'h-[26px] px-2.5 text-sm'"
      :aria-checked="model === option.value"
      @click="model = option.value"
    >
      <component :is="option.icon" v-if="option.icon" width="14" height="14" />
      {{ option.label }}
    </button>
  </div>
</template>
