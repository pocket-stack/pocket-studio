import { computed, readonly, ref, watch } from "vue";

import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { useOperationLog } from "../../shared/composables/useOperationLog";
import { notify } from "../../shared/composables/useNotifications";
import {
  trackOperation,
  useOperations,
  type OperationState,
} from "../../shared/composables/useOperations";
import { useDefaultSshPassword } from "../../shared/preferences/sshPassword";
import {
  GatewayError,
  useGateway,
  type ConsentRecord,
  type PrerequisiteId,
  type PreparationPlan,
  type RiskId,
} from "../../shared/gateway";

export type PreparationStage =
  | "closed"
  | "overview"
  | "risks"
  | "disclaimer"
  | "starting"
  | "awaitingDfu"
  | "running"
  | "success"
  | "failed"
  | "cancelled";

/** Consent older than this must be re-collected before a retry. */
const CONSENT_VALIDITY_MS = 30 * 60 * 1000;

const stage = ref<PreparationStage>("closed");
const minimized = ref(false);
const plan = ref<PreparationPlan | null>(null);
const planError = ref<string | null>(null);
const confirmedPrerequisites = ref<PrerequisiteId[]>([]);
const acknowledgedRisks = ref<RiskId[]>([]);
const riskReadingSeconds = ref(0);
const risksAcknowledgedAt = ref<number | null>(null);
const disclaimerReadingSeconds = ref(0);
const disclaimerAcceptedAt = ref<number | null>(null);
const operationId = ref<string | null>(null);
const startError = ref<string | null>(null);
const sshPassword = ref("");
const attempt = ref(0);
// An empty field means the default from Preferences.
const effectiveSshPassword = computed(
  () => sshPassword.value || useDefaultSshPassword().effective.value,
);

const { get } = useOperations();

const operation = computed<OperationState | undefined>(() =>
  operationId.value ? get(operationId.value) : undefined,
);

watch(
  () => {
    const current = operation.value;
    return current ? `${current.status}:${current.pendingAction ?? ""}` : "";
  },
  () => {
    const current = operation.value;
    if (!current || stage.value === "closed") return;
    switch (current.status) {
      case "running":
        stage.value =
          current.pendingAction === "enterDfu" ? "awaitingDfu" : "running";
        break;
      case "finished":
        stage.value = "success";
        void useDeviceSession().checkReadiness();
        break;
      case "failed":
        stage.value = "failed";
        break;
      case "cancelled":
        stage.value = "cancelled";
        break;
    }
  },
);

function resetConsent(): void {
  confirmedPrerequisites.value = [];
  acknowledgedRisks.value = [];
  riskReadingSeconds.value = 0;
  risksAcknowledgedAt.value = null;
  disclaimerReadingSeconds.value = 0;
  disclaimerAcceptedAt.value = null;
}

async function open(deviceId: string): Promise<void> {
  if (operation.value?.status === "running") {
    minimized.value = false;
    return;
  }
  minimized.value = false;
  resetConsent();
  sshPassword.value = "";
  operationId.value = null;
  startError.value = null;
  planError.value = null;
  plan.value = null;
  attempt.value = 0;
  stage.value = "overview";
  try {
    plan.value = await useGateway().preparation.plan(deviceId);
  } catch (error) {
    planError.value = error instanceof GatewayError ? error.code : "unknown";
  }
}

function close(): void {
  if (operation.value?.status === "running") {
    minimized.value = true;
    return;
  }
  stage.value = "closed";
  sshPassword.value = "";
}

function togglePrerequisite(id: PrerequisiteId): void {
  const next = new Set(confirmedPrerequisites.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  confirmedPrerequisites.value = [...next];
}

function proceedToRisks(): void {
  if (
    !plan.value ||
    stage.value !== "overview" ||
    (useGateway().flavor === "tauri" && !effectiveSshPassword.value)
  )
    return;
  if (confirmedPrerequisites.value.length !== plan.value.prerequisites.length)
    return;
  acknowledgedRisks.value = [];
  risksAcknowledgedAt.value = null;
  disclaimerAcceptedAt.value = null;
  stage.value = "risks";
}

function acknowledgeRisks(readingSeconds: number): void {
  if (
    !plan.value ||
    stage.value !== "risks" ||
    readingSeconds < plan.value.minimumReadingSeconds.risks
  )
    return;
  acknowledgedRisks.value = plan.value.risks.map((risk) => risk.id);
  riskReadingSeconds.value = readingSeconds;
  risksAcknowledgedAt.value = Date.now();
  useOperationLog().recordUiEvent(
    "log.preparation.risksAcknowledged",
    "User acknowledged every risk",
    {
      plan: plan.value.id,
      readingSeconds: String(readingSeconds),
      risks: acknowledgedRisks.value.join(","),
    },
  );
  stage.value = "disclaimer";
}

function backToOverview(): void {
  stage.value = "overview";
}

function backToRisks(): void {
  risksAcknowledgedAt.value = null;
  disclaimerAcceptedAt.value = null;
  stage.value = "risks";
}

function buildConsent(): ConsentRecord | null {
  const current = plan.value;
  if (
    !current ||
    risksAcknowledgedAt.value === null ||
    disclaimerAcceptedAt.value === null
  )
    return null;
  return {
    planId: current.id,
    prerequisitesConfirmed: confirmedPrerequisites.value,
    acknowledgedRiskIds: acknowledgedRisks.value,
    riskReadingSeconds: riskReadingSeconds.value,
    risksAcknowledgedAt: risksAcknowledgedAt.value,
    disclaimerVersion: current.disclaimerVersion,
    disclaimerReadingSeconds: disclaimerReadingSeconds.value,
    disclaimerAcceptedAt: disclaimerAcceptedAt.value,
  };
}

async function launch(): Promise<void> {
  const consent = buildConsent();
  if (!consent) return;
  stage.value = "starting";
  startError.value = null;
  try {
    await useOperations().ready();
    const handle = await useGateway().preparation.start(
      consent,
      effectiveSshPassword.value,
    );
    attempt.value += 1;
    trackOperation(handle);
    operationId.value = handle.operationId;
  } catch (error) {
    startError.value = error instanceof GatewayError ? error.code : "unknown";
    stage.value = "disclaimer";
    notify("error", "notifications.preparationStartFailed");
  } finally {
    sshPassword.value = "";
  }
}

async function acceptDisclaimer(readingSeconds: number): Promise<void> {
  if (
    !plan.value ||
    stage.value !== "disclaimer" ||
    risksAcknowledgedAt.value === null ||
    readingSeconds < plan.value.minimumReadingSeconds.disclaimer
  )
    return;
  disclaimerReadingSeconds.value = readingSeconds;
  disclaimerAcceptedAt.value = Date.now();
  useOperationLog().recordUiEvent(
    "log.preparation.disclaimerAccepted",
    "User accepted the preparation disclaimer",
    {
      plan: plan.value.id,
      version: plan.value.disclaimerVersion,
      readingSeconds: String(readingSeconds),
    },
  );
  await launch();
}

function consentStillValid(): boolean {
  return (
    disclaimerAcceptedAt.value !== null &&
    Date.now() - disclaimerAcceptedAt.value < CONSENT_VALIDITY_MS
  );
}

async function retry(): Promise<void> {
  if (
    !plan.value ||
    !["failed", "cancelled"].includes(stage.value) ||
    (operation.value?.status === "failed" &&
      !operation.value.error?.recoverable)
  )
    return;
  if (useGateway().flavor === "tauri") {
    // Native plans are consumed once, and the device must be identified again
    // in a supported mode before a fresh confirmation can authorize another run.
    const device = useDeviceSession().device.value;
    if (device) await open(device.id);
    else notify("warning", "notifications.deviceDetached");
    return;
  }
  if (!consentStillValid()) {
    resetConsent();
    stage.value = "overview";
    notify("warning", "notifications.consentExpired");
    return;
  }
  await launch();
}

async function cancel(): Promise<void> {
  if (!operationId.value) return;
  try {
    await useOperations().cancel(operationId.value);
    notify("info", "notifications.cancelRequested");
  } catch (error) {
    const code = error instanceof GatewayError ? error.code : "unknown";
    notify(
      "warning",
      code === "notCancellable"
        ? "notifications.cancelNotAllowed"
        : "notifications.cancelFailed",
    );
  }
}

export function usePreparation() {
  return {
    sshPassword: readonly(sshPassword),
    setSshPassword: (value: string) => {
      sshPassword.value = value;
    },
    stage: readonly(stage),
    visible: computed(() => stage.value !== "closed" && !minimized.value),
    consentVisible: computed(() =>
      ["risks", "disclaimer"].includes(stage.value),
    ),
    resume: () => {
      minimized.value = false;
    },
    minimize: () => {
      minimized.value = true;
    },
    plan: computed(() => plan.value),
    planError: readonly(planError),
    startError: readonly(startError),
    attempt: readonly(attempt),
    confirmedPrerequisites: readonly(confirmedPrerequisites),
    operation,
    allPrerequisitesConfirmed: computed(
      () =>
        plan.value !== null &&
        confirmedPrerequisites.value.length ===
          plan.value.prerequisites.length &&
        (useGateway().flavor !== "tauri" ||
          effectiveSshPassword.value.length > 0),
    ),
    open,
    close,
    togglePrerequisite,
    proceedToRisks,
    acknowledgeRisks,
    acceptDisclaimer,
    backToOverview,
    backToRisks,
    retry,
    cancel,
  };
}
