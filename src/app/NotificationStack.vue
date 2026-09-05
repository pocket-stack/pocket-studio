<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { useNotifications } from "../shared/composables/useNotifications";
import AppIcon from "../shared/ui/AppIcon.vue";

const { t } = useI18n();
const { items, dismiss } = useNotifications();

const tones = {
  info: "border-info/40 text-info",
  success: "border-success/40 text-success",
  warning: "border-warning/40 text-warning",
  error: "border-danger/40 text-danger",
};
const icons = {
  info: "info",
  success: "check",
  warning: "warning",
  error: "cross",
};
</script>

<template>
  <div
    class="pointer-events-none fixed right-4 bottom-4 z-50 flex w-80 flex-col gap-2"
  >
    <TransitionGroup name="toast">
      <div
        v-for="item in items"
        :key="item.id"
        class="pointer-events-auto flex items-start gap-3 rounded-xl border bg-raised p-3 text-sm shadow-lg"
        :class="tones[item.tone]"
        role="status"
      >
        <AppIcon :name="icons[item.tone]" :size="16" class="mt-0.5" />
        <p class="flex-1 text-ink">{{ t(item.key, item.params ?? {}) }}</p>
        <button class="text-muted hover:text-ink" @click="dismiss(item.id)">
          <AppIcon name="cross" :size="14" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>

<style scoped>
.toast-enter-active,
.toast-leave-active {
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
}
.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateY(8px);
}
</style>
