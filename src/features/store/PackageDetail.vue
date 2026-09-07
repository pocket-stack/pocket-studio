<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
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
import StudioPanel from "../../shared/ui/StudioPanel.vue";
import StudioSegmented from "../../shared/ui/StudioSegmented.vue";
import PackageArtwork from "./PackageArtwork.vue";
import StoreMedia from "./StoreMedia.vue";
import { formatBytes } from "./compatibility";
import { packageText, packageMedia } from "./packageContent";
import type { PackageView } from "./useStore";

const SCREENSHOT_RATIO = 194 / 290;
const RELEASE_ROW = 58;

const props = defineProps<{ item: PackageView }>();
const emit = defineEmits<{
  install: [];
  cancel: [];
  prepare: [];
  close: [];
  dependency: [id: string];
}>();
const { t, d, locale, te } = useI18n();
const gateway = useGateway();
const { active } = useOperations();
const name = computed(() =>
  packageText(props.item.entry, "name", locale.value, t),
);
const description = computed(() =>
  packageText(props.item.entry, "description", locale.value, t),
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
const installLabel = computed(() =>
  props.item.installed
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
const facts = computed(() => [
  { label: t("store.detail.developer"), value: props.item.entry.developer },
  {
    label: t("store.detail.size"),
    value: formatBytes(props.item.entry.sizeBytes),
  },
  {
    label: t("store.detail.compatibility"),
    value: `${props.item.entry.compatibility.platform === "ios" ? t("store.platform.ios") : props.item.entry.compatibility.platform} ${props.item.entry.compatibility.minOsVersion} – ${props.item.entry.compatibility.maxOsVersion}`,
  },
  {
    label: t("store.detail.models"),
    value: props.item.entry.compatibility.models.join(", "),
  },
  {
    label: t("store.detail.jailbreak"),
    value: t(
      props.item.entry.compatibility.requiresJailbreak
        ? "common.required"
        : "common.notRequired",
    ),
  },
  {
    label: t("store.detail.policy"),
    value: t(`store.policy.${props.item.entry.installPolicy}`),
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
  <article class="flex h-full min-h-0 flex-col gap-3">
    <StudioButton
      variant="link"
      size="sm"
      class="self-start"
      @click="emit('close')"
    >
      <IconPhArrowLeft width="13" height="13" />{{ t("studio.backToStore") }}
    </StudioButton>
    <header class="flex items-start gap-5">
      <PackageArtwork
        :entry="item.entry"
        :package-id="item.entry.id"
        :size="96"
        class="rounded-[18%] shadow-raised"
      />
      <div class="min-w-0 flex-1">
        <h1 class="truncate text-2xl font-semibold tracking-tight">
          {{ name }}
        </h1>
        <p class="text-xs text-muted">
          {{ item.entry.developer }} ·
          {{ t(`store.category.${item.entry.category}`) }} ·
          {{ item.entry.version
          }}<span v-if="item.entry.details">
            ·
            {{
              t("store.detail.revision", {
                revision: item.entry.details.revision,
              })
            }}</span
          >
        </p>
        <div class="mt-3 flex flex-wrap items-center gap-2">
          <StudioButton
            variant="primary"
            :disabled="!canInstall"
            @click="emit('install')"
          >
            {{ installing ? t("store.state.installing") : installLabel }}
          </StudioButton>
          <StudioButton
            v-if="item.verdict === 'requiresPreparation'"
            @click="emit('prepare')"
          >
            {{ t("store.detail.prepareDevice") }}
          </StudioButton>
          <StatusPill
            :tone="item.verdict === 'compatible' ? 'success' : 'warning'"
            >{{ t(`store.verdict.${item.verdict}`) }}</StatusPill
          >
          <span class="text-xs text-muted">{{
            item.entry.details
              ? t("store.detail.catalogVerified")
              : t(
                  item.entry.signed
                    ? "store.detail.signed"
                    : "store.detail.unsigned",
                )
          }}</span>
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
      <dl
        class="grid w-[300px] shrink-0 grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs max-[1100px]:w-[240px]"
      >
        <template v-for="fact in facts" :key="fact.label">
          <dt class="text-muted">{{ fact.label }}</dt>
          <dd class="truncate" :title="fact.value">{{ fact.value }}</dd>
        </template>
      </dl>
    </header>
    <StudioPanel
      v-if="item.operation"
      :padded="false"
      class="px-4 py-2.5 text-xs"
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
    </StudioPanel>
    <div class="flex items-center gap-3">
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
    <StudioPanel class="flex min-h-0 flex-1 flex-col overflow-hidden">
      <div ref="body" class="min-h-0 flex-1 overflow-hidden">
        <p
          v-if="tab === 'description'"
          class="text-sm leading-6 whitespace-pre-line"
        >
          {{ description }}
        </p>
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
            class="h-full shrink-0 rounded-control shadow-panel"
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
                ><StatusPill
                  v-if="release.status === 'yanked'"
                  tone="warning"
                  >{{ t("store.state.withdrawn") }}</StatusPill
                >
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
    </StudioPanel>
    <footer class="flex items-center gap-3 text-2xs text-muted">
      <span class="truncate font-mono"
        >{{ t("store.detail.checksum") }} {{ item.entry.checksumSha256 }}</span
      >
      <a
        v-if="sourceUrl"
        :href="sourceUrl"
        target="_blank"
        rel="noopener noreferrer"
        class="ml-auto inline-flex shrink-0 items-center gap-1 text-signal hover:underline"
        >{{ t("store.detail.sourceCode")
        }}<IconPhArrowSquareOut width="11" height="11"
      /></a>
    </footer>
  </article>
</template>
