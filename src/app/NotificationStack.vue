<script setup lang="ts">
import IconPhInfo from "~icons/ph/info";
import IconPhCheck from "~icons/ph/check";
import IconPhWarning from "~icons/ph/warning";
import IconPhX from "~icons/ph/x";
import { useI18n } from "vue-i18n";

import { useNotifications } from "../shared/composables/useNotifications";

const { t } = useI18n();
const { items, dismiss } = useNotifications();

const tones = {
  info: "border-info/40 text-info",
  success: "border-success/40 text-success",
  warning: "border-warning/40 text-warning",
  error: "border-danger/40 text-danger",
};
const icons = {
  info: IconPhInfo,
  success: IconPhCheck,
  warning: IconPhWarning,
  error: IconPhX,
};
</script>

<template>
  <div
    class="pointer-events-none fixed right-4 bottom-4 z-50 flex w-80 flex-col gap-2"
  >
    <TransitionGroup
      enter-active-class="transition-[opacity,transform] duration-200 ease-[ease] motion-reduce:transition-none"
      leave-active-class="transition-[opacity,transform] duration-200 ease-[ease] motion-reduce:transition-none"
      enter-from-class="translate-y-2 opacity-0"
      leave-to-class="translate-y-2 opacity-0"
    >
      <div
        v-for="item in items"
        :key="item.id"
        class="pointer-events-auto flex items-start gap-3 rounded-xl border bg-raised p-3 text-sm shadow-lg"
        :class="tones[item.tone]"
        role="status"
      >
        <component
          :is="icons[item.tone]"
          width="16"
          height="16"
          class="mt-0.5"
        />
        <p class="flex-1 text-ink">{{ t(item.key, item.params ?? {}) }}</p>
        <button class="text-muted hover:text-ink" @click="dismiss(item.id)">
          <IconPhX width="14" height="14" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>
