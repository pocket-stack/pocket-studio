<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { focusInitial } from "./dialogFocus";
const props = defineProps<{
  open: boolean;
  title: string;
  compact?: boolean;
}>();
const emit = defineEmits<{ close: [] }>();
const { t } = useI18n();
const dialog = ref<HTMLDialogElement | null>(null);
let returnFocus: HTMLElement | null = null;
watch(
  () => props.open,
  async (open) => {
    if (open) {
      returnFocus = document.activeElement as HTMLElement;
      await nextTick();
      dialog.value?.showModal();
      focusInitial(dialog.value);
    } else {
      dialog.value?.close();
      await nextTick();
      returnFocus?.focus();
    }
  },
);
</script>
<template>
  <dialog
    ref="dialog"
    class="m-auto max-h-[calc(100dvh-60px)] flex-col overflow-hidden outline-none rounded-panel bg-surface p-0 text-ink shadow-overlay backdrop:bg-[#0b1220]/35 backdrop:backdrop-blur-[2px] open:flex"
    :class="
      compact
        ? 'w-[min(440px,calc(100vw-40px))]'
        : 'w-[min(720px,calc(100vw-60px))]'
    "
    :aria-label="title"
    tabindex="-1"
    @cancel.prevent="emit('close')"
  >
    <header class="flex shrink-0 items-center justify-between gap-5 px-5 pt-4">
      <h2 class="text-base font-semibold">{{ title }}</h2>
      <button
        class="-mr-1.5 inline-flex items-center justify-center rounded-control p-1 text-muted hover:bg-ink/6 hover:text-ink"
        :aria-label="t('common.close')"
        @click="emit('close')"
      >
        <IconPhX width="16" height="16" />
      </button>
    </header>
    <div v-if="open" class="flex min-h-0 flex-auto flex-col p-5">
      <slot />
    </div>
  </dialog>
</template>
