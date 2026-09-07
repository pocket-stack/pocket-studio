<script setup lang="ts">
import IconPhCheckCircleFill from "~icons/ph/check-circle-fill";
import IconPhWarningFill from "~icons/ph/warning-fill";
import IconPhCircleDashed from "~icons/ph/circle-dashed";
import { computed, ref } from "vue";
import { useI18n } from "vue-i18n";
import {
  useGateway,
  type DeviceSummary,
  type ReadinessReport,
} from "../../shared/gateway";
import AppSyncCheckDialog from "./AppSyncCheckDialog.vue";
import { useOperations } from "../../shared/composables/useOperations";
import StatusPill from "../../shared/ui/StatusPill.vue";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioPanel from "../../shared/ui/StudioPanel.vue";
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
const checkingAppSync = ref(false);
const appSync = computed(() =>
  props.report?.checks.find((check) => check.id === "appSyncInstalled"),
);
const canCheckAppSync = computed(
  () =>
    gateway.flavor === "tauri" &&
    !!props.device &&
    props.device.mode === "normal" &&
    props.report?.checks.some(
      (check) => check.id === "sshAvailable" && check.status === "pass",
    ),
);
const appSyncUnconfirmed = computed(
  () => appSync.value?.status === "unknown" || appSync.value?.status === "warn",
);
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
  if (props.report.requiredWorkflow === "appSync" && appSyncUnconfirmed.value)
    return false;
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
const statusIcon = {
  pass: IconPhCheckCircleFill,
  fail: IconPhWarningFill,
  warn: IconPhWarningFill,
  unknown: IconPhCircleDashed,
};
const statusClass = {
  pass: "text-success",
  fail: "text-warning",
  warn: "text-warning",
  unknown: "text-muted",
};
</script>
<template>
  <StudioPanel
    :padded="false"
    class="flex min-h-0 flex-col overflow-hidden"
    :aria-busy="checking"
  >
    <header class="flex items-center justify-between gap-3 px-4 pt-3 pb-1">
      <h2 class="text-base font-semibold">
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
      <StatusPill
        v-if="report"
        :tone="report.status === 'ready' ? 'success' : 'neutral'"
        >{{ passCount }} / {{ checks.length }}
        {{ t("studio.passed") }}</StatusPill
      >
    </header>
    <div
      v-if="!report"
      class="flex items-center gap-2 px-4 py-5 text-sm text-muted"
    >
      <IconSvgSpinners90Ring v-if="checking" width="15" height="15" />
      <IconPhWarning v-else width="15" height="15" />
      {{ t(checking ? "readiness.checking" : "studio.checkFailed") }}
    </div>
    <ul v-else class="px-2" :class="!compact && 'grid grid-cols-2 gap-x-4'">
      <li
        v-for="check in checks"
        :key="check.id"
        class="flex items-center gap-2.5 rounded-control px-2 hover:bg-ink/4"
        :class="compact ? 'py-1.5' : 'py-[7px]'"
      >
        <component
          :is="statusIcon[check.status] ?? IconPhCircleDashed"
          width="15"
          height="15"
          :class="statusClass[check.status] ?? 'text-muted'"
        />
        <div class="min-w-0 flex-1">
          <h3 class="text-sm leading-[18px]">
            {{ t(`readiness.checks.${check.id}.title`) }}
          </h3>
          <p v-if="!compact" class="truncate text-xs text-muted">
            {{
              check.id === "appSyncInstalled" &&
              appSyncUnconfirmed &&
              check.previousObservation
                ? t(
                    check.previousObservation.installed
                      ? "readiness.appSyncCheck.previousInstalled"
                      : "readiness.appSyncCheck.previousMissing",
                    {
                      time:
                        d(check.previousObservation.observedAt, "date") +
                        " " +
                        d(check.previousObservation.observedAt, "time"),
                    },
                  )
                : t(`readiness.checks.${check.id}.${check.status}`)
            }}
          </p>
        </div>
        <span class="shrink-0 text-xs text-muted">{{
          check.id === "appSyncInstalled" &&
          appSyncUnconfirmed &&
          check.previousObservation
            ? t(
                check.previousObservation.installed
                  ? "readiness.appSyncCheck.lastInstalled"
                  : "readiness.appSyncCheck.lastMissing",
              )
            : check.value || t(`studio.checkStates.${check.status}`)
        }}</span>
      </li>
    </ul>
    <StudioCallout
      v-if="
        report?.checks.some(
          (check) => check.id === 'appSyncInstalled' && check.status !== 'pass',
        )
      "
      tone="warning"
      class="mx-4 mt-2"
    >
      {{
        t(
          appSyncUnconfirmed
            ? "readiness.appSyncCheck.unknownExplanation"
            : "preparation.appSync.explanation",
        )
      }}
    </StudioCallout>
    <footer class="flex flex-wrap items-center justify-between gap-2 px-4 py-3">
      <div class="flex flex-wrap gap-2">
        <StudioButton
          v-if="canCheckAppSync"
          :disabled="checking || !!active.length"
          @click="checkingAppSync = true"
        >
          {{ t("readiness.appSyncCheck.action") }}
        </StudioButton>
        <StudioButton
          v-if="canReviewPreparation"
          variant="primary"
          :disabled="checking || !!active.length"
          @click="emit('prepare')"
        >
          {{
            t(
              report?.requiredWorkflow === "appSync"
                ? "preparation.appSync.action"
                : "preparation.reviewPlan",
            )
          }}<IconPhArrowRight width="14" height="14" />
        </StudioButton>
        <StudioButton
          v-else-if="
            report?.status === 'ready' && gateway.capabilities.packages
          "
          variant="primary"
          @click="emit('openStore')"
        >
          {{ t("device.next.action")
          }}<IconPhArrowRight width="14" height="14" />
        </StudioButton>
        <StudioButton :disabled="checking" @click="emit('recheck')">
          <IconSvgSpinners90Ring v-if="checking" width="13" height="13" />
          <IconPhArrowsClockwise v-else width="13" height="13" />{{
            t("readiness.recheck")
          }}
        </StudioButton>
      </div>
      <StudioButton v-if="compact" variant="link" @click="emit('details')">
        {{ t("studio.allConditions")
        }}<IconPhCaretRight width="12" height="12" />
      </StudioButton>
      <span v-else-if="report" class="text-xs text-muted">{{
        t("readiness.checkedAt", { time: d(report.checkedAt, "time") })
      }}</span>
    </footer>
  </StudioPanel>
  <AppSyncCheckDialog
    v-if="device"
    :open="checkingAppSync"
    :device-id="device.id"
    @close="checkingAppSync = false"
    @checked="emit('recheck')"
  />
</template>
