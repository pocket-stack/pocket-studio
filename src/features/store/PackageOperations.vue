<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "./useStore";
import { useOperations } from "../../shared/composables/useOperations";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import StepList from "../../shared/ui/StepList.vue";
const store = useStore();
const { get } = useOperations();
const { device } = useDeviceSession();
const { t, locale, te } = useI18n();
const jobs = computed(() =>
  store.packageJobs.value.filter(
    (job, index) =>
      ["queued", "running", "verifying"].includes(job.phase) || index < 20,
  ),
);
const errorText = (reason?: string) => {
  const key = `store.actions.errors.${reason}`;
  return te(key) ? t(key) : t("store.actions.errors.unknown");
};
</script>
<template>
  <section
    v-if="jobs.length"
    class="mb-6 space-y-4 rounded-lg border border-line bg-surface p-5"
  >
    <h2 class="text-sm font-semibold">{{ t("store.actions.operations") }}</h2>
    <article
      v-for="job in jobs"
      :key="job.handle.operationId"
      class="border-t border-line pt-4 text-xs"
    >
      <div class="flex items-start justify-between gap-4">
        <div class="min-w-0">
          <h3 class="font-medium">
            {{ job.plan.names[locale] ?? job.plan.names.en ?? job.plan.appId }}
          </h3>
          <p class="mt-1 break-words text-muted">
            {{ job.plan.deviceName }} ·
            {{ t(`store.actions.kind.${job.plan.action}`)
            }}<template v-if="job.plan.version">
              · {{ job.plan.version }} · r{{ job.plan.revision }}</template
            >
          </p>
        </div>
        <span
          :class="
            ['failed', 'unverified', 'interrupted'].includes(job.phase)
              ? 'text-warning'
              : job.phase === 'verified'
                ? 'text-success'
                : 'text-muted'
          "
          >{{ t(`store.actions.phase.${job.phase}`) }}</span
        >
      </div>
      <p v-if="job.error" class="mt-2 leading-6 text-warning">
        {{ errorText(job.error.diagnostic?.reason ?? job.error.code) }}
      </p>
      <p v-if="job.cleanupComplete === false" class="mt-2 text-muted">
        {{ t("store.actions.cleanupPending") }}
      </p>
      <div class="mt-3 flex items-center gap-4">
        <button
          v-if="['queued', 'running'].includes(job.phase) && !job.submitted"
          class="text-signal"
          @click="store.cancelJob(job)"
        >
          {{ t("common.cancel") }}
        </button>
        <button
          v-if="
            job.submitted &&
            ['unverified', 'failed', 'interrupted'].includes(job.phase)
          "
          class="text-signal disabled:opacity-50"
          :disabled="!device"
          @click="store.verifyJob(job)"
        >
          {{ t("store.actions.verify") }}
        </button>
        <details v-if="get(job.handle.operationId)" class="min-w-0 flex-1">
          <summary class="cursor-pointer text-muted">
            {{ t("studio.viewDetails") }}
          </summary>
          <StepList
            class="mt-3"
            :steps="get(job.handle.operationId)!.steps"
            label-prefix="store.steps"
          />
        </details>
      </div>
    </article>
  </section>
</template>
