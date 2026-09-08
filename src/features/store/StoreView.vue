<script setup lang="ts">
import IconPhDeviceMobile from "~icons/ph/device-mobile";
import IconPhGameController from "~icons/ph/game-controller";
import IconPhWrench from "~icons/ph/wrench";
import IconPhLifebuoy from "~icons/ph/lifebuoy";
import IconPhSquaresFour from "~icons/ph/squares-four";
import {
  computed,
  nextTick,
  onBeforeUnmount,
  onMounted,
  reactive,
  ref,
  watch,
  type Component,
} from "vue";
import { useI18n } from "vue-i18n";
import {
  useGateway,
  type PackageCategory,
  type Platform,
} from "../../shared/gateway";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { useElementSize } from "../../shared/composables/useElementSize";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioSegmented from "../../shared/ui/StudioSegmented.vue";
import PackageCard from "./PackageCard.vue";
import PackageDetail from "./PackageDetail.vue";
import PackageArtwork from "./PackageArtwork.vue";
import { packageText } from "./packageContent";
import {
  artworkSize,
  CARD_CHROME,
  CARD_GAP,
  CARD_PAD,
  HERO_MAX,
  HERO_MIN,
  SECTION_HEADER,
  TILE_ROW,
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

const page = ref<HTMLElement | null>(null);
const content = ref<HTMLElement | null>(null);
const { height } = useElementSize(page);
const { width } = useElementSize(content);
const GAP = 12;
const art = computed(() => artworkSize(height.value));
const cardHeight = computed(() => art.value + CARD_CHROME);
const rowHeight = computed(() => cardHeight.value + SECTION_HEADER);
const columns = computed(() =>
  Math.max(
    3,
    Math.floor((width.value + CARD_GAP) / (art.value + CARD_PAD + CARD_GAP)),
  ),
);

// Featured banners: the newest listings, in an iTunes style peek carousel
// that wraps around.
const featured = computed(() =>
  [...store.packages.value]
    .sort((a, b) => b.entry.publishedAt - a.entry.publishedAt)
    .slice(0, 3),
);
const feature = ref(0);
const hovering = ref(false);
const snapping = ref(new Set<string>());
let rotation: number | undefined;
function offsetOf(index: number, current: number, count: number): number {
  const offset = (index - current + count) % count;
  return offset > Math.floor(count / 2) ? offset - count : offset;
}
const slots = computed(() =>
  featured.value.map((item, index) => ({
    item,
    offset: offsetOf(index, feature.value, featured.value.length),
  })),
);
async function rotate(step: number): Promise<void> {
  const count = featured.value.length;
  if (count < 2) return;
  const next = (feature.value + step + count) % count;
  // A banner that crosses from one edge to the other snaps instead of
  // sliding through the middle.
  const jumping = new Set(
    featured.value
      .filter(
        (_, index) =>
          Math.abs(
            offsetOf(index, next, count) -
              offsetOf(index, feature.value, count),
          ) > 1,
      )
      .map((item) => item.entry.id),
  );
  snapping.value = jumping;
  feature.value = next;
  await nextTick();
  requestAnimationFrame(() => {
    snapping.value = new Set();
  });
}
onMounted(() => {
  rotation = window.setInterval(() => {
    if (!hovering.value) void rotate(1);
  }, 7000);
});
onBeforeUnmount(() => window.clearInterval(rotation));
watch(
  () => featured.value.length,
  (count) => {
    if (feature.value >= count) feature.value = 0;
  },
);

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
// Hero first. Shrink it toward its minimum when that lets every collection
// row fit; otherwise size it proportionally and fall back to one combined row.
const shelvesHeight = computed(() => {
  const count = collections.value.length;
  return count * rowHeight.value + (count - 1) * GAP;
});
const heroHeight = computed(() => {
  const spare = height.value - GAP - shelvesHeight.value;
  if (collections.value.length && spare >= HERO_MIN)
    return Math.round(Math.min(HERO_MAX, spare));
  return Math.round(
    Math.min(HERO_MAX, Math.max(HERO_MIN, height.value * 0.27)),
  );
});
const rowsFit = computed(() =>
  Math.max(
    1,
    Math.floor(
      (height.value - heroHeight.value - GAP + GAP) / (rowHeight.value + GAP),
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
// iTunes style collection tiles fill the band left over on tall windows.
const tilesVisible = computed(
  () =>
    !filtered.value &&
    sections.value === collections.value &&
    height.value - heroHeight.value - shelvesHeight.value - GAP * 2 >= TILE_ROW,
);
const rowsPerSection = computed(() =>
  filtered.value
    ? Math.max(
        1,
        Math.floor(
          (height.value - SECTION_HEADER + GAP) / (cardHeight.value + GAP),
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

interface Tile {
  id: string;
  title: string;
  caption: string;
  gradient: string;
  icon: Component;
  open: () => void;
}
const tiles = computed<Tile[]>(() => [
  {
    id: "madeFor",
    title: device.value
      ? t("store.tiles.madeFor", { name: device.value.marketingName })
      : t("store.tiles.madeForAll"),
    caption: t("store.tiles.compatible"),
    gradient: "from-[#2f6fd6] to-[#5a3fd8]",
    icon: IconPhDeviceMobile,
    open: () => {
      if (device.value) store.compatibleOnly.value = true;
      else expanded.value = "apps";
    },
  },
  {
    id: "games",
    title: t("store.category.game"),
    caption: t("store.tiles.browse"),
    gradient: "from-[#c2417a] to-[#8b2fb8]",
    icon: IconPhGameController,
    open: () => {
      store.categoryFilter.value = "game";
    },
  },
  {
    id: "tools",
    title: t("store.collection.tools"),
    caption: t("store.tiles.browse"),
    gradient: "from-[#2e8b6f] to-[#1f6f9a]",
    icon: IconPhWrench,
    open: () => {
      expanded.value = "tools";
    },
  },
  {
    id: "guide",
    title: t("studio.environmentGuide"),
    caption: t("store.tiles.help"),
    gradient: "from-[#d67a2f] to-[#c0392b]",
    icon: IconPhLifebuoy,
    open: () => emit("openEnvironment"),
  },
  {
    id: "installed",
    title: t("studio.installedApps"),
    caption: t("store.tiles.device"),
    gradient: "from-[#3b3f4a] to-[#1d1f26]",
    icon: IconPhSquaresFour,
    open: () => emit("openInstalled"),
  },
]);
const tileCount = computed(() =>
  Math.max(
    2,
    Math.min(tiles.value.length, Math.floor((width.value + 12) / 184)),
  ),
);

// iTunes' iPhone/iPad switch: one storefront per platform.
const platforms: Platform[] = ["ios", "3ds"];
const platformOptions = computed(() =>
  platforms.map((value) => ({ value, label: t(`store.platform.${value}`) })),
);
const platformModel = computed({
  get: () => store.platformFilter.value as string,
  set: (value) => {
    store.platformFilter.value = value as Platform;
  },
});
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
      @dependency="emit('openPackage', $event)"
    />
  </div>
  <div
    v-else
    ref="page"
    class="flex h-full min-h-0 flex-col gap-3 px-4 motion-safe:animate-rise"
  >
    <StudioCallout v-if="issue || store.snapshot.value?.expired" tone="warning">
      {{ store.snapshot.value?.expired ? t("store.source.expired") : issueText
      }}<span v-if="store.snapshot.value?.verified" class="ml-2 text-muted">{{
        t("store.source.cachedContent")
      }}</span>
    </StudioCallout>
    <div
      v-if="!filtered && featured.length"
      class="relative shrink-0 overflow-clip"
      :style="{ height: `${heroHeight}px` }"
      :aria-label="t('studio.featured')"
      @mouseenter="hovering = true"
      @mouseleave="hovering = false"
    >
      <div
        v-for="slot in slots"
        :key="slot.item.entry.id"
        class="absolute inset-y-0 left-1/2 w-[76%] transition-[transform,opacity] duration-500 ease-[cubic-bezier(0.2,0.8,0.2,1)] will-change-transform motion-reduce:transition-none"
        :class="[
          slot.offset === 0 ? 'z-10 opacity-100' : 'opacity-60',
          snapping.has(slot.item.entry.id) && 'transition-none',
        ]"
        :style="{
          transform: `translateX(calc(-50% + ${slot.offset * 102}%))`,
        }"
        :inert="slot.offset !== 0"
      >
        <div
          class="relative h-full overflow-clip rounded-panel bg-ink text-white shadow-raised"
        >
          <!-- The artwork, blurred at three scales, paints the poster in the
                 app's own colours: a wash over the whole banner plus two
                 brighter pools. -->
          <PackageArtwork
            :entry="slot.item.entry"
            :package-id="slot.item.entry.id"
            :size="heroHeight"
            class="pointer-events-none absolute inset-0 m-auto scale-[7] blur-3xl saturate-150"
            aria-hidden="true"
          />
          <PackageArtwork
            :entry="slot.item.entry"
            :package-id="slot.item.entry.id"
            :size="heroHeight"
            class="pointer-events-none absolute top-[-30%] left-[-6%] scale-[2.4] opacity-80 blur-2xl saturate-200"
            aria-hidden="true"
          />
          <PackageArtwork
            :entry="slot.item.entry"
            :package-id="slot.item.entry.id"
            :size="heroHeight"
            class="pointer-events-none absolute top-[10%] right-[-4%] scale-[2] opacity-60 blur-2xl brightness-125 saturate-200"
            aria-hidden="true"
          />
          <div
            class="absolute inset-0 bg-gradient-to-r from-black/45 via-black/25 to-black/10 dark:from-black/55 dark:via-black/35 dark:to-black/20"
          />
          <div class="relative flex h-full items-center gap-5 px-6">
            <PackageArtwork
              :entry="slot.item.entry"
              :package-id="slot.item.entry.id"
              :size="heroHeight - 44"
              class="rounded-[22%] shadow-raised"
            />
            <div class="min-w-0 flex-1">
              <span
                v-if="heroHeight >= 130"
                class="text-2xs font-semibold tracking-[0.14em] text-white/70 uppercase"
                >{{ t("studio.featured") }}</span
              >
              <h2
                class="truncate font-bold tracking-tight"
                :class="heroHeight >= 130 ? 'text-2xl' : 'text-xl'"
              >
                {{ packageText(slot.item.entry, "name", locale, t) }}
              </h2>
              <p
                class="text-sm text-white/80"
                :class="heroHeight >= 150 ? 'line-clamp-2' : 'truncate'"
              >
                {{ packageText(slot.item.entry, "summary", locale, t) }}
              </p>
            </div>
            <button
              class="mr-6 h-7 shrink-0 rounded-full bg-white/90 px-4 text-xs font-semibold text-[#1d1d1f] shadow-control transition-[background-color,opacity] duration-300 hover:bg-white"
              :class="slot.offset !== 0 && 'opacity-0'"
              @click="emit('openPackage', slot.item.entry.id)"
            >
              {{ t("studio.viewDetails") }}
            </button>
          </div>
        </div>
      </div>
      <template v-if="featured.length > 1">
        <button
          class="absolute top-1/2 left-2 z-20 grid size-8 -translate-y-1/2 place-items-center rounded-full bg-raised/90 text-ink shadow-raised backdrop-blur hover:bg-raised"
          :aria-label="t('store.featuredPrevious')"
          @click="rotate(-1)"
        >
          <IconPhCaretLeft width="16" height="16" />
        </button>
        <button
          class="absolute top-1/2 right-2 z-20 grid size-8 -translate-y-1/2 place-items-center rounded-full bg-raised/90 text-ink shadow-raised backdrop-blur hover:bg-raised"
          :aria-label="t('store.featuredNext')"
          @click="rotate(1)"
        >
          <IconPhCaretRight width="16" height="16" />
        </button>
        <div
          class="absolute bottom-1 left-1/2 z-20 flex -translate-x-1/2 gap-0.5"
        >
          <button
            v-for="(_, index) in featured"
            :key="index"
            class="p-1 after:block after:size-1.5 after:rounded-full after:bg-white/45 after:transition-colors aria-pressed:after:bg-white"
            :aria-label="t('studio.featurePage', { index: index + 1 })"
            :aria-pressed="feature === index"
            @click="rotate(index - feature)"
          />
        </div>
      </template>
    </div>
    <div class="flex min-h-0 flex-1 gap-8">
      <div ref="content" class="flex min-h-0 min-w-0 flex-1 flex-col gap-3">
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
              gridTemplateColumns: `repeat(${columns}, ${art + CARD_PAD}px)`,
              rowGap: `${GAP}px`,
            }"
          >
            <PackageCard
              v-for="item in pageItems(section.id, section.items)"
              :key="item.entry.id"
              :item="item"
              :size="art"
              @select="emit('openPackage', item.entry.id)"
              @install="
                item.missingDependencies.length
                  ? emit('openPackage', item.entry.id)
                  : store.install(item.entry.id)
              "
            />
          </div>
        </section>
        <div
          v-if="tilesVisible && sections.length"
          class="grid gap-3"
          :style="{
            gridTemplateColumns: `repeat(${tileCount}, minmax(0, 1fr))`,
            height: `${TILE_ROW}px`,
          }"
        >
          <button
            v-for="tile in tiles.slice(0, tileCount)"
            :key="tile.id"
            class="group/tile relative flex flex-col justify-end overflow-clip rounded-panel bg-gradient-to-br p-3 text-left text-white shadow-panel transition-[transform,box-shadow] duration-200 hover:-translate-y-0.5 hover:shadow-raised"
            :class="tile.gradient"
            @click="tile.open()"
          >
            <component
              :is="tile.icon"
              width="56"
              height="56"
              class="absolute -top-2 -right-2 text-white/15 transition-transform duration-300 group-hover/tile:scale-110"
            />
            <span
              class="text-2xs font-semibold tracking-[0.12em] text-white/70 uppercase"
              >{{ tile.caption }}</span
            >
            <span class="truncate text-base font-bold">{{ tile.title }}</span>
          </button>
        </div>
      </div>
      <aside class="flex w-[196px] shrink-0 flex-col text-sm">
        <h2 class="truncate text-xl font-semibold tracking-tight">
          {{ device?.marketingName ?? t("store.title") }}
        </h2>
        <StudioSegmented
          v-model="platformModel"
          :options="platformOptions"
          :label="t('store.detail.compatibility')"
          size="sm"
          class="mt-2 w-full [&>button]:flex-1"
        />
        <p
          class="mt-3 mb-1 text-2xs font-semibold tracking-[0.08em] text-muted uppercase"
        >
          {{ t("studio.category") }}
        </p>
        <nav class="flex flex-col gap-px" :aria-label="t('studio.category')">
          <button
            v-for="category in categories"
            :key="category"
            class="flex w-full items-center rounded-control px-2 py-[5px] text-left text-base text-ink transition-colors hover:bg-ink/6 aria-[current=true]:bg-signal aria-[current=true]:text-on-signal aria-[current=true]:hover:bg-signal"
            :aria-current="store.categoryFilter.value === category"
            @click="store.categoryFilter.value = category"
          >
            {{ t(`store.category.${category}`) }}
          </button>
        </nav>
        <label class="mt-3 flex items-center gap-1.5 px-2 text-xs text-muted"
          ><input
            v-model="store.compatibleOnly.value"
            type="checkbox"
            :disabled="!device"
          />{{ t("store.compatibleOnly") }}</label
        >
        <footer class="mt-auto flex flex-col gap-1 text-2xs text-muted">
          <span class="flex items-center gap-1.5">
            <IconPhPackage width="12" height="12" class="text-signal" />
            <span class="truncate">{{ sourceLabel }}</span>
            <button
              class="ml-auto grid size-5 place-items-center rounded-control hover:bg-ink/6 hover:text-ink disabled:opacity-45"
              :disabled="store.loading.value"
              :aria-label="t('store.reloadCatalog')"
              :title="t('store.reloadCatalog')"
              @click="store.reload"
            >
              <IconSvgSpinners90Ring
                v-if="store.loading.value"
                width="12"
                height="12"
              />
              <IconPhArrowsClockwise v-else width="12" height="12" />
            </button>
          </span>
          <span class="flex items-center gap-1.5">
            <IconPhShieldCheck
              v-if="store.snapshot.value?.verified"
              width="12"
              height="12"
              class="text-success"
            />
            <span class="line-clamp-2">{{
              store.snapshot.value?.verified
                ? t("store.source.verified")
                : store.snapshot.value?.source === "demo"
                  ? t("store.source.demoNotice")
                  : t("store.source.unconfigured")
            }}</span>
          </span>
          <span v-if="store.snapshot.value?.checkedAt">{{
            t("store.source.checkedAt", {
              date: d(store.snapshot.value.checkedAt, "date"),
            })
          }}</span>
        </footer>
      </aside>
    </div>
  </div>
</template>
