<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import { useStore } from "./useStore";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import StatusPill from "../../shared/ui/StatusPill.vue";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioPanel from "../../shared/ui/StudioPanel.vue";
const emit = defineEmits<{ detail: [id: string] }>();
const store = useStore();
const { device } = useDeviceSession();
const { t, locale, te } = useI18n();
const jobs = computed(() =>
  store.packageJobs.value
    .filter(
      (job, index) =>
        ["queued", "running", "verifying"].includes(job.phase) || index < 3,
    )
    .slice(0, 4),
);
const errorText = (reason?: string) => {
  const key = `store.actions.errors.${reason}`;
  return te(key) ? t(key) : t("store.actions.errors.unknown");
};
function tone(phase: string): "warning" | "success" | "neutral" | "signal" {
  if (["failed", "unverified", "interrupted"].includes(phase)) return "warning";
  if (phase === "verified") return "success";
  if (["running", "verifying"].includes(phase)) return "signal";
  return "neutral";
}
</script>
<template>
  <StudioPanel v-if="jobs.length" :padded="false" class="px-4 py-2">
    <h2 class="text-xs font-semibold">{{ t("store.actions.operations") }}</h2>
    <ul class="mt-1 flex flex-col">
      <li
        v-for="job in jobs"
        :key="job.handle.operationId"
        class="flex items-center gap-3 py-1.5 text-xs"
      >
        <div class="min-w-0 flex-1">
          <span class="font-medium">{{
            job.plan.names[locale] ?? job.plan.names.en ?? job.plan.appId
          }}</span>
          <span class="text-muted">
            · {{ job.plan.deviceName }} ·
            {{ t(`store.actions.kind.${job.plan.action}`)
            }}<template v-if="job.plan.version">
              · {{ job.plan.version }} · r{{ job.plan.revision }}</template
            ></span
          >
          <p v-if="job.error" class="truncate text-2xs text-warning">
            {{ errorText(job.error.diagnostic?.reason ?? job.error.code) }}
          </p>
          <p
            v-if="job.cleanupComplete === false"
            class="truncate text-2xs text-muted"
          >
            {{ t("store.actions.cleanupPending") }}
          </p>
        </div>
        <StatusPill :tone="tone(job.phase)">{{
          t(`store.actions.phase.${job.phase}`)
        }}</StatusPill>
        <StudioButton
          v-if="['queued', 'running'].includes(job.phase) && !job.submitted"
          size="sm"
          variant="ghost"
          @click="store.cancelJob(job)"
        >
          {{ t("common.cancel") }}
        </StudioButton>
        <StudioButton
          v-if="
            job.submitted &&
            ['unverified', 'failed', 'interrupted'].includes(job.phase)
          "
          size="sm"
          :disabled="!device"
          @click="store.verifyJob(job)"
        >
          {{ t("store.actions.verify") }}
        </StudioButton>
        <StudioButton
          size="sm"
          variant="link"
          @click="emit('detail', job.plan.appId)"
        >
          {{ t("studio.viewDetails") }}
        </StudioButton>
      </li>
    </ul>
  </StudioPanel>
</template>
