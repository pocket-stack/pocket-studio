<script setup lang="ts">
import IconPhInfo from "~icons/ph/info";
import IconPhCheckCircle from "~icons/ph/check-circle";
import IconPhWarning from "~icons/ph/warning";
import IconPhWarningOctagon from "~icons/ph/warning-octagon";
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import { useNotifications } from "../shared/composables/useNotifications";

const { t } = useI18n();
const { items, dismiss } = useNotifications();
// Everything but errors is shown transiently in the LCD instead.
const errors = computed(() =>
  items.value.filter((item) => item.tone === "error"),
);

const tones = {
  info: "text-info",
  success: "text-success",
  warning: "text-warning",
  error: "text-danger",
};
const icons = {
  info: IconPhInfo,
  success: IconPhCheckCircle,
  warning: IconPhWarning,
  error: IconPhWarningOctagon,
};
</script>

<template>
  <div
    class="pointer-events-none fixed right-3 bottom-[42px] z-50 flex w-80 flex-col gap-2"
  >
    <TransitionGroup
      enter-active-class="transition-[opacity,transform] duration-200 ease-out motion-reduce:transition-none"
      leave-active-class="transition-[opacity,transform] duration-150 ease-in motion-reduce:transition-none"
      enter-from-class="translate-y-2 opacity-0"
      leave-to-class="translate-y-2 opacity-0"
    >
      <div
        v-for="item in errors"
        :key="item.id"
        class="pointer-events-auto flex items-start gap-2.5 rounded-panel bg-raised px-3 py-2.5 text-sm shadow-overlay"
        role="status"
      >
        <component
          :is="icons[item.tone]"
          width="16"
          height="16"
          class="mt-px shrink-0"
          :class="tones[item.tone]"
        />
        <p class="min-w-0 flex-1 text-ink">
          {{ t(item.key, item.params ?? {}) }}
        </p>
        <button
          class="-mr-1 rounded p-0.5 text-muted hover:text-ink"
          :aria-label="t('common.close')"
          @click="dismiss(item.id)"
        >
          <IconPhX width="13" height="13" />
        </button>
      </div>
    </TransitionGroup>
  </div>
</template>
