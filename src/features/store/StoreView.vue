<script setup lang="ts">
import { useGateway } from "../../shared/gateway";
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { useOperations } from "../../shared/composables/useOperations";
import type { PackageCategory } from "../../shared/gateway";
import AppIcon from "../../shared/ui/AppIcon.vue";
import PackageCard from "./PackageCard.vue";
import PackageDetail from "./PackageDetail.vue";
import { useStore } from "./useStore";
const emit = defineEmits<{
  prepare: [];
  openInstalled: [];
  openEnvironment: [];
  openPackage: [id: string | null];
}>();
const { t } = useI18n();
const store = useStore();
const gateway = useGateway();
const { device } = useDeviceSession();
const { active } = useOperations();
const feature = ref(0);
const featuredIds = ["pocket-runtime", "pocket-reader", "pocket-arcade"];
const featuredId = computed(
  () => featuredIds[feature.value] ?? "pocket-runtime",
);
const categories: Array<PackageCategory | "all"> = [
  "all",
  "runtime",
  "tool",
  "app",
  "game",
];
const sections = computed(() =>
  store.query.value || store.categoryFilter.value !== "all"
    ? [{ id: "results", items: store.packages.value }]
    : [
        {
          id: "essentials",
          items: store.packages.value.filter((item) =>
            ["runtime", "tool"].includes(item.entry.category),
          ),
        },
        {
          id: "madeForDevice",
          items: store.packages.value.filter((item) =>
            ["app", "game"].includes(item.entry.category),
          ),
        },
      ],
);
onMounted(() => void store.initialize());
</script>
<template>
  <section
    v-if="!gateway.capabilities.packages"
    class="flex min-h-full flex-col items-center justify-center gap-4 p-8 text-center"
  >
    <AppIcon name="grid" :size="32" class="text-muted" />
    <h1 class="text-xl font-semibold">{{ t("studio.nav.store") }}</h1>
    <p class="max-w-[500px] text-sm leading-7 text-muted">
      {{ t("connection.packagesUnavailable") }}
    </p>
  </section>
  <div
    v-else-if="store.selected.value"
    class="mx-auto min-h-full max-w-none motion-safe:animate-rise"
  >
    <button
      class="mb-5 inline-flex items-center gap-[5px] text-[11px] text-signal hover:underline hover:underline-offset-[3px]"
      @click="emit('openPackage', null)"
    >
      <AppIcon name="arrowLeft" :size="14" />{{
        t("studio.backToStore")
      }}</button
    ><PackageDetail
      :item="store.selected.value"
      @install="store.install(store.selected.value.entry.id)"
      @cancel="store.cancelInstall(store.selected.value.entry.id)"
      @prepare="emit('prepare')"
      @close="emit('openPackage', null)"
      @dependency="emit('openPackage', $event)"
    />
  </div>
  <div v-else class="mx-auto min-h-full max-w-none motion-safe:animate-rise">
    <section
      v-if="!store.query.value && store.categoryFilter.value === 'all'"
      class="flex h-[214px] items-center pt-0"
      :aria-label="t('studio.featured')"
    >
      <button
        class="relative flex h-44 flex-1 items-center justify-center gap-[15px] overflow-hidden bg-[#c4c4c4] text-[#647a8e] rounded-l-lg"
        :aria-label="t('studio.previousFeature')"
        @click="feature = (feature + 2) % 3"
      >
        <AppIcon
          name="chevronRight"
          class="rotate-180 opacity-35 absolute left-[15px]"
          :size="17"
        />
      </button>
      <div
        :key="feature"
        :data-feature="feature"
        class="relative z-[1] flex h-[196px] max-w-[65%] flex-[0_0_620px] items-center overflow-hidden rounded-lg bg-[#3a3a3a] px-10 py-0 text-white shadow-none data-[feature=1]:bg-[#435462] data-[feature=2]:bg-[#4b5752] max-[600px]:max-w-[88%] max-[600px]:basis-[88%] max-[600px]:p-6 motion-safe:animate-rise"
      >
        <div class="relative z-[2]">
          <span
            class="flex items-center gap-2.5 text-[12px] tracking-[0.08em] text-[#b5b5b5]"
            >{{ t("studio.featured") }}<i class="hidden" />{{
              t("studio.forLegacy")
            }}</span
          >
          <h2
            class="mt-2 text-[24px] leading-[1.35] font-semibold tracking-[-0.4px] whitespace-pre-line"
          >
            {{ t(`studio.features.${feature}.title`) }}
          </h2>
          <p class="mt-2 max-w-none text-[13px] leading-[1.8] text-[#c4c4c4]">
            {{ t(`studio.features.${feature}.body`) }}
          </p>
          <button
            class="mt-3.5 flex items-center gap-3.5 rounded border border-[#9fbde6] bg-[#ffffff0b] px-3.5 py-[5px] text-[13px] text-white hover:bg-[#ffffff20]"
            @click="
              feature === 0
                ? emit('openEnvironment')
                : emit('openPackage', featuredId)
            "
          >
            {{ t("studio.explore") }}<AppIcon name="arrowRight" :size="14" />
          </button>
        </div>
      </div>
      <button
        class="relative flex h-44 flex-1 items-center justify-center gap-[15px] overflow-hidden bg-[#c4c4c4] text-[#647a8e] rounded-r-lg"
        :aria-label="t('studio.nextFeature')"
        @click="feature = (feature + 1) % 3"
      >
        <AppIcon
          class="opacity-35 absolute right-[15px]"
          name="chevronRight"
          :size="17"
        />
      </button>
    </section>
    <div
      v-if="!store.query.value && store.categoryFilter.value === 'all'"
      class="flex justify-center gap-[3px] px-0 pt-1 pb-5"
    >
      <button
        v-for="index in 3"
        :key="index"
        class="p-[3px] after:block after:size-1.5 after:rounded-full after:bg-line after:content-[''] aria-pressed:after:bg-signal"
        :aria-label="t('studio.featurePage', { index })"
        :aria-pressed="feature === index - 1"
        @click="feature = index - 1"
      />
    </div>
    <div class="flex gap-7">
      <div class="min-w-0 flex-1">
        <div
          v-if="store.query.value || store.categoryFilter.value !== 'all'"
          class="flex items-center justify-between pb-5 text-[12px]"
        >
          <span>{{
            t("studio.searchResults", { count: store.packages.value.length })
          }}</span
          ><button
            class="inline-flex items-center gap-[5px] text-[11px] text-signal hover:underline hover:underline-offset-[3px]"
            @click="
              store.query.value = '';
              store.categoryFilter.value = 'all';
            "
          >
            {{ t("studio.clearFilters") }}
          </button>
        </div>
        <div
          v-if="store.loading.value"
          class="flex min-h-[250px] items-center justify-center gap-2.5 text-[12px] text-muted"
        >
          <AppIcon name="refresh" class="motion-safe:animate-studio-spin" />{{
            t("store.loading")
          }}
        </div>
        <div
          v-else-if="store.loadError.value"
          class="flex min-h-[250px] items-center justify-center gap-2.5 text-[12px] text-muted"
        >
          {{ t("store.loadFailed")
          }}<button
            class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
            @click="store.reload"
          >
            {{ t("studio.retry") }}
          </button>
        </div>
        <template v-else
          ><section
            v-for="section in sections"
            :key="section.id"
            class="mb-[18px]"
          >
            <header
              v-if="section.id !== 'results'"
              class="mb-3 flex items-center gap-3.5 border-b border-line pb-1.5 max-[600px]:flex-wrap max-[600px]:gap-[5px]"
            >
              <h2 class="text-[18px] font-semibold">
                {{
                  section.id === "madeForDevice" && device
                    ? t("studio.adaptedFor", { name: t("studio.deviceName") })
                    : t(`studio.storeSections.${section.id}`)
                }}
              </h2>
              <button
                class="ml-auto inline-flex items-center gap-[5px] text-[11px] text-signal hover:underline hover:underline-offset-[3px]"
                @click="
                  store.categoryFilter.value =
                    section.id === 'essentials' ? 'tool' : 'app'
                "
              >
                {{ t("studio.viewAll")
                }}<AppIcon name="chevronRight" :size="12" />
              </button>
            </header>
            <div
              v-if="section.items.length"
              class="grid grid-cols-7 gap-4 max-[1150px]:gap-3 max-[850px]:grid-cols-4 max-[850px]:gap-y-6 max-[600px]:grid-cols-3"
            >
              <PackageCard
                v-for="item in section.items"
                :key="item.entry.id"
                :item="item"
                @select="emit('openPackage', item.entry.id)"
                @install="
                  item.missingDependencies.length
                    ? emit('openPackage', item.entry.id)
                    : store.install(item.entry.id)
                "
              />
            </div>
            <div
              v-else
              class="flex min-h-[250px] items-center justify-center gap-2.5 text-[12px] text-muted"
            >
              <AppIcon name="search" :size="26" />{{ t("store.empty") }}
            </div>
          </section></template
        >
      </div>
      <aside
        class="w-[220px] shrink-0 border-l border-line pl-6 max-[1150px]:w-[190px] max-[850px]:w-40 max-[600px]:hidden"
      >
        <h2 class="mb-2 text-[18px] font-semibold">{{ t("store.title") }}</h2>
        <label class="sr-only" for="store-category">{{
          t("studio.category")
        }}</label
        ><select
          id="store-category"
          v-model="store.categoryFilter.value"
          class="w-full border bg-raised text-ink focus:border-signal rounded-md border-[#b5b5b5] px-2.5 py-1 text-[12px]"
        >
          <option
            v-for="category in categories"
            :key="category"
            :value="category"
          >
            {{ t(`store.category.${category}`) }}
          </option>
        </select>
        <h3 class="mt-3.5 mb-2 text-[12px] text-muted">
          {{ t("studio.quickLinks") }}
        </h3>
        <button
          class="flex w-full items-center justify-between gap-2.5 px-0 py-[5px] text-left text-[13px] hover:text-signal"
          @click="emit('openInstalled')"
        >
          {{ t("studio.installedApps")
          }}<span class="text-[10px] text-muted">{{
            store.installed.value.length
          }}</span></button
        ><button
          class="flex w-full items-center justify-between gap-2.5 px-0 py-[5px] text-left text-[13px] hover:text-signal"
          @click="emit('openInstalled')"
        >
          {{ t("studio.queue")
          }}<span class="text-[10px] text-muted">{{
            active.length + store.queuedIds.value.length
          }}</span>
        </button>
        <h3 class="mt-3.5 mb-2 text-[12px] text-muted">
          {{ t("studio.sources") }}
        </h3>
        <div class="flex items-center gap-2 px-0 py-2 text-[12px]">
          <span class="rounded-md bg-track p-1.5 text-signal"
            ><AppIcon name="pocket" :size="14" /></span
          ><span
            ><b class="text-[12px] font-medium">{{ t("studio.demoCatalog") }}</b
            ><small class="mt-[3px] block text-[11px] text-muted">{{
              t("studio.bundledCatalog")
            }}</small></span
          ><AppIcon name="check" :size="13" class="ml-auto text-success" />
        </div>
        <h3 class="mt-3.5 mb-2 text-[12px] text-muted">
          {{ t("studio.support") }}
        </h3>
        <button
          class="flex w-full items-center justify-between gap-2.5 px-0 py-[5px] text-left text-[13px] hover:text-signal"
          @click="emit('openEnvironment')"
        >
          {{ t("studio.environmentGuide")
          }}<AppIcon name="chevronRight" :size="12" /></button
        ><a
          class="flex w-full items-center justify-between gap-2.5 px-0 py-[5px] text-left text-[13px] hover:text-signal"
          href="https://github.com/HalfSweet/Legacy-iOS-Kit-rs"
          target="_blank"
          rel="noreferrer"
          >{{ t("studio.toolkitDocs") }}<AppIcon name="external" :size="12"
        /></a>
      </aside>
    </div>
    <p class="mt-[15px] border-t border-line pt-3 text-[10px] text-muted">
      {{ t("studio.catalogNotice") }}
    </p>
  </div>
</template>
