<script setup lang="ts">
withDefaults(defineProps<{ level?: 1 | 2; as?: string; padded?: boolean }>(), {
  level: 1,
  as: "section",
  padded: true,
});
</script>

<template>
  <component
    :is="as"
    class="min-w-0 rounded-panel bg-surface"
    :class="[level === 2 ? 'shadow-raised' : 'shadow-panel', padded && 'p-4']"
  >
    <slot />
  </component>
</template>
