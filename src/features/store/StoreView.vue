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
import { useOperations } from "../../shared/composables/useOperations";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioPanel from "../../shared/ui/StudioPanel.vue";
import StudioSelect from "../../shared/ui/StudioSelect.vue";
import PackageCard from "./PackageCard.vue";
import PackageDetail from "./PackageDetail.vue";
import PackageArtwork from "./PackageArtwork.vue";
import { packageText } from "./packageContent";
import {
  CARD_ART,
  CARD_GAP,
  CARD_HEIGHT,
  HERO_MAX,
  HERO_MIN,
  SECTION_HEADER,
} from "./storeLayout";
import { useStore, type PackageView } from "./useStore";

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
const { active } = useOperations();

const content = ref<HTMLElement | null>(null);
const { width, height } = useElementSize(content);
const GAP = 12;
const FOOTER = 18;

const featured = computed(() =>
  [...store.packages.value]
    .sort((a, b) => b.entry.publishedAt - a.entry.publishedAt)
    .slice(0, 3),
);
const recent = computed(() =>
  [...store.allPackages.value]
    .sort((a, b) => b.entry.publishedAt - a.entry.publishedAt)
    .slice(0, 4),
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

const expanded = ref<string | null>(null);
const filtered = computed(
  () =>
    !!store.query.value ||
    store.categoryFilter.value !== "all" ||
    store.compatibleOnly.value ||
    expanded.value !== null,
);
const collections = computed(() =>
  [
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
const rowHeight = CARD_HEIGHT + SECTION_HEADER;
// Hero first. Shrink it toward its minimum when that lets every collection
// row fit; otherwise size it proportionally and fall back to one combined row.
const heroHeight = computed(() => {
  const count = collections.value.length;
  const spare =
    height.value - FOOTER - GAP * 2 - count * rowHeight - (count - 1) * GAP;
  if (count && spare >= HERO_MIN) return Math.round(Math.min(HERO_MAX, spare));
  return Math.round(
    Math.min(HERO_MAX, Math.max(HERO_MIN, height.value * 0.27)),
  );
});
const rowsFit = computed(() =>
  Math.max(
    1,
    Math.floor(
      (height.value - heroHeight.value - FOOTER - GAP * 2 + GAP) /
        (rowHeight + GAP),
    ),
  ),
);
const sections = computed<Array<{ id: string; items: PackageView[] }>>(() => {
  if (expanded.value) {
    const section = collections.value.find(
      (item) => item.id === expanded.value,
    );
    return section ? [{ id: "results", items: section.items }] : [];
  }
  if (filtered.value) return [{ id: "results", items: store.packages.value }];
  if (rowsFit.value >= collections.value.length) return collections.value;
  return [{ id: "all", items: store.packages.value }];
});
const rowsPerSection = computed(() =>
  filtered.value
    ? Math.max(
        1,
        Math.floor(
          (height.value - FOOTER - GAP - SECTION_HEADER) / CARD_HEIGHT,
        ),
      )
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
function sectionTitle(id: string, count: number): string {
  if (id === "results") return t("studio.searchResults", { count });
  if (id === "all") return t("store.allApps");
  return t(`store.collection.${id}`);
}
function clearFilters(): void {
  store.query.value = "";
  store.categoryFilter.value = "all";
  store.compatibleOnly.value = false;
  expanded.value = null;
}
watch([filtered, () => store.query.value], () => {
  for (const key of Object.keys(pages)) pages[key] = 0;
});

const categories: Array<PackageCategory | "all"> = [
  "all",
  "app",
  "game",
  "tool",
  "runtime",
];
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
  <div v-else class="flex h-full min-h-0 gap-8 px-4 motion-safe:animate-rise">
    <div ref="content" class="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
      <StudioCallout
        v-if="issue || store.snapshot.value?.expired"
        tone="warning"
      >
        {{
          store.snapshot.value?.expired ? t("store.source.expired") : issueText
        }}<span v-if="store.snapshot.value?.verified" class="ml-2 text-muted">{{
          t("store.source.cachedContent")
        }}</span>
      </StudioCallout>
      <div
        v-if="!filtered && featured.length"
        class="relative shrink-0 overflow-hidden"
        :style="{ height: `${heroHeight}px` }"
        :aria-label="t('studio.featured')"
        @mouseenter="hovering = true"
        @mouseleave="hovering = false"
      >
        <div
          class="flex h-full transition-transform duration-500 ease-[cubic-bezier(0.2,0.8,0.2,1)] will-change-transform motion-reduce:transition-none"
          :style="{ transform: `translateX(calc(11% - ${feature * 78}%))` }"
        >
          <div
            v-for="(item, index) in featured"
            :key="item.entry.id"
            class="h-full w-[78%] shrink-0 px-1.5 transition-opacity duration-500"
            :class="index === feature ? 'opacity-100' : 'opacity-55'"
            :inert="index !== feature"
          >
            <StudioPanel
              :level="2"
              :padded="false"
              class="relative flex h-full items-center gap-5 overflow-hidden px-6"
            >
              <PackageArtwork
                :entry="item.entry"
                :package-id="item.entry.id"
                :size="heroHeight - 40"
                class="rounded-[22%] shadow-raised"
              />
              <div class="min-w-0 flex-1">
                <span
                  v-if="heroHeight >= 130"
                  class="text-2xs font-semibold tracking-[0.12em] text-muted uppercase"
                  >{{ t("store.fromCatalog") }}</span
                >
                <h2
                  class="truncate font-semibold tracking-tight"
                  :class="heroHeight >= 130 ? 'text-2xl' : 'text-xl'"
                >
                  {{ packageText(item.entry, "name", locale, t) }}
                </h2>
                <p
                  class="text-sm text-muted"
                  :class="heroHeight >= 150 ? 'line-clamp-2' : 'truncate'"
                >
                  {{ packageText(item.entry, "summary", locale, t) }}
                </p>
              </div>
              <StudioButton
                variant="primary"
                size="sm"
                class="mr-6 shrink-0 rounded-full px-4"
                @click="emit('openPackage', item.entry.id)"
              >
                {{ t("studio.viewDetails") }}
              </StudioButton>
            </StudioPanel>
          </div>
        </div>
        <template v-if="featured.length > 1">
          <button
            class="absolute top-1/2 left-2 grid size-8 -translate-y-1/2 place-items-center rounded-full bg-raised/90 text-ink shadow-raised backdrop-blur hover:bg-raised"
            :aria-label="t('store.featuredPrevious')"
            @click="rotate(-1)"
          >
            <IconPhCaretLeft width="16" height="16" />
          </button>
          <button
            class="absolute top-1/2 right-2 grid size-8 -translate-y-1/2 place-items-center rounded-full bg-raised/90 text-ink shadow-raised backdrop-blur hover:bg-raised"
            :aria-label="t('store.featuredNext')"
            @click="rotate(1)"
          >
            <IconPhCaretRight width="16" height="16" />
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
      <div class="flex min-h-0 flex-1 flex-col gap-3 overflow-hidden">
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
          <StudioButton v-if="filtered" variant="link" @click="clearFilters">
            {{ t("studio.clearFilters") }}
          </StudioButton>
        </div>
        <section
          v-for="section in sections"
          v-else
          :key="section.id"
          class="flex flex-col"
        >
          <header class="flex h-7 items-center gap-2">
            <h2 class="text-base font-semibold">
              {{ sectionTitle(section.id, section.items.length) }}
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
              @click="clearFilters"
            >
              {{ expanded ? t("common.back") : t("studio.clearFilters") }}
            </StudioButton>
            <div class="ml-auto flex items-center gap-1 text-xs text-muted">
              <template v-if="pageCount(section.items) > 1">
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
              </template>
              <StudioButton
                v-if="section.id !== 'results' && section.id !== 'all'"
                variant="link"
                size="sm"
                @click="expanded = section.id"
              >
                {{ t("store.seeAll")
                }}<IconPhCaretRight width="11" height="11" />
              </StudioButton>
            </div>
          </header>
          <div
            class="grid justify-between"
            :style="{
              gridTemplateColumns: `repeat(${columns}, ${CARD_ART}px)`,
              rowGap: '12px',
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
      </footer>
    </div>
    <aside class="flex w-[196px] shrink-0 flex-col gap-2 text-sm">
      <h2 class="truncate text-xl font-semibold tracking-tight">
        {{ device?.marketingName ?? t("store.title") }}
      </h2>
      <StudioSelect
        v-model="categoryModel"
        size="sm"
        class="w-full"
        :label="t('studio.category')"
      >
        <option
          v-for="category in categories"
          :key="category"
          :value="category"
        >
          {{ t(`store.category.${category}`) }}
        </option>
      </StudioSelect>
      <label class="flex items-center gap-1.5 text-xs text-muted"
        ><input
          v-model="store.compatibleOnly.value"
          type="checkbox"
          :disabled="!device"
        />{{ t("store.compatibleOnly") }}</label
      >
      <p
        class="mt-2 text-2xs font-semibold tracking-[0.08em] text-muted uppercase"
      >
        {{ t("studio.quickLinks") }}
      </p>
      <button
        class="flex items-center justify-between rounded-control px-1.5 py-1 text-left hover:bg-ink/6"
        @click="emit('openInstalled')"
      >
        {{ t("studio.installedApps")
        }}<span class="text-xs text-muted tabular-nums">{{
          gateway.capabilities.installed ? store.installed.value.length : "—"
        }}</span>
      </button>
      <button
        class="flex items-center justify-between rounded-control px-1.5 py-1 text-left hover:bg-ink/6"
        @click="emit('openInstalled')"
      >
        {{ t("store.installQueue")
        }}<span class="text-xs text-muted tabular-nums">{{
          active.length + store.queuedIds.value.length
        }}</span>
      </button>
      <button
        class="flex items-center justify-between rounded-control px-1.5 py-1 text-left hover:bg-ink/6"
        @click="emit('openEnvironment')"
      >
        {{ t("studio.environmentGuide") }}
      </button>
      <button
        class="flex items-center justify-between rounded-control px-1.5 py-1 text-left hover:bg-ink/6 disabled:opacity-45"
        :disabled="store.loading.value"
        @click="store.reload"
      >
        {{ t("store.reloadCatalog")
        }}<IconSvgSpinners90Ring
          v-if="store.loading.value"
          width="12"
          height="12"
        />
      </button>
      <p
        class="mt-2 text-2xs font-semibold tracking-[0.08em] text-muted uppercase"
      >
        {{ t("store.recentlyPublished") }}
      </p>
      <ol class="flex flex-col gap-1">
        <li v-for="(item, index) in recent" :key="item.entry.id">
          <button
            class="flex w-full items-center gap-2 rounded-control px-1.5 py-1 text-left hover:bg-ink/6"
            @click="emit('openPackage', item.entry.id)"
          >
            <span class="w-3 text-xs text-muted tabular-nums">{{
              index + 1
            }}</span>
            <PackageArtwork
              :entry="item.entry"
              :package-id="item.entry.id"
              :size="26"
              class="rounded-[22%]"
            />
            <span class="min-w-0">
              <span class="block truncate text-xs font-medium">{{
                packageText(item.entry, "name", locale, t)
              }}</span>
              <span class="block truncate text-2xs text-muted">{{
                t(`store.category.${item.entry.category}`)
              }}</span>
            </span>
          </button>
        </li>
      </ol>
    </aside>
  </div>
</template>
