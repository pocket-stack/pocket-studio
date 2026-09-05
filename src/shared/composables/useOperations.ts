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
}

export type OperationStatus = "running" | "finished" | "failed" | "cancelled";

export interface OperationState {
  id: string;
  kind: OperationKind;
  subject?: string;
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

function apply(event: OperationEvent): void {
  const operation = operations.get(event.operationId);
  if (!operation) return;

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
      }
      if (event.status === "done") step.percent = 100;
      break;
    }
    case "progress": {
      const step = operation.steps.find((item) => item.id === event.stepId);
      if (step) step.percent = event.percent;
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

function ensureSubscribed(): void {
  if (subscribed) return;
  subscribed = true;
  useGateway().operations.onEvent(apply);
}

export function trackOperation(handle: OperationHandle): OperationState {
  ensureSubscribed();
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
  ensureSubscribed();
  return {
    operations: readonly(operations),
    get: (id: string) => operations.get(id),
    active: computed(() =>
      [...operations.values()].filter((item) => item.status === "running"),
    ),
    track: trackOperation,
    cancel: cancelOperation,
  };
}
