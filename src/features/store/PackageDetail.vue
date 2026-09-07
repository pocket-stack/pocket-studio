<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useGateway } from "../../shared/gateway";
import {
  operationProgress,
  useOperations,
} from "../../shared/composables/useOperations";
import ProgressBar from "../../shared/ui/ProgressBar.vue";
import StepList from "../../shared/ui/StepList.vue";
import PackageArtwork from "./PackageArtwork.vue";
import StoreMedia from "./StoreMedia.vue";
import { formatBytes } from "./compatibility";
import { packageText, packageMedia } from "./packageContent";
import type { PackageView } from "./useStore";
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
]);
</script>

<template>
  <article class="flex min-h-full gap-7">
    <div class="min-w-0 flex-1">
      <header class="mb-6 flex items-center gap-6 max-[850px]:gap-4">
        <PackageArtwork
          :entry="item.entry"
          :package-id="item.entry.id"
          :size="144"
          class="max-[1150px]:size-[112px]!"
        />
        <div class="min-w-0">
          <h1 class="break-words text-[26px] font-semibold tracking-tight">
            {{ name }}
          </h1>
          <p class="mt-2 text-xs leading-6 text-muted">
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
          <div class="mt-4 flex flex-wrap items-center gap-2.5">
            <button
              class="inline-flex items-center justify-center gap-2 rounded-md bg-signal px-4 py-1.5 text-sm text-on-signal enabled:hover:brightness-110 disabled:cursor-not-allowed disabled:opacity-45"
              :disabled="!canInstall"
              @click="emit('install')"
            >
              {{ installing ? t("store.state.installing") : installLabel }}
            </button>
            <button
              v-if="item.verdict === 'requiresPreparation'"
              class="rounded-md border border-line bg-raised px-3 py-1.5 text-xs hover:border-muted"
              @click="emit('prepare')"
            >
              {{ t("store.detail.prepareDevice") }}
            </button>
            <span class="text-[11px] text-muted">{{
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
            class="mt-3 text-xs leading-6"
            :class="
              item.verdict === 'compatible' ? 'text-success' : 'text-warning'
            "
          >
            {{ t(`store.verdict.${item.verdict}`) }}
          </p>
        </div>
      </header>
      <div
        v-if="item.missingDependencies.length"
        class="mb-4 flex flex-wrap gap-2 text-xs"
      >
        <span>{{ t("store.detail.installDependenciesFirst") }}</span
        ><button
          v-for="id in item.missingDependencies"
          :key="id"
          class="text-signal hover:underline"
          @click="emit('dependency', id)"
        >
          {{ t(`catalog.${id}.name`) }}
        </button>
      </div>
      <p v-if="item.queuePosition" class="mb-4 text-xs text-muted">
        {{ t("studio.queued", { position: item.queuePosition })
        }}<button
          class="ml-3 text-signal hover:underline"
          @click="emit('cancel')"
        >
          {{ t("common.cancel") }}
        </button>
      </p>
      <section
        v-if="item.operation"
        class="mb-6 rounded-md border border-line bg-surface p-4 text-xs"
        aria-live="polite"
      >
        <div class="flex justify-between gap-4">
          <h2>
            {{
              installing && currentStep
                ? t(`store.steps.${currentStep.id}.title`)
                : t(`studio.taskStatus.${item.operation.status}`)
            }}
          </h2>
          <span>{{ operationProgress(item.operation) }}%</span>
        </div>
        <ProgressBar
          class="mt-3"
          :percent="operationProgress(item.operation)"
          :active="installing"
          :tone="
            item.operation.status === 'failed'
              ? 'danger'
              : item.operation.status === 'finished'
                ? 'success'
                : 'signal'
          "
        />
        <div v-if="item.operation.error" class="mt-3">
          <b class="text-danger">{{ errorTitle }}</b>
          <p class="mt-1 leading-6 text-muted">
            {{ errorBody }}
          </p>
        </div>
        <button
          v-if="errorReason === 'signatureRejected'"
          class="mt-3 text-signal hover:underline"
          @click="emit('prepare')"
        >
          {{ t("preparation.appSync.action") }}
        </button>
        <details class="mt-3">
          <summary class="cursor-pointer text-signal">
            {{ t("studio.viewDetails") }}
          </summary>
          <StepList :steps="item.operation.steps" label-prefix="store.steps" />
        </details>
        <button
          v-if="installing"
          class="mt-3 rounded-md border border-line bg-raised px-3 py-1.5 disabled:opacity-45"
          :disabled="!currentStep?.cancellable"
          @click="emit('cancel')"
        >
          {{ t("common.cancel") }}
        </button>
      </section>
      <section
        v-if="screenshots.length"
        class="mb-7"
        :aria-label="t('studio.appPreviews')"
      >
        <div class="flex gap-3 overflow-x-auto pb-2">
          <StoreMedia
            v-for="(media, index) in screenshots"
            :key="media.blob.sha256"
            :blob="media.blob"
            :alt="t('store.screenshot', { name, index: index + 1 })"
            class="h-[290px] w-[194px] shrink-0 rounded-lg border border-line"
          />
        </div>
      </section>
      <section class="border-t border-line py-5">
        <h2 class="mb-3 text-sm font-semibold">
          {{ t("studio.description") }}
        </h2>
        <p class="whitespace-pre-line text-sm leading-7">{{ description }}</p>
      </section>
      <section class="border-t border-line py-5">
        <h2 class="mb-4 text-sm font-semibold">
          {{ t("studio.versionHistory") }}
        </h2>
        <template v-if="history.length"
          ><article
            v-for="release in history"
            :key="release.id"
            class="mb-5 last:mb-0"
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
              ><span
                v-if="release.status === 'yanked'"
                class="rounded bg-warning/10 px-1.5 py-0.5 text-warning"
                >{{ t("store.state.withdrawn") }}</span
              >
            </div>
            <p class="mt-2 whitespace-pre-line text-xs leading-6 text-muted">
              {{ release.notes[locale] ?? release.notes.en }}
            </p>
          </article></template
        >
        <p v-else class="text-xs text-muted">
          {{ item.entry.version }} · {{ d(item.entry.publishedAt, "date") }}
        </p>
      </section>
    </div>
    <aside
      class="w-[205px] shrink-0 border-l border-line pl-6 text-xs max-[1150px]:w-[180px] max-[850px]:w-40"
    >
      <h2 class="mb-4 text-sm font-semibold">{{ t("studio.information") }}</h2>
      <dl>
        <div v-for="fact in facts" :key="fact.label" class="mb-4">
          <dt class="mb-1 text-muted">{{ fact.label }}</dt>
          <dd class="break-words leading-5">{{ fact.value }}</dd>
        </div>
      </dl>
      <h3 class="mb-1 mt-5 text-muted">{{ t("store.detail.policy") }}</h3>
      <p class="leading-5">
        {{ t(`store.policy.${item.entry.installPolicy}`) }}
      </p>
      <h3 class="mb-2 mt-5 text-muted">{{ t("store.detail.checksum") }}</h3>
      <code class="block break-all text-[10px] leading-5">{{
        item.entry.checksumSha256
      }}</code>
      <a
        v-if="sourceUrl"
        :href="sourceUrl"
        target="_blank"
        rel="noopener noreferrer"
        class="mt-5 inline-flex items-center gap-1 text-signal hover:underline"
        >{{ t("store.detail.sourceCode")
        }}<IconPhArrowSquareOut width="12" height="12"
      /></a>
    </aside>
  </article>
</template>
