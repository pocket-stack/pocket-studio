<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { operationProgress } from "../../shared/composables/useOperations";
import ProgressBar from "../../shared/ui/ProgressBar.vue";
import PackageArtwork from "./PackageArtwork.vue";
import type { PackageView } from "./useStore";
import { useGateway } from "../../shared/gateway";
import { packageText } from "./packageContent";
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
</script>
<template>
  <article class="min-w-0">
    <button
      class="group/package block w-full text-left"
      :aria-label="t('studio.viewApp', { name })"
      @click="emit('select')"
    >
      <PackageArtwork
        class="aspect-square h-auto! w-full! max-w-none transition-[transform,box-shadow] duration-200 group-hover/package:-translate-y-[3px] group-hover/package:shadow-[inset_0_1px_1px_#ffffff80,0_8px_16px_#00000012]"
        :package-id="item.entry.id"
        :entry="item.entry"
      />
      <h3
        class="mt-[5px] truncate text-[13px] font-semibold max-[1150px]:text-[11px]"
      >
        {{ name }}
      </h3>
      <p
        class="mt-[3px] flex gap-[5px] text-[11px] text-muted max-[1150px]:text-[10px]"
      >
        {{ t(`store.category.${item.entry.category}`) }}<span>·</span
        >{{
          (item.entry.details?.app.publisher.verified ??
          item.entry.developer === "PocketJS")
            ? t("studio.official")
            : t("studio.community")
        }}
      </p>
    </button>
    <div
      v-if="state === 'installing' && item.operation"
      class="mt-3 flex items-center gap-[7px] text-[10px] text-signal"
    >
      <ProgressBar
        class="flex-1"
        :percent="operationProgress(item.operation)"
        compact
        active
      /><span>{{ operationProgress(item.operation) }}%</span>
    </div>
    <button
      v-else
      :data-muted="state !== 'details' && state !== 'update'"
      class="mt-[5px] inline-flex min-h-[22px] min-w-[45px] items-center justify-center rounded-[5px] border border-[#2f6fd6] bg-[#2f6fd6] px-2.5 py-px text-[12px] text-white enabled:hover:bg-[#255cad] enabled:hover:text-white disabled:opacity-55 data-[muted=true]:border-[#b5b5b5] data-[muted=true]:bg-canvas data-[muted=true]:text-muted data-[muted=true]:enabled:hover:bg-[#255cad] data-[muted=true]:enabled:hover:text-white"
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
      @click="
        state === 'details' || state === 'update'
          ? emit('install')
          : emit('select')
      "
    >
      {{
        state === "details"
          ? t("store.detail.install")
          : state === "update"
            ? t("store.detail.update")
            : state === "queued"
              ? t("studio.queued", { position: item.queuePosition })
              : state === "failed"
                ? t("preparation.result.retry")
                : t(`store.state.${state}`)
      }}
    </button>
  </article>
</template>
