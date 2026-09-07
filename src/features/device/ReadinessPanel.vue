<script setup lang="ts">
import IconStudioCheck from "~icons/studio/check";
import IconStudioWarning from "~icons/studio/warning";
import IconStudioMinus from "~icons/studio/minus";
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import {
  useGateway,
  type DeviceSummary,
  type ReadinessReport,
} from "../../shared/gateway";
import { useOperations } from "../../shared/composables/useOperations";
const props = defineProps<{
  report: ReadinessReport | null;
  device?: DeviceSummary | null;
  checking: boolean;
  compact?: boolean;
}>();
const emit = defineEmits<{
  recheck: [];
  prepare: [];
  details: [];
  openStore: [];
}>();
const { t, d } = useI18n();
const gateway = useGateway();
const { active } = useOperations();
const checks = computed(
  () =>
    props.report?.checks.filter(
      (check) =>
        !props.compact ||
        [
          "osVersionSupported",
          "pairingTrusted",
          "jailbroken",
          "appSyncInstalled",
          "sshAvailable",
        ].includes(check.id),
    ) ?? [],
);
const passCount = computed(
  () => checks.value.filter((check) => check.status === "pass").length,
);
const canReviewPreparation = computed(() => {
  if (!gateway.capabilities.preparation || !props.report) return false;
  if (
    props.device?.id === props.report.deviceId &&
    props.device.mode === "dfu" &&
    props.device.modelIdentifier === "iPod4,1" &&
    props.device.boardConfig === "N81AP" &&
    props.device.ecidMasked
  )
    return true;
  if (props.report.requiredWorkflow) return true;
  const checks = props.report.checks;
  return (
    gateway.flavor === "tauri" &&
    [
      "modelSupported",
      "osVersionSupported",
      "normalMode",
      "pairingTrusted",
      "batteryLevel",
    ].every((id) =>
      checks.some((check) => check.id === id && check.status === "pass"),
    ) &&
    checks.some(
      (check) => check.id === "jailbroken" && check.status === "unknown",
    )
  );
});
</script>
<template>
  <section
    :data-detailed="!compact"
    class="group/readiness overflow-hidden rounded-lg border border-line bg-surface data-[detailed=true]:mb-5"
    :aria-busy="checking"
  >
    <header
      class="flex items-center justify-between gap-[15px] px-4 pt-4 pb-1.5"
    >
      <div>
        <h2 class="text-[15px] font-semibold">
          {{
            t(
              report?.status === "ready"
                ? "studio.readinessReady"
                : report?.status === "needsAttention"
                  ? "connection.readinessPendingTitle"
                  : report?.status === "unsupported"
                    ? "connection.unsupportedTitle"
                    : "studio.readinessTitle",
            )
          }}
        </h2>
      </div>
      <span
        v-if="report"
        :data-ready="report.status === 'ready'"
        class="rounded-[10px] border border-line px-2 py-px text-[12px] whitespace-nowrap text-muted data-[ready=true]:border-success/35 data-[ready=true]:text-success"
        >{{ passCount }} / {{ checks.length }} {{ t("studio.passed") }}</span
      >
    </header>
    <div v-if="!report" class="p-7 text-sm text-muted flex items-center gap-2">
      <IconStudioRefresh
        :class="{ 'motion-safe:animate-studio-spin': checking }"
        width="16"
        height="16"
      />{{ t(checking ? "readiness.checking" : "studio.checkFailed") }}
    </div>
    <ul v-else class="px-4 py-0">
      <li
        v-for="check in checks"
        :key="check.id"
        class="flex items-center gap-2.5 border-b border-line px-0 py-[9px] last:border-0 group-data-[detailed=true]/readiness:py-[15px]"
      >
        <span
          :data-status="check.status"
          class="grid h-[18px] w-4 place-items-center rounded-none bg-transparent text-muted data-[status=pass]:text-success data-[status=fail]:text-warning data-[status=warn]:text-warning"
          ><component
            :is="
              check.status === 'pass'
                ? IconStudioCheck
                : check.status === 'fail'
                  ? IconStudioWarning
                  : IconStudioMinus
            "
            width="14"
            height="14"
        /></span>
        <div class="flex-1">
          <h3 class="text-[13px]">
            {{ t(`readiness.checks.${check.id}.title`) }}
          </h3>
          <p v-if="!compact" class="mt-1 text-[11px] text-muted">
            {{ t(`readiness.checks.${check.id}.${check.status}`) }}
          </p>
        </div>
        <span class="text-[12px] text-muted">{{
          check.value || t(`studio.checkStates.${check.status}`)
        }}</span>
      </li>
    </ul>
    <p
      v-if="
        report?.checks.some(
          (check) => check.id === 'appSyncInstalled' && check.status !== 'pass',
        )
      "
      class="mx-4 mt-3 rounded-md border border-warning/30 bg-warning/5 p-3 text-xs leading-6 text-muted"
    >
      {{ t("preparation.appSync.explanation") }}
    </p>
    <footer
      class="flex items-center justify-between gap-[15px] px-4 pt-3 pb-4 max-[800px]:flex-wrap"
    >
      <div class="flex flex-wrap gap-2">
        <button
          v-if="canReviewPreparation"
          class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
          :disabled="checking || !!active.length"
          @click="emit('prepare')"
        >
          {{
            t(
              report?.requiredWorkflow === "appSync"
                ? "preparation.appSync.action"
                : "preparation.reviewPlan",
            )
          }}<IconStudioArrowRight width="14" height="14" /></button
        ><button
          v-else-if="
            report?.status === 'ready' && gateway.capabilities.packages
          "
          class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
          @click="emit('openStore')"
        >
          {{ t("device.next.action")
          }}<IconStudioArrowRight width="14" height="14" /></button
        ><button
          class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
          :disabled="checking"
          @click="emit('recheck')"
        >
          <IconStudioRefresh
            width="13"
            height="13"
            :class="{ 'motion-safe:animate-studio-spin': checking }"
          />{{ t("readiness.recheck") }}
        </button>
      </div>
      <button
        v-if="compact"
        class="inline-flex items-center gap-[5px] text-[11px] text-signal hover:underline hover:underline-offset-[3px]"
        @click="emit('details')"
      >
        {{ t("studio.allConditions")
        }}<IconStudioChevronRight width="12" height="12" /></button
      ><span v-else-if="report" class="text-xs text-muted">{{
        t("readiness.checkedAt", { time: d(report.checkedAt, "time") })
      }}</span>
    </footer>
  </section>
</template>
