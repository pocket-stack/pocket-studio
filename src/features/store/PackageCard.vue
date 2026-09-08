<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import {
  operationProgress,
  operationStepCeiling,
} from "../../shared/composables/useOperations";
import ProgressBar from "../../shared/ui/ProgressBar.vue";
import PackageArtwork from "./PackageArtwork.vue";
import type { PackageView } from "./useStore";
import { useGateway } from "../../shared/gateway";
import { packageText } from "./packageContent";
import { CARD_PAD } from "./storeLayout";

const props = defineProps<{ item: PackageView; size: number }>();
const emit = defineEmits<{ select: []; install: [] }>();
const { t, locale } = useI18n();
const gateway = useGateway();
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
// App Store style: a price-like pill when the app can be acted on, quiet
// text otherwise.
const actionable = computed(
  () =>
    gateway.capabilities.packages &&
    ["details", "update", "failed"].includes(state.value),
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
  <article
    class="flex min-w-0 flex-col"
    :style="{ width: `${size + CARD_PAD}px` }"
  >
    <button
      class="group/package flex flex-col text-left"
      :aria-label="t('studio.viewApp', { name })"
      @click="emit('select')"
    >
      <PackageArtwork
        class="rounded-[22%] transition-[transform,box-shadow] duration-200 group-hover/package:-translate-y-0.5 group-hover/package:shadow-raised"
        :package-id="item.entry.id"
        :entry="item.entry"
        :size="size"
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
    <div class="mt-1.5 flex h-[22px] items-center">
      <template v-if="state === 'installing' && item.operation">
        <ProgressBar
          class="flex-1"
          :percent="operationProgress(item.operation)"
          :trickle-to="operationStepCeiling(item.operation)"
          compact
          active
        /><span class="ml-1.5 text-2xs text-signal tabular-nums"
          >{{ operationProgress(item.operation) }}%</span
        >
      </template>
      <button
        v-else-if="actionable"
        class="inline-flex h-[22px] min-w-[54px] items-center justify-center gap-1 rounded-full px-3 text-xs font-bold transition-colors disabled:cursor-not-allowed disabled:opacity-45"
        :class="
          state === 'failed'
            ? 'bg-danger/10 text-danger enabled:hover:bg-danger/16'
            : 'bg-ink/8 text-signal enabled:hover:bg-ink/12'
        "
        :disabled="item.pending"
        @click="emit('install')"
      >
        <IconSvgSpinners90Ring v-if="item.pending" width="11" height="11" />{{
          label
        }}
      </button>
      <button
        v-else
        class="truncate text-2xs text-muted hover:text-ink"
        @click="emit('select')"
      >
        {{ label }}
      </button>
    </div>
  </article>
</template>
