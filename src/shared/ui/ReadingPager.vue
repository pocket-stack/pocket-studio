<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import ProgressBar from "./ProgressBar.vue";
import StudioButton from "./StudioButton.vue";
import { useReadingTimer } from "./useReadingTimer";

/**
 * Forced reading without scrolling: content is split into pages, every page
 * must be turned to, and the minimum reading time must elapse.
 */
const props = defineProps<{ pages: number; minimumSeconds: number }>();
const page = defineModel<number>("page", { default: 0 });
const emit = defineEmits<{ ready: [elapsedSeconds: number] }>();

const { t } = useI18n();
const { elapsed, remaining, satisfied } = useReadingTimer(
  () => props.minimumSeconds,
);
const visited = ref(new Set<number>([page.value]));
watch(page, (current) => {
  visited.value = new Set([...visited.value, current]);
});
const allViewed = computed(() => visited.value.size >= props.pages);
const ready = computed(() => satisfied.value && allViewed.value);
watch(
  ready,
  (value) => {
    if (value) emit("ready", elapsed.value);
  },
  { immediate: true },
);
function go(next: number): void {
  page.value = Math.min(props.pages - 1, Math.max(0, next));
}
defineExpose({ elapsed });
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-3">
    <div class="min-h-0 flex-1" :aria-label="t('reading.content')">
      <slot :page="page" />
    </div>
    <footer class="flex items-center gap-3 text-xs text-muted">
      <div v-if="minimumSeconds > 0" class="w-28">
        <ProgressBar
          :percent="(elapsed / minimumSeconds) * 100"
          compact
          :tone="satisfied ? 'success' : 'signal'"
        />
      </div>
      <span v-if="!satisfied" class="flex items-center gap-1">
        <IconPhClock width="13" height="13" />
        {{ t("reading.remaining", { seconds: remaining }) }}
      </span>
      <span v-else-if="!allViewed" class="flex items-center gap-1">
        <IconPhBookOpen width="13" height="13" />{{ t("reading.viewAll") }}
      </span>
      <span v-else class="flex items-center gap-1 text-success">
        <IconPhCheck width="13" height="13" />{{ t("reading.allViewed") }}
      </span>
      <div v-if="pages > 1" class="ml-auto flex items-center gap-1">
        <span class="mr-1 tabular-nums">{{
          t("reading.page", { page: page + 1, total: pages })
        }}</span>
        <button
          v-for="index in pages"
          :key="index"
          class="size-4 rounded-full p-1 after:block after:size-1.5 after:rounded-full after:bg-ink/15 hover:after:bg-ink/40 aria-[current=page]:after:bg-signal data-[visited=true]:after:bg-ink/40"
          :aria-current="page === index - 1 ? 'page' : undefined"
          :data-visited="visited.has(index - 1)"
          :aria-label="t('reading.page', { page: index, total: pages })"
          @click="go(index - 1)"
        />
        <StudioButton
          size="sm"
          variant="ghost"
          :disabled="page <= 0"
          :aria-label="t('reading.previous')"
          @click="go(page - 1)"
        >
          <IconPhCaretLeft width="13" height="13" />
        </StudioButton>
        <StudioButton
          size="sm"
          variant="secondary"
          :disabled="page >= pages - 1"
          @click="go(page + 1)"
        >
          {{ t("reading.next") }}<IconPhCaretRight width="13" height="13" />
        </StudioButton>
      </div>
    </footer>
  </div>
</template>
