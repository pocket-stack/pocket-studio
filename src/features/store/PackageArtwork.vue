<script setup lang="ts">
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import type { CatalogEntry } from "../../shared/gateway";
import { packageMedia } from "./packageContent";
import StoreMedia from "./StoreMedia.vue";

const props = defineProps<{
  packageId: string;
  size?: number;
  entry?: CatalogEntry;
}>();
const { locale } = useI18n();
const icon = computed(() =>
  props.entry ? packageMedia(props.entry, "icon", locale.value)[0] : undefined,
);
const failedPackageId = ref<string>();
</script>

<template>
  <StoreMedia
    v-if="icon"
    :blob="icon.blob"
    class="inline-block shrink-0 rounded-[18%]"
    :style="{ width: `${size ?? 80}px`, height: `${size ?? 80}px` }"
  />
  <span
    v-else-if="entry?.details"
    class="inline-flex shrink-0 items-center justify-center rounded-[18%] border border-line bg-track text-muted"
    :style="{ width: `${size ?? 80}px`, height: `${size ?? 80}px` }"
    ><IconStudioGrid width="32" height="32" aria-hidden="true"
  /></span>
  <img
    v-else
    :src="`/vectors/packages/${failedPackageId === packageId ? 'pocket-runtime' : packageId}.svg`"
    :width="size ?? 80"
    :height="size ?? 80"
    alt=""
    class="inline-block shrink-0"
    @error="failedPackageId = packageId"
  />
</template>
