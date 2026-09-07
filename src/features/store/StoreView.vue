<script setup lang="ts">
import {
  computed,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  watch,
} from "vue";
import { useI18n } from "vue-i18n";
import { useGateway, type PackageCategory } from "../../shared/gateway";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { useElementSize } from "../../shared/composables/useElementSize";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioPanel from "../../shared/ui/StudioPanel.vue";
import StudioSegmented from "../../shared/ui/StudioSegmented.vue";
import PackageCard from "./PackageCard.vue";
import PackageDetail from "./PackageDetail.vue";
import PackageArtwork from "./PackageArtwork.vue";
import { packageText } from "./packageContent";
import { CARD_ART, CARD_GAP, CARD_HEIGHT, SECTION_HEADER } from "./storeLayout";
import { useStore } from "./useStore";

const emit = defineEmits<{
  prepare: [];
  openInstalled: [];
  openEnvironment: [];
  openPackage: [id: string | null];
}>();
const { t, d, te, locale } = useI18n();
const store = useStore();
const gateway = useGateway();
const { device } = useDeviceSession();

const page = ref<HTMLElement | null>(null);
const { width, height } = useElementSize(page);
const CHROME_HEIGHT = 98; // header + footer + gaps
const HERO_HEIGHT = 150;
const HERO_COMPACT_HEIGHT = 104;

const featured = computed(() =>
  [...store.packages.value]
    .sort((a, b) => b.entry.publishedAt - a.entry.publishedAt)
    .slice(0, 3),
);
const feature = ref(0);
const hovering = ref(false);
let rotation: number | undefined;
function rotate(step: number): void {
  const count = featured.value.length;
  if (!count) return;
  feature.value = (feature.value + step + count) % count;
}
onMounted(() => {
  rotation = window.setInterval(() => {
    if (!hovering.value && featured.value.length > 1) rotate(1);
  }, 7000);
});
onBeforeUnmount(() => window.clearInterval(rotation));

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
const columns = computed(() =>
  Math.max(3, Math.floor((width.value + CARD_GAP) / (CARD_ART + CARD_GAP))),
);
const available = computed(() => height.value - CHROME_HEIGHT);
const spare = computed(
  () =>
    available.value - sections.value.length * (CARD_HEIGHT + SECTION_HEADER),
);
const showHero = computed(
  () =>
    !filtered.value &&
    featured.value.length > 0 &&
    spare.value >= HERO_COMPACT_HEIGHT + 8,
);
const heroHeight = computed(() =>
  spare.value >= HERO_HEIGHT + 20 ? HERO_HEIGHT : HERO_COMPACT_HEIGHT,
);
const rowsPerSection = computed(() =>
  filtered.value
    ? Math.max(1, Math.floor((available.value - SECTION_HEADER) / CARD_HEIGHT))
    : 1,
);
const perPage = computed(() => rowsPerSection.value * columns.value);
const pages = reactive<Record<string, number>>({});
function pageCount(items: unknown[]): number {
  return Math.max(1, Math.ceil(items.length / perPage.value));
}
function pageItems<T>(id: string, items: T[]): T[] {
  const current = Math.min(pages[id] ?? 0, pageCount(items) - 1);
  return items.slice(current * perPage.value, (current + 1) * perPage.value);
}
function turn(id: string, items: unknown[], step: number): void {
  pages[id] = Math.min(
    pageCount(items) - 1,
    Math.max(0, (pages[id] ?? 0) + step),
  );
}
watch([filtered, () => store.query.value], () => {
  for (const key of Object.keys(pages)) pages[key] = 0;
});

const categories = computed(() =>
  (["all", "app", "game", "tool", "runtime"] as const).map((value) => ({
    value,
    label: t(`store.category.${value}`),
  })),
);
const categoryModel = computed({
  get: () => store.categoryFilter.value as string,
  set: (value) => {
    store.categoryFilter.value = value as PackageCategory | "all";
  },
});
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
    class="flex h-full items-center justify-center text-muted"
  >
    {{ t("store.loadFailed") }}
  </section>
  <div
    v-else-if="store.selected.value"
    class="flex h-full min-h-0 flex-col motion-safe:animate-rise"
  >
    <PackageDetail
      :item="store.selected.value"
      @install="store.install(store.selected.value.entry.id)"
      @cancel="store.cancelInstall(store.selected.value.entry.id)"
      @prepare="emit('prepare')"
      @close="emit('openPackage', null)"
      @dependency="emit('openPackage', $event)"
    />
  </div>
  <div
    v-else
    ref="page"
    class="flex h-full min-h-0 flex-col gap-3 motion-safe:animate-rise"
  >
    <header class="flex items-center gap-3">
      <div class="min-w-0 flex-1">
        <h1 class="text-xl font-semibold">{{ t("store.title") }}</h1>
        <p class="truncate text-xs text-muted">{{ t("store.subtitle") }}</p>
      </div>
      <StudioSegmented
        v-model="categoryModel"
        :options="categories"
        :label="t('studio.category')"
        size="sm"
      />
      <label class="flex items-center gap-1.5 text-xs text-muted"
        ><input
          v-model="store.compatibleOnly.value"
          type="checkbox"
          :disabled="!device"
        />{{ t("store.compatibleOnly") }}</label
      >
      <StudioButton
        size="sm"
        variant="ghost"
        :loading="store.loading.value"
        :aria-label="t('store.refresh')"
        :title="t('store.refresh')"
        @click="store.reload"
      >
        <IconPhArrowsClockwise
          v-if="!store.loading.value"
          width="14"
          height="14"
        />
      </StudioButton>
    </header>
    <StudioCallout v-if="issue || store.snapshot.value?.expired" tone="warning">
      {{ store.snapshot.value?.expired ? t("store.source.expired") : issueText
      }}<span v-if="store.snapshot.value?.verified" class="ml-2 text-muted">{{
        t("store.source.cachedContent")
      }}</span>
    </StudioCallout>
    <div
      v-if="showHero"
      class="relative shrink-0 overflow-hidden"
      :style="{ height: `${heroHeight}px` }"
      :aria-label="t('studio.featured')"
      @mouseenter="hovering = true"
      @mouseleave="hovering = false"
    >
      <div
        class="flex h-full transition-transform duration-500 ease-[cubic-bezier(0.2,0.8,0.2,1)] will-change-transform motion-reduce:transition-none"
        :style="{ transform: `translateX(calc(14% - ${feature * 72}%))` }"
      >
        <div
          v-for="(item, index) in featured"
          :key="item.entry.id"
          class="h-full w-[72%] shrink-0 px-1.5 transition-opacity duration-500"
          :class="index === feature ? 'opacity-100' : 'opacity-55'"
          :inert="index !== feature"
        >
          <StudioPanel
            :level="2"
            :padded="false"
            class="flex h-full items-center gap-5 px-6"
          >
            <PackageArtwork
              :entry="item.entry"
              :package-id="item.entry.id"
              :size="heroHeight - 40"
              class="rounded-[18%] shadow-raised"
            />
            <div class="min-w-0 flex-1">
              <span
                class="text-2xs font-semibold tracking-[0.12em] text-muted uppercase"
                >{{ t("store.fromCatalog") }}</span
              >
              <h2 class="mt-0.5 truncate text-2xl font-semibold tracking-tight">
                {{ packageText(item.entry, "name", locale, t) }}
              </h2>
              <p class="mt-0.5 line-clamp-1 text-sm text-muted">
                {{ packageText(item.entry, "summary", locale, t) }}
              </p>
              <StudioButton
                variant="primary"
                size="sm"
                class="mt-2"
                @click="emit('openPackage', item.entry.id)"
              >
                {{ t("studio.viewDetails")
                }}<IconPhArrowRight width="12" height="12" />
              </StudioButton>
            </div>
          </StudioPanel>
        </div>
      </div>
      <template v-if="featured.length > 1">
        <button
          class="absolute top-1/2 left-2 grid size-7 -translate-y-1/2 place-items-center rounded-full bg-raised text-muted shadow-control hover:text-ink"
          :aria-label="t('store.featuredPrevious')"
          @click="rotate(-1)"
        >
          <IconPhCaretLeft width="14" height="14" />
        </button>
        <button
          class="absolute top-1/2 right-2 grid size-7 -translate-y-1/2 place-items-center rounded-full bg-raised text-muted shadow-control hover:text-ink"
          :aria-label="t('store.featuredNext')"
          @click="rotate(1)"
        >
          <IconPhCaretRight width="14" height="14" />
        </button>
        <div class="absolute bottom-1.5 left-1/2 flex -translate-x-1/2 gap-1">
          <button
            v-for="(_, index) in featured"
            :key="index"
            class="p-1 after:block after:size-1.5 after:rounded-full after:bg-ink/20 aria-pressed:after:bg-signal"
            :aria-label="t('studio.featurePage', { index: index + 1 })"
            :aria-pressed="feature === index"
            @click="feature = index"
          />
        </div>
      </template>
    </div>
    <div class="flex min-h-0 flex-1 flex-col gap-3">
      <div
        v-if="store.loading.value && !store.catalog.value.length"
        class="flex flex-1 items-center justify-center gap-2 text-sm text-muted"
      >
        <IconSvgSpinnersRingResize width="16" height="16" />{{
          t("store.loading")
        }}
      </div>
      <div
        v-else-if="!store.packages.value.length"
        class="flex flex-1 flex-col items-center justify-center gap-2 text-center text-muted"
      >
        <IconPhSquaresFour width="28" height="28" />
        <p class="text-sm">{{ issue ? issueText : t("store.empty") }}</p>
        <p v-if="!issue && !filtered" class="text-xs">
          {{ t("store.emptyCatalog") }}
        </p>
        <StudioButton
          v-if="filtered"
          variant="link"
          @click="
            store.query.value = '';
            store.categoryFilter.value = 'all';
            store.compatibleOnly.value = false;
          "
        >
          {{ t("studio.clearFilters") }}
        </StudioButton>
      </div>
      <section
        v-for="section in sections"
        v-else
        :key="section.id"
        class="flex flex-col"
      >
        <header class="flex h-[30px] items-center gap-2">
          <h2 class="text-base font-semibold">
            {{
              section.id === "results"
                ? t("studio.searchResults", { count: section.items.length })
                : t(`store.collection.${section.id}`)
            }}
          </h2>
          <span
            v-if="section.id !== 'results'"
            class="text-xs text-muted tabular-nums"
            >{{ section.items.length }}</span
          >
          <StudioButton
            v-if="filtered"
            variant="link"
            size="sm"
            @click="
              store.query.value = '';
              store.categoryFilter.value = 'all';
              store.compatibleOnly.value = false;
            "
          >
            {{ t("studio.clearFilters") }}
          </StudioButton>
          <div
            v-if="pageCount(section.items) > 1"
            class="ml-auto flex items-center gap-1 text-xs text-muted"
          >
            <span class="tabular-nums">{{
              t("common.pageOf", {
                page: (pages[section.id] ?? 0) + 1,
                total: pageCount(section.items),
              })
            }}</span>
            <StudioButton
              size="sm"
              variant="ghost"
              :disabled="(pages[section.id] ?? 0) <= 0"
              :aria-label="t('common.previous')"
              @click="turn(section.id, section.items, -1)"
            >
              <IconPhCaretLeft width="13" height="13" />
            </StudioButton>
            <StudioButton
              size="sm"
              variant="ghost"
              :disabled="
                (pages[section.id] ?? 0) >= pageCount(section.items) - 1
              "
              :aria-label="t('common.next')"
              @click="turn(section.id, section.items, 1)"
            >
              <IconPhCaretRight width="13" height="13" />
            </StudioButton>
          </div>
        </header>
        <div
          class="grid justify-between"
          :style="{
            gridTemplateColumns: `repeat(${columns}, ${CARD_ART}px)`,
            rowGap: '14px',
          }"
        >
          <PackageCard
            v-for="item in pageItems(section.id, section.items)"
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
    <footer class="flex items-center gap-2 text-2xs text-muted">
      <IconPhPackage width="12" height="12" class="text-signal" />
      <span class="truncate">{{ sourceLabel }}</span>
      <IconPhShieldCheck
        v-if="store.snapshot.value?.verified"
        width="12"
        height="12"
        class="text-success"
      />
      <span class="truncate">{{
        store.snapshot.value?.verified
          ? t("store.source.verified")
          : store.snapshot.value?.source === "demo"
            ? t("store.source.demoNotice")
            : t("store.source.unconfigured")
      }}</span>
      <span v-if="store.snapshot.value?.checkedAt">{{
        t("store.source.checkedAt", {
          date: d(store.snapshot.value.checkedAt, "date"),
        })
      }}</span>
      <StudioButton
        variant="link"
        size="sm"
        class="ml-auto text-2xs"
        @click="emit('openEnvironment')"
      >
        {{ t("studio.environmentGuide")
        }}<IconPhCaretRight width="11" height="11" />
      </StudioButton>
    </footer>
  </div>
</template>
