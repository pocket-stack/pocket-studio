<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { useGateway } from "../../shared/gateway";
import { useElementSize } from "../../shared/composables/useElementSize";
import {
  operationProgress,
  operationStepCeiling,
  useOperations,
} from "../../shared/composables/useOperations";
import ProgressBar from "../../shared/ui/ProgressBar.vue";
import StatusPill from "../../shared/ui/StatusPill.vue";
import StepList from "../../shared/ui/StepList.vue";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioSegmented from "../../shared/ui/StudioSegmented.vue";
import PackageArtwork from "./PackageArtwork.vue";
import StoreMedia from "./StoreMedia.vue";
import { formatBytes, installationForms } from "./compatibility";
import { useDeviceConnect } from "../device/useDeviceConnect";
import { packageText, packageMedia } from "./packageContent";
import type { PackageView } from "./useStore";

const SCREENSHOT_RATIO = 194 / 290;
const RELEASE_ROW = 58;
const DESCRIPTION_LINE = 24;
const SECTION_TITLE = 30;

const props = defineProps<{ item: PackageView }>();
const emit = defineEmits<{
  install: [];
  cancel: [];
  prepare: [];
  dependency: [id: string];
}>();
const { t, d, locale, te } = useI18n();
const gateway = useGateway();
const { device } = useDeviceSession();
const { active } = useOperations();
const name = computed(() =>
  packageText(props.item.entry, "name", locale.value, t),
);
const description = computed(() =>
  packageText(props.item.entry, "description", locale.value, t),
);
const summary = computed(
  () =>
    packageText(props.item.entry, "summary", locale.value, t) ||
    t("store.summaryUnavailable"),
);
const screenshots = computed(() =>
  packageMedia(props.item.entry, "screenshot", locale.value),
);
const history = computed(() =>
  [...(props.item.entry.details?.history ?? [])].sort(
    (a, b) => b.published_at - a.published_at || b.revision - a.revision,
  ),
);
const installing = computed(() => props.item.operation?.status === "running");
const currentStep = computed(() =>
  props.item.operation?.steps.find(
    (step) => step.id === props.item.operation?.currentStepId,
  ),
);
const canInstall = computed(
  () =>
    gateway.capabilities.packages &&
    props.item.verdict === "compatible" &&
    !installing.value &&
    !props.item.queuePosition &&
    !props.item.missingDependencies.length &&
    !active.value.some((op) => op.kind === "preparation"),
);
// A 3DS title with several installation forms always offers "install": the
// form is chosen in a dialog and updates happen from the Installed page.
const installLabel = computed(() =>
  device.value?.platform === "3ds"
    ? props.item.installed && installationForms(props.item.entry).length <= 1
      ? props.item.installed.releaseId === props.item.entry.details?.releaseId
        ? t("store.detail.reinstall")
        : t("store.detail.update")
      : t("store.detail.install")
    : props.item.installed
      ? (
          gateway.flavor === "tauri"
            ? !props.item.installed.revision ||
              props.item.installed.artifactId ===
                props.item.entry.details?.artifactId
            : props.item.installed.version === props.item.entry.version
        )
        ? t("store.detail.reinstall")
        : t("store.detail.update")
      : t("store.detail.install"),
);
// A standalone 3DS title can be copied to the card whenever the launcher is
// not there to install it: no console, another platform, or no launcher host.
const cardFormats = computed(() =>
  installationForms(props.item.entry)
    .filter((form) => form.delivery === "bundled")
    .map((form) => form.format),
);
const canCopyToCard = computed(
  () =>
    props.item.entry.compatibility.platform === "3ds" &&
    cardFormats.value.length > 0 &&
    (device.value?.platform !== "3ds" ||
      !device.value.threeDs?.launcher ||
      props.item.verdict !== "compatible"),
);
function copyToCard(): void {
  useDeviceConnect().show("card", {
    target: {
      appId: props.item.entry.id,
      name: name.value,
      formats: cardFormats.value,
    },
  });
}
const sourceUrl = computed(() => {
  const source = props.item.entry.details?.app.source.repository;
  if (!source) return null;
  try {
    const url = new URL(source);
    return ["https:", "http:"].includes(url.protocol) ? url.href : null;
  } catch {
    return null;
  }
});
const errorReason = computed(
  () =>
    props.item.operation?.error?.diagnostic?.reason ??
    props.item.operation?.error?.code,
);
const errorTitle = computed(() => {
  const specific = `store.errors.${errorReason.value}.title`;
  return te(specific)
    ? t(specific)
    : t(`store.errors.${props.item.operation?.error?.code}.title`);
});
const errorBody = computed(() => {
  const specific = `store.actions.errors.${errorReason.value}`;
  return te(specific)
    ? t(specific)
    : t(`store.errors.${props.item.operation?.error?.code}.body`);
});
const official = computed(
  () =>
    props.item.entry.details?.app.publisher.verified ??
    props.item.entry.developer === "PocketJS",
);
const platformLabel = computed(() =>
  t(`store.platform.${props.item.entry.compatibility.platform}`),
);
// App Store style strip: a label above a large value, with a small detail line.
const stats = computed(() => [
  {
    label: t("store.detail.developer"),
    value: props.item.entry.developer,
    sub: official.value ? t("studio.official") : t("studio.community"),
  },
  {
    label: t("store.detail.version"),
    value: props.item.entry.version,
    sub: props.item.entry.details
      ? t("store.detail.revision", {
          revision: props.item.entry.details.revision,
        })
      : d(props.item.entry.publishedAt, "date"),
  },
  {
    label: t("store.detail.size"),
    value: formatBytes(props.item.entry.sizeBytes),
    sub: t(`store.policy.${props.item.entry.installPolicy}`),
  },
  {
    label: t("store.detail.compatibility"),
    value: `${platformLabel.value} ${props.item.entry.compatibility.minOsVersion}+`,
    sub: props.item.entry.compatibility.maxOsVersion
      ? `${platformLabel.value} ${props.item.entry.compatibility.minOsVersion} – ${props.item.entry.compatibility.maxOsVersion}`
      : `${platformLabel.value} ${props.item.entry.compatibility.minOsVersion}+`,
  },
  {
    label: t("store.detail.models"),
    value: String(props.item.entry.compatibility.models.length),
    sub: props.item.entry.compatibility.models.join(", "),
  },
  {
    label: t(
      `store.detail.environment.${props.item.entry.compatibility.platform}`,
    ),
    value: t(
      props.item.entry.compatibility.requiresJailbreak
        ? "common.required"
        : "common.notRequired",
    ),
    sub: t(`store.category.${props.item.entry.category}`),
  },
]);
// The App Store "Information" grid: everything the strip only hints at.
const information = computed(() => [
  {
    label: t("store.detail.category"),
    value: t(`store.category.${props.item.entry.category}`),
  },
  {
    label: t("store.detail.policy"),
    value: t(`store.policy.${props.item.entry.installPolicy}`),
  },
  {
    label: t("store.detail.published"),
    value: d(props.item.entry.publishedAt, "date"),
  },
  {
    label: t("store.detail.osRange"),
    value: `${props.item.entry.compatibility.minOsVersion} – ${props.item.entry.compatibility.maxOsVersion}`,
  },
  {
    label: t("store.detail.models"),
    value: props.item.entry.compatibility.models.join(", "),
  },
  {
    label: t("store.detail.dependencies"),
    value: props.item.entry.dependencies.length
      ? props.item.entry.dependencies
          .map((id) => t(`catalog.${id}.name`))
          .join(", ")
      : t("store.detail.none"),
  },
  {
    label: t("store.detail.catalogState"),
    value: props.item.entry.details
      ? t("store.detail.catalogVerified")
      : t(
          props.item.entry.signed
            ? "store.detail.signed"
            : "store.detail.unsigned",
        ),
  },
  {
    label: t("store.detail.checksum"),
    value: props.item.entry.checksumSha256,
    mono: true,
  },
]);

const tabs = computed(() => [
  { value: "description", label: t("store.tabs.description") },
  ...(screenshots.value.length
    ? [{ value: "previews", label: t("store.tabs.previews") }]
    : []),
  { value: "history", label: t("store.tabs.history") },
  ...(props.item.operation
    ? [{ value: "steps", label: t("store.tabs.steps") }]
    : []),
]);
const tab = ref("description");
watch(
  () => props.item.entry.id,
  () => {
    tab.value = "description";
    previewPage.value = 0;
    historyPage.value = 0;
  },
);
const body = ref<HTMLElement | null>(null);
const { width: bodyWidth, height: bodyHeight } = useElementSize(body);
const descriptionLines = computed(() =>
  Math.max(
    2,
    Math.floor((bodyHeight.value - SECTION_TITLE) / DESCRIPTION_LINE),
  ),
);
const shotHeight = computed(() => Math.max(120, bodyHeight.value));
const shotsPerPage = computed(() =>
  Math.max(
    1,
    Math.floor(
      (bodyWidth.value + 12) / (shotHeight.value * SCREENSHOT_RATIO + 12),
    ),
  ),
);
const previewPage = ref(0);
const previewPages = computed(() =>
  Math.max(1, Math.ceil(screenshots.value.length / shotsPerPage.value)),
);
const pageShots = computed(() =>
  screenshots.value.slice(
    previewPage.value * shotsPerPage.value,
    (previewPage.value + 1) * shotsPerPage.value,
  ),
);
const releasesPerPage = computed(() =>
  Math.max(1, Math.floor(bodyHeight.value / RELEASE_ROW)),
);
const historyPage = ref(0);
const historyPages = computed(() =>
  Math.max(1, Math.ceil(history.value.length / releasesPerPage.value)),
);
const pageReleases = computed(() =>
  history.value.slice(
    historyPage.value * releasesPerPage.value,
    (historyPage.value + 1) * releasesPerPage.value,
  ),
);
</script>

<template>
  <article class="flex h-full min-h-0 flex-col">
    <header class="flex items-start gap-6">
      <PackageArtwork
        :entry="item.entry"
        :package-id="item.entry.id"
        :size="120"
        class="rounded-[22%] shadow-raised"
      />
      <div class="min-w-0 flex-1">
        <h1 class="truncate text-3xl font-bold tracking-tight">
          {{ name }}
        </h1>
        <p class="truncate text-lg text-muted">{{ item.entry.developer }}</p>
        <p class="mt-0.5 truncate text-sm text-muted">{{ summary }}</p>
        <div class="mt-3.5 flex flex-wrap items-center gap-3">
          <StudioButton
            variant="primary"
            class="h-8 rounded-full px-6 text-sm font-bold"
            :disabled="!canInstall"
            :loading="item.pending"
            @click="emit('install')"
          >
            {{ installing ? t("store.state.installing") : installLabel }}
          </StudioButton>
          <StudioButton
            v-if="item.verdict === 'requiresPreparation'"
            class="rounded-full"
            @click="emit('prepare')"
          >
            {{ t("store.detail.prepareDevice") }}
          </StudioButton>
          <StudioButton
            v-if="canCopyToCard"
            class="rounded-full"
            @click="copyToCard"
          >
            {{ t("threeDs.copyToCard") }}
          </StudioButton>
          <span
            class="text-xs"
            :class="
              item.verdict === 'compatible' ? 'text-success' : 'text-warning'
            "
            >{{ t(`store.verdict.${item.verdict}`) }}</span
          >
        </div>
        <p
          v-if="item.missingDependencies.length"
          class="mt-2 flex flex-wrap gap-2 text-xs"
        >
          <span>{{ t("store.detail.installDependenciesFirst") }}</span
          ><StudioButton
            v-for="id in item.missingDependencies"
            :key="id"
            variant="link"
            size="sm"
            @click="emit('dependency', id)"
          >
            {{ t(`catalog.${id}.name`) }}
          </StudioButton>
        </p>
        <p v-if="item.queuePosition" class="mt-2 text-xs text-muted">
          {{ t("studio.queued", { position: item.queuePosition })
          }}<StudioButton
            variant="link"
            size="sm"
            class="ml-2"
            @click="emit('cancel')"
          >
            {{ t("common.cancel") }}
          </StudioButton>
        </p>
      </div>
    </header>
    <div class="mt-4 h-px shrink-0 bg-ink/8" />
    <dl class="grid shrink-0 grid-cols-6 py-2.5">
      <div
        v-for="(stat, index) in stats"
        :key="stat.label"
        class="relative min-w-0 px-3 text-center"
        :title="stat.sub"
      >
        <span
          v-if="index"
          class="absolute inset-y-1.5 left-0 w-px bg-ink/10"
          aria-hidden="true"
        />
        <dt
          class="truncate text-2xs font-semibold tracking-[0.06em] text-muted uppercase"
        >
          {{ stat.label }}
        </dt>
        <dd class="mt-0.5 truncate text-lg leading-6 font-semibold">
          {{ stat.value }}
        </dd>
        <dd class="truncate text-2xs text-muted">{{ stat.sub }}</dd>
      </div>
    </dl>
    <div class="h-px shrink-0 bg-ink/8" />
    <div
      v-if="item.operation"
      class="mt-3 shrink-0 rounded-panel bg-ink/4 px-4 py-2.5 text-xs"
      aria-live="polite"
    >
      <div class="flex items-center gap-3">
        <span class="font-medium">{{
          installing && currentStep
            ? t(`store.steps.${currentStep.id}.title`)
            : t(`studio.taskStatus.${item.operation.status}`)
        }}</span>
        <ProgressBar
          class="flex-1"
          :percent="operationProgress(item.operation)"
          :trickle-to="operationStepCeiling(item.operation)"
          :active="installing"
          :indeterminate="installing && currentStep?.indeterminate"
          compact
          :tone="
            item.operation.status === 'failed'
              ? 'danger'
              : item.operation.status === 'finished'
                ? 'success'
                : 'signal'
          "
        />
        <span class="tabular-nums text-muted"
          >{{ operationProgress(item.operation) }}%</span
        >
        <StudioButton
          v-if="installing"
          size="sm"
          :disabled="!currentStep?.cancellable"
          @click="emit('cancel')"
        >
          {{ t("common.cancel") }}
        </StudioButton>
      </div>
      <StudioCallout
        v-if="item.operation.error"
        tone="danger"
        :title="errorTitle"
        class="mt-2"
      >
        {{ errorBody }}
        <template v-if="errorReason === 'signatureRejected'" #action>
          <StudioButton variant="link" size="sm" @click="emit('prepare')">
            {{ t("preparation.appSync.action") }}
          </StudioButton>
        </template>
      </StudioCallout>
    </div>
    <div class="mt-3 flex shrink-0 items-center gap-3">
      <StudioSegmented v-model="tab" :options="tabs" size="sm" />
      <div
        v-if="tab === 'previews' && previewPages > 1"
        class="ml-auto flex items-center gap-1 text-xs text-muted"
      >
        <span class="tabular-nums">{{
          t("common.pageOf", { page: previewPage + 1, total: previewPages })
        }}</span>
        <StudioButton
          size="sm"
          variant="ghost"
          :disabled="previewPage <= 0"
          :aria-label="t('common.previous')"
          @click="previewPage -= 1"
        >
          <IconPhCaretLeft width="13" height="13" />
        </StudioButton>
        <StudioButton
          size="sm"
          variant="ghost"
          :disabled="previewPage >= previewPages - 1"
          :aria-label="t('common.next')"
          @click="previewPage += 1"
        >
          <IconPhCaretRight width="13" height="13" />
        </StudioButton>
      </div>
      <div
        v-else-if="tab === 'history' && historyPages > 1"
        class="ml-auto flex items-center gap-1 text-xs text-muted"
      >
        <span class="tabular-nums">{{
          t("common.pageOf", { page: historyPage + 1, total: historyPages })
        }}</span>
        <StudioButton
          size="sm"
          variant="ghost"
          :disabled="historyPage <= 0"
          :aria-label="t('common.previous')"
          @click="historyPage -= 1"
        >
          <IconPhCaretLeft width="13" height="13" />
        </StudioButton>
        <StudioButton
          size="sm"
          variant="ghost"
          :disabled="historyPage >= historyPages - 1"
          :aria-label="t('common.next')"
          @click="historyPage += 1"
        >
          <IconPhCaretRight width="13" height="13" />
        </StudioButton>
      </div>
    </div>
    <div ref="body" class="mt-3 min-h-0 flex-1">
      <div
        v-if="tab === 'description'"
        class="grid h-full grid-cols-[minmax(0,1fr)_280px] gap-10"
      >
        <section class="min-w-0">
          <h2 class="text-lg font-semibold">{{ t("studio.description") }}</h2>
          <p
            class="mt-1.5 line-clamp-[var(--lines)] text-sm leading-6 whitespace-pre-line"
            :style="{ '--lines': descriptionLines }"
          >
            {{ description }}
          </p>
        </section>
        <section class="min-w-0">
          <h2 class="text-lg font-semibold">{{ t("studio.information") }}</h2>
          <dl class="mt-1.5 grid grid-cols-2 gap-x-4 gap-y-2 text-xs">
            <div
              v-for="row in information"
              :key="row.label"
              class="min-w-0"
              :class="row.mono && 'col-span-2'"
              :title="row.value"
            >
              <dt class="text-2xs text-muted">{{ row.label }}</dt>
              <dd class="truncate" :class="row.mono && 'font-mono'">
                {{ row.value }}
              </dd>
            </div>
            <div v-if="sourceUrl" class="col-span-2">
              <a
                :href="sourceUrl"
                target="_blank"
                rel="noopener noreferrer"
                class="inline-flex items-center gap-1 text-signal hover:underline"
                >{{ t("store.detail.sourceCode")
                }}<IconPhArrowSquareOut width="11" height="11"
              /></a>
            </div>
          </dl>
        </section>
      </div>
      <div
        v-else-if="tab === 'previews'"
        class="flex h-full gap-3"
        :aria-label="t('studio.appPreviews')"
      >
        <StoreMedia
          v-for="(media, index) in pageShots"
          :key="media.blob.sha256"
          :blob="media.blob"
          :alt="t('store.screenshot', { name, index: index + 1 })"
          class="h-full shrink-0 rounded-panel shadow-panel"
          :style="{ width: `${shotHeight * SCREENSHOT_RATIO}px` }"
        />
      </div>
      <template v-else-if="tab === 'history'">
        <div v-if="history.length" class="flex flex-col gap-2">
          <article
            v-for="release in pageReleases"
            :key="release.id"
            class="flex flex-col"
          >
            <div class="flex flex-wrap items-center gap-2 text-xs">
              <b class="font-medium"
                >{{ release.version }} ·
                {{
                  t("store.detail.revision", { revision: release.revision })
                }}</b
              ><span class="text-muted">{{
                d(release.published_at, "date")
              }}</span
              ><StatusPill v-if="release.status === 'yanked'" tone="warning">{{
                t("store.state.withdrawn")
              }}</StatusPill>
            </div>
            <p class="mt-0.5 line-clamp-2 text-xs leading-[17px] text-muted">
              {{ release.notes[locale] ?? release.notes.en }}
            </p>
          </article>
        </div>
        <p v-else class="text-xs text-muted">
          {{ item.entry.version }} · {{ d(item.entry.publishedAt, "date") }}
        </p>
      </template>
      <StepList
        v-else-if="tab === 'steps' && item.operation"
        :steps="item.operation.steps"
        label-prefix="store.steps"
      />
    </div>
  </article>
</template>
