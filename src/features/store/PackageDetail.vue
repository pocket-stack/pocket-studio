<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import { operationProgress } from "../../shared/composables/useOperations";
import AppIcon from "../../shared/ui/AppIcon.vue";
import ProgressBar from "../../shared/ui/ProgressBar.vue";
import StatusPill from "../../shared/ui/StatusPill.vue";
import StepList from "../../shared/ui/StepList.vue";
import { formatBytes } from "./compatibility";
import type { PackageView } from "./useStore";

const props = defineProps<{ item: PackageView }>();
const emit = defineEmits<{ install: []; cancel: []; prepare: []; close: [] }>();
const { t, d } = useI18n();

const installing = computed(() => props.item.operation?.status === "running");
const currentStep = computed(() =>
  props.item.operation?.steps.find(
    (step) => step.id === props.item.operation?.currentStepId,
  ),
);
const canCancel = computed(
  () => installing.value && currentStep.value?.cancellable === true,
);
const canInstall = computed(
  () =>
    props.item.verdict === "compatible" &&
    !installing.value &&
    props.item.missingDependencies.length === 0,
);
const installLabel = computed(() =>
  props.item.installed
    ? props.item.installed.version === props.item.entry.version
      ? t("store.detail.reinstall")
      : t("store.detail.update")
    : t("store.detail.install"),
);
const lastError = computed(() =>
  props.item.operation?.status === "failed"
    ? props.item.operation.error
    : undefined,
);

const facts = computed(() => [
  { label: t("store.detail.version"), value: props.item.entry.version },
  { label: t("store.detail.developer"), value: props.item.entry.developer },
  {
    label: t("store.detail.size"),
    value: formatBytes(props.item.entry.sizeBytes),
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
    label: t("store.detail.checksum"),
    value: props.item.entry.checksumSha256,
    mono: true,
  },
]);
</script>

<template>
  <aside class="card flex min-h-0 flex-col">
    <header
      class="flex items-start justify-between gap-3 border-b border-line p-5"
    >
      <div class="min-w-0">
        <h2 class="text-lg font-semibold">
          {{ t(`catalog.${item.entry.id}.name`) }}
        </h2>
        <p class="mt-1 text-sm text-muted">
          {{ t(`catalog.${item.entry.id}.summary`) }}
        </p>
        <div class="mt-2 flex flex-wrap gap-1.5">
          <StatusPill tone="neutral">{{
            t(`store.category.${item.entry.category}`)
          }}</StatusPill>
          <StatusPill :tone="item.entry.signed ? 'success' : 'warning'">
            <AppIcon name="shield" :size="12" />
            {{
              item.entry.signed
                ? t("store.detail.signed")
                : t("store.detail.unsigned")
            }}
          </StatusPill>
          <StatusPill v-if="item.installed" tone="success">
            {{
              t("store.detail.installedVersion", {
                version: item.installed.version,
              })
            }}
          </StatusPill>
        </div>
      </div>
      <button class="btn btn-ghost -mr-2 px-2" @click="emit('close')">
        <AppIcon name="cross" :size="18" />
      </button>
    </header>

    <div class="scroll-thin min-h-0 flex-1 space-y-5 overflow-y-auto p-5">
      <section
        v-if="item.operation"
        class="rounded-xl border border-line bg-raised p-4"
      >
        <div class="flex items-center justify-between text-sm">
          <span class="font-medium">
            <template v-if="installing">{{
              currentStep ? t(`store.steps.${currentStep.id}.title`) : ""
            }}</template>
            <template v-else-if="item.operation.status === 'finished'">{{
              t("store.detail.installDone")
            }}</template>
            <template v-else-if="item.operation.status === 'cancelled'">{{
              t("store.detail.installCancelled")
            }}</template>
            <template v-else>{{ t("store.detail.installFailed") }}</template>
          </span>
          <span class="font-mono text-xs text-muted"
            >{{ operationProgress(item.operation) }}%</span
          >
        </div>
        <ProgressBar
          class="mt-2"
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
        <div v-if="lastError" class="mt-3 text-xs">
          <p class="font-medium text-danger">
            {{ t(`store.errors.${lastError.code}.title`) }}
          </p>
          <p class="mt-0.5 text-muted">
            {{ t(`store.errors.${lastError.code}.body`) }}
          </p>
        </div>
        <div class="mt-3">
          <StepList :steps="item.operation.steps" label-prefix="store.steps" />
        </div>
      </section>

      <section>
        <p class="text-sm leading-relaxed">
          {{ t(`catalog.${item.entry.id}.description`) }}
        </p>
      </section>

      <section>
        <h3 class="text-xs font-semibold tracking-wide text-muted uppercase">
          {{ t("store.detail.compatibility") }}
        </h3>
        <div
          class="mt-2 flex items-start gap-2 rounded-lg border p-3 text-sm"
          :class="
            item.verdict === 'compatible'
              ? 'border-success/40 bg-success/6'
              : 'border-warning/40 bg-warning/6'
          "
        >
          <AppIcon
            :name="item.verdict === 'compatible' ? 'check' : 'warning'"
            :size="16"
            class="mt-0.5"
            :class="
              item.verdict === 'compatible' ? 'text-success' : 'text-warning'
            "
          />
          <p>{{ t(`store.verdict.${item.verdict}`) }}</p>
        </div>
        <dl class="mt-3 grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
          <dt class="text-muted">{{ t("store.detail.models") }}</dt>
          <dd class="font-mono">
            {{ item.entry.compatibility.models.join(", ") }}
          </dd>
          <dt class="text-muted">{{ t("store.detail.osRange") }}</dt>
          <dd class="font-mono">
            iOS {{ item.entry.compatibility.minOsVersion }} –
            {{ item.entry.compatibility.maxOsVersion }}
          </dd>
          <dt class="text-muted">{{ t("store.detail.jailbreak") }}</dt>
          <dd>
            {{
              item.entry.compatibility.requiresJailbreak
                ? t("common.required")
                : t("common.notRequired")
            }}
          </dd>
        </dl>
      </section>

      <section v-if="item.entry.dependencies.length">
        <h3 class="text-xs font-semibold tracking-wide text-muted uppercase">
          {{ t("store.detail.dependencies") }}
        </h3>
        <ul class="mt-2 space-y-1 text-sm">
          <li
            v-for="dependency in item.entry.dependencies"
            :key="dependency"
            class="flex items-center gap-2"
          >
            <AppIcon
              :name="
                item.missingDependencies.includes(dependency)
                  ? 'warning'
                  : 'check'
              "
              :size="14"
              :class="
                item.missingDependencies.includes(dependency)
                  ? 'text-warning'
                  : 'text-success'
              "
            />
            {{ t(`catalog.${dependency}.name`) }}
            <span
              v-if="item.missingDependencies.includes(dependency)"
              class="text-xs text-muted"
              >{{ t("store.detail.dependencyMissing") }}</span
            >
          </li>
        </ul>
      </section>

      <section>
        <h3 class="text-xs font-semibold tracking-wide text-muted uppercase">
          {{ t("store.detail.details") }}
        </h3>
        <dl class="mt-2 text-xs">
          <div
            v-for="fact in facts"
            :key="fact.label"
            class="flex justify-between gap-4 border-b border-line/60 py-1.5"
          >
            <dt class="text-muted">{{ fact.label }}</dt>
            <dd :class="fact.mono ? 'font-mono' : ''">{{ fact.value }}</dd>
          </div>
        </dl>
      </section>
    </div>

    <footer
      class="flex items-center justify-between gap-3 border-t border-line p-4"
    >
      <p class="text-xs text-muted">
        <template v-if="item.verdict === 'requiresPreparation'">{{
          t("store.detail.prepareHint")
        }}</template>
        <template v-else-if="item.missingDependencies.length">{{
          t("store.detail.installDependenciesFirst")
        }}</template>
        <template v-else-if="item.verdict === 'compatible'">{{
          t("store.detail.installHint")
        }}</template>
      </p>
      <div class="flex gap-2">
        <button v-if="canCancel" class="btn btn-danger" @click="emit('cancel')">
          <AppIcon name="stop" :size="16" />
          {{ t("common.cancel") }}
        </button>
        <button
          v-if="item.verdict === 'requiresPreparation'"
          class="btn btn-secondary"
          @click="emit('prepare')"
        >
          <AppIcon name="bolt" :size="16" />
          {{ t("store.detail.prepareDevice") }}
        </button>
        <button
          class="btn btn-primary"
          :disabled="!canInstall"
          @click="emit('install')"
        >
          <AppIcon name="download" :size="16" />
          {{ installLabel }}
        </button>
      </div>
    </footer>
  </aside>
</template>
