<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
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
    class="m-auto max-h-[calc(100dvh-60px)] overflow-auto rounded-[10px] border border-line bg-canvas p-0 text-ink shadow-[0_25px_100px_#00000035] backdrop:bg-[#151c2a50] backdrop:backdrop-blur-[3px]"
    :class="
      compact
        ? 'w-[min(440px,calc(100vw-40px))]'
        : 'w-[min(760px,calc(100vw-60px))]'
    "
    :aria-label="title"
    @cancel.prevent="emit('close')"
  >
    <header
      class="flex items-center justify-between gap-5 border-b border-line bg-chrome px-[22px] py-[15px]"
    >
      <h2 class="text-[13px] font-semibold">{{ title }}</h2>
      <button
        class="inline-flex items-center justify-center rounded p-[5px] text-muted hover:bg-track hover:text-ink"
        :aria-label="t('common.close')"
        @click="emit('close')"
      >
        <IconStudioCross width="18" height="18" />
      </button>
    </header>
    <div
      v-if="open"
      class="max-h-[calc(100dvh-135px)] overflow-y-auto p-6 [scrollbar-width:thin] [scrollbar-color:var(--color-line)_transparent]"
    >
      <slot />
    </div>
  </dialog>
</template>
