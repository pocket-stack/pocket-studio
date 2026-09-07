<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useGateway, type StoreApplication } from "../../shared/gateway";

const props = defineProps<{
  blob: StoreApplication["media"][number]["blob"];
  alt?: string;
}>();
const { t } = useI18n();
const url = ref<string | null>(null);
const failed = ref(false);
let request = 0;
watch(
  () => props.blob,
  async (blob) => {
    const generation = ++request;
    url.value = null;
    failed.value = false;
    try {
      const resolved = await useGateway().store.media(blob.sha256);
      if (generation === request) url.value = resolved;
    } catch {
      if (generation === request) failed.value = true;
    }
  },
  { immediate: true },
);
</script>

<template>
  <div class="relative overflow-hidden bg-track">
    <img
      v-if="url && !failed"
      :src="url"
      :alt="alt ?? ''"
      class="h-full w-full object-contain"
      loading="lazy"
      @error="failed = true"
    />
    <div
      v-else
      class="flex h-full min-h-12 w-full items-center justify-center text-muted"
      :aria-label="
        failed ? t('store.mediaUnavailable') : t('store.mediaLoading')
      "
    >
      <IconStudioGrid v-if="failed" width="24" height="24" aria-hidden="true" />
      <IconStudioSpinner
        v-else
        class="motion-safe:animate-studio-spin"
        width="18"
        height="18"
        aria-hidden="true"
      />
    </div>
  </div>
</template>
