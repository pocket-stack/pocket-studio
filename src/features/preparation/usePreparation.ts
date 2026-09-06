import { computed, readonly, ref, watch } from "vue";

import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { useOperationLog } from "../../shared/composables/useOperationLog";
import { notify } from "../../shared/composables/useNotifications";
import {
  trackOperation,
  useOperations,
  type OperationState,
} from "../../shared/composables/useOperations";
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
const attempt = ref(0);

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
}

function togglePrerequisite(id: PrerequisiteId): void {
  const next = new Set(confirmedPrerequisites.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  confirmedPrerequisites.value = [...next];
}

function toggleRisk(id: RiskId): void {
  const next = new Set(acknowledgedRisks.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  acknowledgedRisks.value = [...next];
}

function proceedToRisks(): void {
  if (!plan.value || stage.value !== "overview") return;
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
  if (acknowledgedRisks.value.length !== plan.value.risks.length) return;
  riskReadingSeconds.value = readingSeconds;
  risksAcknowledgedAt.value = Date.now();
  useOperationLog().recordUiEvent(
    "log.preparation.risksAcknowledged",
    "User acknowledged every risk (simulation)",
    {
      plan: plan.value.id,
      readingSeconds: String(readingSeconds),
      risks: acknowledgedRisks.value.join(","),
      scrolledToEnd: "true",
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
    const handle = await useGateway().preparation.start(consent);
    attempt.value += 1;
    trackOperation(handle);
    operationId.value = handle.operationId;
  } catch (error) {
    startError.value = error instanceof GatewayError ? error.code : "unknown";
    stage.value = "disclaimer";
    notify("error", "notifications.preparationStartFailed");
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
    "User accepted the demo disclaimer",
    {
      plan: plan.value.id,
      version: plan.value.disclaimerVersion,
      readingSeconds: String(readingSeconds),
      scrolledToEnd: "true",
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
    acknowledgedRisks: readonly(acknowledgedRisks),
    operation,
    allPrerequisitesConfirmed: computed(
      () =>
        plan.value !== null &&
        confirmedPrerequisites.value.length === plan.value.prerequisites.length,
    ),
    allRisksAcknowledged: computed(
      () =>
        plan.value !== null &&
        acknowledgedRisks.value.length === plan.value.risks.length,
    ),
    open,
    close,
    togglePrerequisite,
    toggleRisk,
    proceedToRisks,
    acknowledgeRisks,
    acceptDisclaimer,
    backToOverview,
    backToRisks,
    retry,
    cancel,
  };
}
