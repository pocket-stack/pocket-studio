import { computed, reactive, readonly } from "vue";

import {
  useGateway,
  type OperationError,
  type OperationEvent,
  type OperationHandle,
  type OperationKind,
  type PlanStep,
  type RequiredAction,
  type StepId,
  type StepStatus,
} from "../gateway";

export interface StepState extends PlanStep {
  status: StepStatus;
  percent: number;
  indeterminate?: boolean;
}

export type OperationStatus = "running" | "finished" | "failed" | "cancelled";

export interface OperationState {
  id: string;
  kind: OperationKind;
  subject?: string;
  packagePlan?: import("../gateway").PackagePlan;
  steps: StepState[];
  status: OperationStatus;
  currentStepId?: StepId;
  pendingAction?: RequiredAction;
  error?: OperationError;
  startedAt: number;
  endedAt?: number;
}

const operations = reactive(new Map<string, OperationState>());
let subscribed = false;
let subscription: Promise<void> | undefined;
// Native events can arrive before the command promise returns its handle.
const pendingEvents = new Map<string, OperationEvent[]>();

function apply(event: OperationEvent): void {
  const operation = operations.get(event.operationId);
  if (!operation) {
    const pending = pendingEvents.get(event.operationId) ?? [];
    pending.push(event);
    pendingEvents.set(event.operationId, pending.slice(-100));
    if (pendingEvents.size > 32)
      pendingEvents.delete(pendingEvents.keys().next().value!);
    return;
  }

  switch (event.type) {
    case "started":
      break;
    case "stepChanged": {
      const step = operation.steps.find((item) => item.id === event.stepId);
      if (!step) return;
      step.status = event.status;
      if (event.status === "running") {
        operation.currentStepId = step.id;
        step.percent = 0;
        step.indeterminate =
          useGateway().flavor === "tauri" &&
          operation.kind !== "preparation" &&
          !["download", "transfer"].includes(step.id);
      }
      if (event.status === "done") step.percent = 100;
      break;
    }
    case "progress": {
      const step = operation.steps.find((item) => item.id === event.stepId);
      if (step) {
        step.percent = event.percent;
        step.indeterminate = false;
      }
      break;
    }
    case "actionRequired":
      operation.pendingAction = event.action;
      break;
    case "actionResolved":
      operation.pendingAction = undefined;
      break;
    case "finished":
      operation.status = "finished";
      operation.endedAt = Date.now();
      operation.currentStepId = undefined;
      operation.pendingAction = undefined;
      break;
    case "failed":
      operation.status = "failed";
      operation.error = event.error;
      operation.endedAt = Date.now();
      operation.pendingAction = undefined;
      break;
    case "cancelled":
      operation.status = "cancelled";
      operation.endedAt = Date.now();
      operation.pendingAction = undefined;
      for (const step of operation.steps) {
        if (step.status === "pending") step.status = "skipped";
      }
      break;
  }
}

function ensureSubscribed(): Promise<void> {
  if (subscribed) return Promise.resolve();
  if (!subscription) {
    subscription = Promise.resolve(useGateway().operations.onEvent(apply))
      .then(() => {
        subscribed = true;
      })
      .finally(() => {
        subscription = undefined;
      });
  }
  return subscription;
}

export function trackOperation(handle: OperationHandle): OperationState {
  void ensureSubscribed().catch(() => {});
  const state: OperationState = {
    id: handle.operationId,
    kind: handle.kind,
    subject: handle.subject,
    steps: handle.steps.map((step) => ({
      ...step,
      status: "pending",
      percent: 0,
    })),
    status: "running",
    startedAt: Date.now(),
  };
  operations.set(handle.operationId, state);
  for (const event of pendingEvents.get(handle.operationId) ?? []) apply(event);
  pendingEvents.delete(handle.operationId);
  return state;
}

export async function cancelOperation(operationId: string): Promise<void> {
  await useGateway().operations.cancel(operationId);
}

export function operationProgress(operation: OperationState): number {
  const total = operation.steps.reduce(
    (sum, step) => sum + step.estimatedSeconds,
    0,
  );
  if (total === 0) return 0;
  const done = operation.steps.reduce((sum, step) => {
    if (step.status === "done") return sum + step.estimatedSeconds;
    if (step.status === "running")
      return sum + (step.estimatedSeconds * step.percent) / 100;
    return sum;
  }, 0);
  return Math.round((done / total) * 100);
}

export function useOperations() {
  void ensureSubscribed().catch(() => {});
  return {
    ready: ensureSubscribed,
    operations: readonly(operations),
    get: (id: string) => operations.get(id),
    active: computed(() =>
      [...operations.values()].filter((item) => item.status === "running"),
    ),
    track: trackOperation,
    cancel: cancelOperation,
  };
}

/** Restore native package jobs after webview reload without replaying commands. */
export function hydratePackageJobs(
  jobs: import("../gateway").PackageJob[],
): void {
  for (const job of jobs) {
    const existing = operations.get(job.handle.operationId);
    const state = existing ?? trackOperation(job.handle);
    const active = ["queued", "running", "verifying"].includes(job.phase);
    state.packagePlan = job.plan;
    if (state.status !== "running" && active) continue;
    const previousStep = state.currentStepId;
    state.status = active
      ? "running"
      : job.phase === "verified"
        ? "finished"
        : job.phase === "cancelled"
          ? "cancelled"
          : "failed";
    state.currentStepId =
      job.phase === "queued" || !active ? undefined : job.step;
    state.error = job.error ?? undefined;
    for (const step of state.steps) {
      if (job.completedSteps.includes(step.id)) {
        step.status = "done";
        step.percent = 100;
      } else if (step.id === job.step && job.phase !== "queued") {
        step.status = active
          ? "running"
          : job.phase === "cancelled"
            ? "cancelled"
            : job.phase === "verified"
              ? "done"
              : "failed";
        step.percent =
          previousStep === job.step
            ? Math.max(step.percent, job.percent)
            : job.percent;
      } else step.status = active ? "pending" : "skipped";
      step.indeterminate =
        step.status === "running" &&
        !["download", "transfer"].includes(step.id) &&
        step.percent === 0;
    }
  }
}
