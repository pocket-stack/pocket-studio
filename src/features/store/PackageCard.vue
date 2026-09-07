<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import {
  operationProgress,
  operationStepCeiling,
} from "../../shared/composables/useOperations";
import ProgressBar from "../../shared/ui/ProgressBar.vue";
import StudioButton from "../../shared/ui/StudioButton.vue";
import PackageArtwork from "./PackageArtwork.vue";
import type { PackageView } from "./useStore";
import { useGateway } from "../../shared/gateway";
import { packageText } from "./packageContent";

import { CARD_ART } from "./storeLayout";

const props = defineProps<{ item: PackageView }>();
const emit = defineEmits<{ select: []; install: [] }>();
const { t, locale } = useI18n();
const name = computed(() =>
  packageText(props.item.entry, "name", locale.value, t),
);
const state = computed(() => {
  if (props.item.queuePosition) return "queued";
  if (props.item.operation?.status === "running") return "installing";
  if (props.item.operation?.status === "failed") return "failed";
  if (
    [
      "withdrawn",
      "catalogExpired",
      "unsupportedInstaller",
      "unknownDevice",
    ].includes(props.item.verdict)
  )
    return "unavailable";
  if (props.item.installed)
    return props.item.installed.version === props.item.entry.version
      ? "installed"
      : "update";
  if (props.item.verdict === "requiresPreparation")
    return "requiresPreparation";
  if (props.item.verdict === "noDevice") return "noDevice";
  if (
    props.item.verdict === "unsupportedModel" ||
    props.item.verdict === "unsupportedOs"
  )
    return "incompatible";
  return "details";
});
const actionable = computed(
  () => state.value === "details" || state.value === "update",
);
const label = computed(() => {
  switch (state.value) {
    case "details":
      return t("store.detail.install");
    case "update":
      return t("store.detail.update");
    case "queued":
      return t("studio.queued", { position: props.item.queuePosition });
    case "failed":
      return t("preparation.result.retry");
    default:
      return t(`store.state.${state.value}`);
  }
});
</script>
<template>
  <article class="flex min-w-0 flex-col" :style="{ width: `${CARD_ART}px` }">
    <button
      class="group/package flex flex-col text-left"
      :aria-label="t('studio.viewApp', { name })"
      @click="emit('select')"
    >
      <PackageArtwork
        class="rounded-[18%] transition-[transform,box-shadow] duration-200 group-hover/package:-translate-y-0.5 group-hover/package:shadow-raised"
        :package-id="item.entry.id"
        :entry="item.entry"
        :size="CARD_ART"
      />
      <h3 class="mt-1.5 w-full truncate text-sm leading-[18px] font-semibold">
        {{ name }}
      </h3>
      <p class="w-full truncate text-2xs text-muted">
        {{ t(`store.category.${item.entry.category}`) }} ·
        {{
          (item.entry.details?.app.publisher.verified ??
          item.entry.developer === "PocketJS")
            ? t("studio.official")
            : t("studio.community")
        }}
      </p>
    </button>
    <div
      v-if="state === 'installing' && item.operation"
      class="mt-2 flex h-6 items-center gap-1.5 text-2xs text-signal tabular-nums"
    >
      <ProgressBar
        class="flex-1"
        :percent="operationProgress(item.operation)"
        :trickle-to="operationStepCeiling(item.operation)"
        compact
        active
      /><span>{{ operationProgress(item.operation) }}%</span>
    </div>
    <StudioButton
      v-else
      size="sm"
      class="mt-1.5 min-w-[52px] self-start"
      :variant="actionable ? 'primary' : 'secondary'"
      :disabled="
        !useGateway().capabilities.packages ||
        [
          'incompatible',
          'installed',
          'noDevice',
          'queued',
          'unavailable',
        ].includes(state)
      "
      @click="actionable ? emit('install') : emit('select')"
    >
      {{ label }}
    </StudioButton>
  </article>
</template>
