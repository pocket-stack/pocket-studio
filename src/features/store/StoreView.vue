<script setup lang="ts">
import { onMounted } from "vue";
import { useI18n } from "vue-i18n";

import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import type { PackageCategory } from "../../shared/gateway";
import AppIcon from "../../shared/ui/AppIcon.vue";
import StatusPill from "../../shared/ui/StatusPill.vue";
import PackageCard from "./PackageCard.vue";
import PackageDetail from "./PackageDetail.vue";
import { useStore } from "./useStore";

const emit = defineEmits<{ prepare: [] }>();
const { t } = useI18n();
const store = useStore();
const { device, readiness } = useDeviceSession();

const categories: Array<PackageCategory | "all"> = [
  "all",
  "runtime",
  "tool",
  "app",
  "game",
];

onMounted(() => void store.initialize());
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-5">
    <header class="flex flex-wrap items-end justify-between gap-3">
      <div>
        <h1 class="text-2xl font-semibold tracking-tight">
          {{ t("store.title") }}
        </h1>
        <p class="mt-1 text-sm text-muted">{{ t("store.subtitle") }}</p>
      </div>
      <div class="flex items-center gap-2 text-xs">
        <StatusPill v-if="!device" tone="neutral" dot>{{
          t("store.target.none")
        }}</StatusPill>
        <StatusPill
          v-else-if="readiness?.status === 'ready'"
          tone="success"
          dot
        >
          {{ t("store.target.ready", { name: device.marketingName }) }}
        </StatusPill>
        <StatusPill v-else tone="warning" dot>{{
          t("store.target.notReady", { name: device.marketingName })
        }}</StatusPill>
      </div>
    </header>

    <div class="flex flex-wrap items-center gap-2">
      <div class="flex gap-1 rounded-lg border border-line bg-surface p-1">
        <button
          v-for="category in categories"
          :key="category"
          class="rounded-md px-3 py-1 text-xs font-medium transition"
          :class="
            store.categoryFilter.value === category
              ? 'bg-signal text-on-signal'
              : 'text-muted hover:text-ink'
          "
          @click="store.categoryFilter.value = category"
        >
          {{
            category === "all"
              ? t("store.category.all")
              : t(`store.category.${category}`)
          }}
        </button>
      </div>
      <input
        v-model="store.query.value"
        type="search"
        class="field ml-auto w-56"
        :placeholder="t('store.search')"
      />
    </div>

    <div
      class="grid min-h-0 flex-1 gap-5"
      :class="store.selected.value ? 'lg:grid-cols-[minmax(0,1fr)_420px]' : ''"
    >
      <div class="scroll-thin min-h-0 overflow-y-auto pr-1">
        <div
          v-if="store.loading.value"
          class="flex items-center gap-3 text-sm text-muted"
        >
          <span
            class="spin block h-4 w-4 rounded-full border-2 border-signal border-t-transparent"
          />
          {{ t("store.loading") }}
        </div>
        <div
          v-else-if="store.loadError.value"
          class="card p-6 text-sm text-danger"
        >
          {{ t("store.loadFailed") }}
        </div>
        <div
          v-else-if="store.packages.value.length === 0"
          class="card p-8 text-center text-sm text-muted"
        >
          <AppIcon name="store" :size="28" class="mx-auto mb-2 opacity-50" />
          {{ t("store.empty") }}
        </div>
        <div v-else class="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
          <PackageCard
            v-for="item in store.packages.value"
            :key="item.entry.id"
            :item="item"
            :selected="store.selectedId.value === item.entry.id"
            @select="store.select(item.entry.id)"
          />
        </div>
      </div>
      <PackageDetail
        v-if="store.selected.value"
        class="rise min-h-0"
        :item="store.selected.value"
        @install="store.install(store.selected.value.entry.id)"
        @cancel="store.cancelInstall(store.selected.value.entry.id)"
        @prepare="emit('prepare')"
        @close="store.select(null)"
      />
    </div>
  </div>
</template>
