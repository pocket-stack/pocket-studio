<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import {
  operationProgress,
  useOperations,
} from "../../shared/composables/useOperations";
import AppIcon from "../../shared/ui/AppIcon.vue";
import ProgressBar from "../../shared/ui/ProgressBar.vue";
import StepList from "../../shared/ui/StepList.vue";
import PackageArtwork from "./PackageArtwork.vue";
import { formatBytes } from "./compatibility";
import type { PackageView } from "./useStore";
const props = defineProps<{ item: PackageView }>();
const emit = defineEmits<{
  install: [];
  cancel: [];
  prepare: [];
  close: [];
  dependency: [id: string];
}>();
const { t, d } = useI18n();
const { active } = useOperations();
const installing = computed(() => props.item.operation?.status === "running");
const currentStep = computed(() =>
  props.item.operation?.steps.find(
    (step) => step.id === props.item.operation?.currentStepId,
  ),
);
const canInstall = computed(
  () =>
    props.item.verdict === "compatible" &&
    !installing.value &&
    !props.item.queuePosition &&
    !props.item.missingDependencies.length &&
    !active.value.some((op) => op.kind === "preparation"),
);
const installLabel = computed(() =>
  props.item.installed
    ? props.item.installed.version === props.item.entry.version
      ? t("store.detail.reinstall")
      : t("store.detail.update")
    : t("studio.installToDevice", { name: t("studio.deviceName") }),
);
const facts = computed(() => [
  { label: t("store.detail.developer"), value: props.item.entry.developer },
  {
    label: t("store.detail.size"),
    value: formatBytes(props.item.entry.sizeBytes),
  },
  {
    label: t("studio.category"),
    value: t(`store.category.${props.item.entry.category}`),
  },
  {
    label: t("store.detail.compatibility"),
    value: `iOS ${props.item.entry.compatibility.minOsVersion} – ${props.item.entry.compatibility.maxOsVersion}`,
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
  { label: t("studio.sources"), value: t("studio.demoCatalog") },
]);
</script>
<template>
  <article class="flex min-h-full gap-7">
    <div class="min-w-0 flex-1">
      <header
        class="mb-[22px] flex items-center gap-7 max-[600px]:flex-wrap max-[600px]:gap-5"
      >
        <PackageArtwork
          class="max-[1150px]:size-[115px]!"
          :package-id="item.entry.id"
          :size="160"
        />
        <div>
          <h1 class="text-[26px] font-semibold">
            {{ t(`catalog.${item.entry.id}.name`) }}
          </h1>
          <p class="mt-2 text-[12px] text-muted">
            {{ item.entry.developer }} ·
            {{ t(`store.category.${item.entry.category}`) }} ·
            {{ t("store.detail.version") }} {{ item.entry.version }} ·
            {{ formatBytes(item.entry.sizeBytes) }} ·
            {{ d(item.entry.publishedAt, "date") }}
          </p>
          <div class="mt-[18px] flex flex-wrap items-center gap-2.5">
            <button
              class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
              :disabled="!canInstall"
              @click="emit('install')"
            >
              {{
                installing ? t("store.state.installing") : installLabel
              }}</button
            ><button
              v-if="item.verdict === 'requiresPreparation'"
              class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
              @click="emit('prepare')"
            >
              {{ t("store.detail.prepareDevice") }}</button
            ><span
              class="text-[12px]"
              :class="
                item.verdict === 'compatible' ? 'text-success' : 'text-warning'
              "
              >{{ t(`store.verdict.${item.verdict}`) }}</span
            >
          </div>
          <div
            v-if="item.missingDependencies.length"
            class="mt-2.5 flex flex-wrap gap-2 text-[12px]"
          >
            <span>{{ t("store.detail.installDependenciesFirst") }}</span
            ><button
              v-for="id in item.missingDependencies"
              :key="id"
              class="inline-flex items-center gap-[5px] text-[11px] text-signal hover:underline hover:underline-offset-[3px]"
              @click="emit('dependency', id)"
            >
              {{ t(`catalog.${id}.name`)
              }}<AppIcon name="chevronRight" :size="11" />
            </button>
          </div>
        </div>
      </header>
      <p v-if="item.queuePosition" class="mb-4 text-xs text-muted">
        {{ t("studio.queued", { position: item.queuePosition }) }}
        <button
          class="ml-3 inline-flex items-center gap-[5px] text-[11px] text-signal hover:underline hover:underline-offset-[3px]"
          @click="emit('cancel')"
        >
          {{ t("common.cancel") }}
        </button>
      </p>
      <section
        v-if="item.operation"
        class="mb-[22px] rounded-md border border-line bg-surface p-4 text-[12px]"
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
        <div v-if="item.operation.error" class="mt-3 text-xs">
          <b class="text-danger">{{
            t(`store.errors.${item.operation.error.code}.title`)
          }}</b>
          <p class="mt-1 text-muted">
            {{ t(`store.errors.${item.operation.error.code}.body`) }}
          </p>
        </div>
        <details class="mt-3">
          <summary
            class="inline-flex items-center gap-[5px] text-[11px] text-signal hover:underline hover:underline-offset-[3px]"
          >
            {{ t("studio.viewDetails") }}
          </summary>
          <StepList :steps="item.operation.steps" label-prefix="store.steps" />
        </details>
        <button
          v-if="installing"
          class="mt-3 inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
          :disabled="!currentStep?.cancellable"
          @click="emit('cancel')"
        >
          {{ t("common.cancel") }}
        </button>
      </section>
      <div
        class="mb-[26px] flex gap-3 overflow-x-auto"
        :aria-label="t('studio.appPreviews')"
      >
        <div
          v-for="index in 4"
          :key="index"
          class="flex h-[270px] w-[150px] shrink-0 flex-col items-center gap-[18px] rounded-lg bg-track p-3.5 max-[600px]:h-[245px] max-[600px]:w-[135px]"
        >
          <span class="flex w-full gap-[3px]" aria-hidden="true"
            ><i class="size-[3px] rounded-full bg-muted" /><i
              class="size-[3px] rounded-full bg-muted" /><i
              class="size-[3px] rounded-full bg-muted" /></span
          ><PackageArtwork :package-id="item.entry.id" :size="40" />
          <div class="flex w-full flex-col gap-3.5" aria-hidden="true">
            <i
              class="h-[7px] w-full rounded-[2px] bg-muted/12 nth-[2]:w-[70%]"
            /><i
              class="h-[7px] w-full rounded-[2px] bg-muted/12 nth-[2]:w-[70%]"
            /><i
              class="h-[7px] w-full rounded-[2px] bg-muted/12 nth-[2]:w-[70%]"
            /><i
              class="h-[7px] w-full rounded-[2px] bg-muted/12 nth-[2]:w-[70%]"
            />
          </div>
          <small class="mt-auto text-[10px] text-muted">{{
            t("studio.previewNumber", { index })
          }}</small>
        </div>
      </div>
      <div
        class="grid grid-cols-[1fr_300px] gap-7 max-[1150px]:grid-cols-[1fr_230px] max-[850px]:grid-cols-1"
      >
        <section>
          <h2 class="mb-[9px] text-[14px] font-semibold">
            {{ t("studio.description") }}
          </h2>
          <p class="text-[13px] leading-[1.8] font-normal">
            {{ t(`catalog.${item.entry.id}.description`) }}
          </p>
        </section>
        <section>
          <h2 class="mb-[9px] text-[14px] font-semibold">
            {{ t("studio.versionHistory") }}
          </h2>
          <b class="text-[13px] leading-[1.8] font-normal"
            >{{ item.entry.version }} ·
            {{ d(item.entry.publishedAt, "date") }}</b
          >
          <p class="text-[13px] leading-[1.8] font-normal">
            {{ t("studio.releaseNote") }}
          </p>
        </section>
      </div>
    </div>
    <aside
      class="w-[220px] shrink-0 border-l border-line pl-6 text-[12px] max-[1150px]:w-[190px] max-[850px]:w-[170px] max-[600px]:hidden"
    >
      <h2 class="mb-3 text-[14px] font-semibold">
        {{ t("studio.information") }}
      </h2>
      <dl>
        <div
          v-for="fact in facts"
          :key="fact.label"
          class="mt-2.5 flex flex-wrap gap-1 leading-[1.7]"
        >
          <dt class="text-muted">{{ fact.label }}</dt>
          <dd>{{ fact.value }}</dd>
        </div>
      </dl>
      <h3 class="mt-[22px] mb-2.5 text-muted">
        {{ t("store.detail.policy") }}
      </h3>
      <p class="text-[12px] leading-[1.8]">
        {{ t(`store.policy.${item.entry.installPolicy}`) }}
      </p>
      <h3 class="mt-[22px] mb-2.5 text-muted">
        {{ t("store.detail.checksum") }}
      </h3>
      <code class="text-[12px] leading-[1.8]">{{
        item.entry.checksumSha256
      }}</code>
      <p class="mt-5 text-muted text-[12px] leading-[1.8]">
        {{ t("studio.previewNotice") }}
      </p>
    </aside>
  </article>
</template>
