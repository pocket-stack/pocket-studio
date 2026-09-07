<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";
import { useGateway, type PackageCategory } from "../../shared/gateway";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { useOperations } from "../../shared/composables/useOperations";
import PackageCard from "./PackageCard.vue";
import PackageDetail from "./PackageDetail.vue";
import PackageArtwork from "./PackageArtwork.vue";
import { packageText } from "./packageContent";
import { useStore } from "./useStore";
const emit = defineEmits<{
  prepare: [];
  openInstalled: [];
  openEnvironment: [];
  openPackage: [id: string | null];
}>();
const { t, d, te, locale } = useI18n();
const store = useStore(),
  gateway = useGateway();
const { device } = useDeviceSession();
const { active } = useOperations();
const feature = ref(0);
const featured = computed(() =>
  [...store.packages.value]
    .sort((a, b) => b.entry.publishedAt - a.entry.publishedAt)
    .slice(0, 3),
);
const featuredPackage = computed(
  () => featured.value[feature.value % Math.max(1, featured.value.length)],
);
const filtered = computed(
  () =>
    !!store.query.value ||
    store.categoryFilter.value !== "all" ||
    store.compatibleOnly.value,
);
const sections = computed(() =>
  filtered.value
    ? [{ id: "results", items: store.packages.value }]
    : [
        {
          id: "apps",
          items: store.packages.value.filter((item) =>
            ["app", "game"].includes(item.entry.category),
          ),
        },
        {
          id: "tools",
          items: store.packages.value.filter((item) =>
            ["runtime", "tool"].includes(item.entry.category),
          ),
        },
      ].filter((section) => section.items.length),
);
const categories: Array<PackageCategory | "all"> = [
  "all",
  "app",
  "game",
  "tool",
  "runtime",
];
const issue = computed(
  () => store.loadError.value ?? store.snapshot.value?.issue,
);
const issueText = computed(() => {
  const key = `store.sourceErrors.${issue.value}`;
  return te(key) ? t(key) : t("store.loadFailed");
});
const sourceLabel = computed(() =>
  store.snapshot.value?.source === "demo"
    ? t("store.source.demo")
    : (store.snapshot.value?.sourceLabel ?? t("store.source.unconfigured")),
);
onMounted(() => void store.initialize());
</script>

<template>
  <section
    v-if="!gateway.capabilities.catalog"
    class="flex min-h-full items-center justify-center p-8 text-muted"
  >
    {{ t("store.loadFailed") }}
  </section>
  <div
    v-else-if="store.selected.value"
    class="mx-auto min-h-full motion-safe:animate-rise"
  >
    <button
      class="mb-5 inline-flex items-center gap-1 text-xs text-signal hover:underline"
      @click="emit('openPackage', null)"
    >
      <IconStudioArrowLeft width="14" height="14" />{{
        t("studio.backToStore")
      }}
    </button>
    <PackageDetail
      :item="store.selected.value"
      @install="store.install(store.selected.value.entry.id)"
      @cancel="store.cancelInstall(store.selected.value.entry.id)"
      @prepare="emit('prepare')"
      @close="emit('openPackage', null)"
      @dependency="emit('openPackage', $event)"
    />
  </div>
  <div v-else class="mx-auto min-h-full motion-safe:animate-rise">
    <header
      class="mb-5 flex items-center justify-between gap-5 border-b border-line pb-4"
    >
      <div>
        <h1 class="text-xl font-semibold">{{ t("store.title") }}</h1>
        <p class="mt-1 max-w-2xl text-xs leading-6 text-muted">
          {{ t("store.subtitle") }}
        </p>
      </div>
      <button
        class="inline-flex shrink-0 items-center gap-2 rounded-md border border-line bg-raised px-3 py-1.5 text-xs hover:border-muted disabled:opacity-50"
        :disabled="store.loading.value"
        @click="store.reload"
      >
        <IconStudioRefresh
          width="14"
          height="14"
          :class="{ 'motion-safe:animate-studio-spin': store.loading.value }"
        />{{ t("store.refresh") }}
      </button>
    </header>
    <div
      v-if="issue || store.snapshot.value?.expired"
      class="mb-5 flex items-start gap-2.5 rounded-md border border-warning/30 bg-warning/5 px-4 py-3 text-xs leading-6"
      role="status"
    >
      <IconStudioInfo
        width="16"
        height="16"
        class="mt-1 shrink-0 text-warning"
      />
      <p>
        {{
          store.snapshot.value?.expired ? t("store.source.expired") : issueText
        }}<span v-if="store.snapshot.value?.verified" class="ml-2 text-muted">{{
          t("store.source.cachedContent")
        }}</span>
      </p>
    </div>
    <section
      v-if="featuredPackage && !filtered"
      class="mb-6 overflow-hidden rounded-lg border border-line bg-raised"
      :aria-label="t('studio.featured')"
    >
      <div
        class="flex min-h-[190px] items-center gap-8 px-8 py-7 max-[850px]:gap-5 max-[850px]:px-5"
      >
        <PackageArtwork
          :entry="featuredPackage.entry"
          :package-id="featuredPackage.entry.id"
          :size="116"
          class="shadow-[0_6px_20px_#00000012]"
        />
        <div class="min-w-0 flex-1">
          <span
            class="text-[10px] font-medium uppercase tracking-[0.14em] text-muted"
            >{{ t("store.fromCatalog") }}</span
          >
          <h2 class="mt-2 truncate text-2xl font-semibold tracking-tight">
            {{ packageText(featuredPackage.entry, "name", locale, t) }}
          </h2>
          <p class="mt-2 max-w-xl text-sm leading-6 text-muted">
            {{ packageText(featuredPackage.entry, "summary", locale, t) }}
          </p>
          <button
            class="mt-4 inline-flex items-center gap-2 rounded-md bg-signal px-3.5 py-1.5 text-xs text-on-signal hover:brightness-110"
            @click="emit('openPackage', featuredPackage.entry.id)"
          >
            {{ t("studio.viewDetails")
            }}<IconStudioArrowRight width="13" height="13" />
          </button>
        </div>
        <div v-if="featured.length > 1" class="flex shrink-0 gap-1.5">
          <button
            v-for="(_, index) in featured"
            :key="index"
            class="p-2 after:block after:size-1.5 after:rounded-full after:bg-line aria-pressed:after:bg-signal"
            :aria-label="t('studio.featurePage', { index: index + 1 })"
            :aria-pressed="feature % featured.length === index"
            @click="feature = index"
          />
        </div>
      </div>
    </section>
    <div class="flex gap-7">
      <div class="min-w-0 flex-1">
        <div
          v-if="filtered"
          class="mb-5 flex items-center justify-between text-xs"
        >
          <span>{{
            t("studio.searchResults", { count: store.packages.value.length })
          }}</span
          ><button
            class="text-signal hover:underline"
            @click="
              store.query.value = '';
              store.categoryFilter.value = 'all';
              store.compatibleOnly.value = false;
            "
          >
            {{ t("studio.clearFilters") }}
          </button>
        </div>
        <div
          v-if="store.loading.value && !store.catalog.value.length"
          class="flex min-h-[250px] items-center justify-center gap-2 text-xs text-muted"
        >
          <IconStudioSpinner class="motion-safe:animate-studio-spin" />{{
            t("store.loading")
          }}
        </div>
        <div
          v-else-if="!store.packages.value.length"
          class="flex min-h-[250px] flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-line px-6 text-center text-muted"
        >
          <IconStudioGrid width="32" height="32" />
          <p class="text-sm">{{ issue ? issueText : t("store.empty") }}</p>
          <p v-if="!issue && !filtered" class="text-xs leading-6">
            {{ t("store.emptyCatalog") }}
          </p>
        </div>
        <section
          v-for="section in sections"
          v-else
          :key="section.id"
          class="mb-7"
        >
          <header
            v-if="section.id !== 'results'"
            class="mb-4 flex items-center justify-between border-b border-line pb-2"
          >
            <h2 class="text-base font-semibold">
              {{ t(`store.collection.${section.id}`) }}
            </h2>
            <span class="text-xs tabular-nums text-muted">{{
              section.items.length
            }}</span>
          </header>
          <div
            class="grid grid-cols-[repeat(auto-fill,minmax(108px,1fr))] gap-x-5 gap-y-6"
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
        </section>
      </div>
      <aside
        class="w-[205px] shrink-0 border-l border-line pl-6 max-[1150px]:w-[180px] max-[850px]:w-40"
      >
        <h2 class="mb-2 text-sm font-semibold">{{ t("studio.category") }}</h2>
        <label class="sr-only" for="store-category">{{
          t("studio.category")
        }}</label>
        <select
          id="store-category"
          v-model="store.categoryFilter.value"
          class="w-full rounded-md border border-line bg-raised px-2 py-1.5 text-xs text-ink focus:border-signal"
        >
          <option
            v-for="category in categories"
            :key="category"
            :value="category"
          >
            {{ t(`store.category.${category}`) }}
          </option>
        </select>
        <label class="mt-3 flex items-start gap-2 text-xs leading-5 text-muted"
          ><input
            v-model="store.compatibleOnly.value"
            type="checkbox"
            class="mt-1 accent-signal"
            :disabled="!device"
          />{{ t("store.compatibleOnly") }}</label
        >
        <h3 class="mb-2 mt-6 text-xs text-muted">
          {{ t("studio.quickLinks") }}
        </h3>
        <button
          class="flex w-full items-center justify-between py-1.5 text-left text-xs hover:text-signal"
          @click="emit('openInstalled')"
        >
          {{ t("studio.installedApps")
          }}<span class="text-muted">{{
            gateway.capabilities.packages ? store.installed.value.length : "—"
          }}</span>
        </button>
        <button
          class="flex w-full items-center justify-between py-1.5 text-left text-xs hover:text-signal"
          @click="emit('openInstalled')"
        >
          {{ t("studio.queue")
          }}<span class="text-muted">{{
            active.length + store.queuedIds.value.length
          }}</span>
        </button>
        <h3 class="mb-2 mt-6 text-xs text-muted">{{ t("studio.sources") }}</h3>
        <div class="flex items-center gap-2 text-xs">
          <IconStudioPocket
            width="16"
            height="16"
            class="shrink-0 text-signal"
          /><span class="truncate">{{ sourceLabel }}</span
          ><IconStudioShield
            v-if="store.snapshot.value?.verified"
            width="14"
            height="14"
            class="ml-auto shrink-0 text-success"
          />
        </div>
        <p class="mt-2 text-[11px] leading-5 text-muted">
          {{
            store.snapshot.value?.verified
              ? t("store.source.verified")
              : store.snapshot.value?.source === "demo"
                ? t("store.source.demoNotice")
                : t("store.source.unconfigured")
          }}
        </p>
        <p
          v-if="store.snapshot.value?.checkedAt"
          class="mt-2 text-[11px] leading-5 text-muted"
        >
          {{
            t("store.source.checkedAt", {
              date: d(store.snapshot.value.checkedAt, "date"),
            })
          }}
        </p>
        <button
          class="mt-6 inline-flex items-center gap-1 text-xs text-signal hover:underline"
          @click="emit('openEnvironment')"
        >
          {{ t("studio.environmentGuide")
          }}<IconStudioChevronRight width="12" height="12" />
        </button>
      </aside>
    </div>
  </div>
</template>
