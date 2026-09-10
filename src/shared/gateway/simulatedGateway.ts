import { createThreeDsDemo, threeDsReadiness } from "./threeDsDemo";
import { validateConsent } from "./consent";
import { compareVersions } from "../versions";
import {
  buildJailbreakPlan,
  demoCatalog,
  demoDevice,
  demoDevice3ds,
  installSteps,
  preparationFailureCodes,
} from "./fixtures";
import {
  GatewayError,
  type CatalogEntry,
  type DeviceEvent,
  type DeviceMode,
  type DeviceSummary,
  type InstalledPackage,
  type InstallStepId,
  type LogEntry,
  type LogLevel,
  type LogSource,
  type OperationError,
  type OperationEvent,
  type OperationHandle,
  type OperationKind,
  type PlanStep,
  type PreparationPlan,
  type PreparationStepId,
  type ReadinessCheck,
  type ReadinessReport,
  type StepId,
  type StudioGateway,
  type Unsubscribe,
} from "./types";

/**
 * In-browser stand-in for the native layer. It reproduces the same
 * event stream the Rust demo driver emits so UI work does not need Tauri.
 * Timing is compressed: one "estimated second" of a step is ~120 ms here.
 */

const DEMO_TIME_SCALE_MS = 120;
const DFU_WAIT_TIMEOUT_MS = 90_000;

type Handler<T> = (value: T) => void;

class Emitter<T> {
  private handlers = new Set<Handler<T>>();

  subscribe(handler: Handler<T>): Unsubscribe {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }

  emit(value: T): void {
    for (const handler of this.handlers) {
      handler(value);
    }
  }
}

interface RunningOperation {
  handle: OperationHandle;
  cancelRequested: boolean;
  currentStep?: PlanStep;
  finished: boolean;
  disconnected?: boolean;
  /** Resolves a pending "enter DFU" wait; set while the runner is blocked. */
  resolveDfu?: () => void;
  rejectDisconnect?: () => void;
}

class ModeWaiters {
  private waiters = new Set<(mode: DeviceMode | null) => void>();
  wait(timeoutMs: number): Promise<DeviceMode | null> {
    return new Promise((resolve) => {
      const complete = (mode: DeviceMode | null) => {
        clearTimeout(timer);
        this.waiters.delete(complete);
        resolve(mode);
      };
      const timer = setTimeout(() => complete(null), timeoutMs);
      this.waiters.add(complete);
    });
  }
  notify(mode: DeviceMode): void {
    for (const waiter of this.waiters) waiter(mode);
    this.waiters.clear();
  }
}

const sleep = (ms: number) =>
  new Promise<void>((resolve) => setTimeout(resolve, ms));

export function createSimulatedGateway(): StudioGateway {
  let device: DeviceSummary | null = null;
  let jailbroken = false;
  let appsyncInstalled = false;
  // The 3DS ships with custom firmware already installed in the demo.
  let cfwInstalled = true;
  let planSequence = 0;
  let operationSequence = 0;
  let logSequence = 0;
  const sessionId = crypto.randomUUID().slice(0, 6);
  let failNext: StepId | null = null;

  const plans = new Map<string, PreparationPlan>();
  const operations = new Map<string, RunningOperation>();
  const installed: InstalledPackage[] = [];
  const logs: LogEntry[] = [];

  const deviceEvents = new Emitter<DeviceEvent>();
  const operationEvents = new Emitter<OperationEvent>();
  const logEvents = new Emitter<LogEntry>();
  const modeWaiters = new ModeWaiters();

  function log(
    level: LogLevel,
    source: LogSource,
    code: string,
    message: string,
    params?: Record<string, string>,
    operationId?: string,
  ): void {
    logSequence += 1;
    const entry: LogEntry = {
      id: `log-${logSequence}`,
      timestamp: Date.now(),
      level,
      source,
      code,
      message,
      params,
      operationId,
    };
    logs.push(entry);
    logEvents.emit(entry);
  }

  log(
    "info",
    "system",
    "log.system.started",
    "Pocket Studio started (browser demo mode)",
  );

  function assertIdle(): void {
    if ([...operations.values()].some((operation) => !operation.finished))
      throw new GatewayError("operationBusy", "The simulated device is busy");
  }

  function requireDevice(deviceId: string): DeviceSummary {
    if (!device || device.id !== deviceId) {
      throw new GatewayError(
        "deviceNotFound",
        `device ${deviceId} is not attached`,
      );
    }
    return device;
  }

  function readiness(current: DeviceSummary): ReadinessReport {
    if (current.platform === "3ds")
      return threeDsReadiness(current, cfwInstalled);
    const checks: ReadinessCheck[] = [
      { id: "platformSupported", status: "pass", value: "iOS" },
      {
        id: "modelSupported",
        status: "pass",
        value: current.modelIdentifier ?? undefined,
      },
      {
        id: "osVersionSupported",
        status: "pass",
        value: current.osVersion ?? undefined,
      },
      { id: "pairingTrusted", status: "pass" },
      { id: "jailbroken", status: jailbroken ? "pass" : "fail" },
      { id: "appSyncInstalled", status: appsyncInstalled ? "pass" : "fail" },
      { id: "sshAvailable", status: jailbroken ? "pass" : "unknown" },
      {
        id: "batteryLevel",
        status: (current.batteryPercent ?? 0) >= 50 ? "pass" : "warn",
        value: `${current.batteryPercent}%`,
      },
      { id: "physicalButtons", status: "unknown" },
    ];
    return {
      deviceId: current.id,
      status: jailbroken && appsyncInstalled ? "ready" : "needsPreparation",
      checks,
      requiredWorkflow: !jailbroken
        ? "jailbreak"
        : appsyncInstalled
          ? undefined
          : "appSync",
      checkedAt: Date.now(),
    };
  }

  function preparationError(stepId: PreparationStepId): OperationError {
    const spec = preparationFailureCodes[stepId];
    return {
      code: spec.code,
      recoverable: spec.recoverable,
      retryFromStepId: spec.retryFrom,
    };
  }

  function installError(stepId: InstallStepId): OperationError {
    switch (stepId) {
      case "download":
        return {
          code: "downloadFailed",
          recoverable: true,
          retryFromStepId: "download",
        };
      case "verify":
        return { code: "checksumMismatch", recoverable: false };
      case "transfer":
        return {
          code: "transferFailed",
          recoverable: true,
          retryFromStepId: "transfer",
        };
      case "install":
        return {
          code: "installRejected",
          recoverable: true,
          retryFromStepId: "install",
        };
      default:
        return {
          code: "verificationFailed",
          recoverable: true,
          retryFromStepId: "verifyInstall",
        };
    }
  }

  async function waitForDfu(operation: RunningOperation): Promise<boolean> {
    const deadline = Date.now() + DFU_WAIT_TIMEOUT_MS;
    while (Date.now() < deadline) {
      if (!device || operation.disconnected || operation.cancelRequested)
        return false;
      if (device.mode === "dfu") return true;
      const mode = await modeWaiters.wait(500);
      if (mode === "dfu") return true;
    }
    return false;
  }

  async function runOperation(
    kind: OperationKind,
    steps: PlanStep[],
    source: LogSource,
    operation: RunningOperation,
    onSuccess: () => void,
  ): Promise<void> {
    const { operationId } = operation.handle;
    operationEvents.emit({ type: "started", operationId, kind });
    log(
      "info",
      source,
      `log.${kind}.started`,
      `${kind} operation started`,
      undefined,
      operationId,
    );

    const fail = (step: PlanStep, error: OperationError) => {
      operation.finished = true;
      operationEvents.emit({
        type: "stepChanged",
        operationId,
        stepId: step.id,
        status: "failed",
      });
      operationEvents.emit({
        type: "failed",
        operationId,
        stepId: step.id,
        error,
      });
      log(
        "error",
        source,
        `log.${kind}.failed`,
        `${kind} failed at ${step.id}: ${error.code}`,
        { step: step.id, code: error.code },
        operationId,
      );
    };

    for (const step of steps) {
      if (operation.cancelRequested) {
        operation.finished = true;
        operationEvents.emit({
          type: "stepChanged",
          operationId,
          stepId: step.id,
          status: "cancelled",
        });
        operationEvents.emit({
          type: "cancelled",
          operationId,
          stepId: step.id,
        });
        log(
          "warn",
          source,
          `log.${kind}.cancelled`,
          `${kind} cancelled before ${step.id}`,
          { step: step.id },
          operationId,
        );
        return;
      }

      operation.currentStep = step;
      operationEvents.emit({
        type: "stepChanged",
        operationId,
        stepId: step.id,
        status: "running",
      });
      log(
        "info",
        source,
        "log.step.started",
        `step ${step.id} started`,
        { step: step.id },
        operationId,
      );

      if (step.requiresAction === "enterDfu") {
        operationEvents.emit({
          type: "actionRequired",
          operationId,
          stepId: step.id,
          action: "enterDfu",
        });
        log(
          "info",
          source,
          "log.step.actionRequired",
          "waiting for the device to enter DFU mode",
          { action: "enterDfu" },
          operationId,
        );
        const entered = await waitForDfu(operation);
        if (operation.cancelRequested) continue;
        if (!entered) {
          fail(
            step,
            device
              ? preparationError("enterDfu")
              : {
                  code: "deviceDisconnected",
                  recoverable: true,
                  retryFromStepId: "enterDfu",
                },
          );
          return;
        }
        operationEvents.emit({
          type: "actionResolved",
          operationId,
          stepId: step.id,
        });
        log(
          "info",
          "device",
          "log.device.modeChanged",
          "device entered DFU mode",
          { mode: "dfu" },
          operationId,
        );
      }

      const ticks = Math.max(4, Math.min(20, step.estimatedSeconds));
      const tickMs = (step.estimatedSeconds * DEMO_TIME_SCALE_MS) / ticks;
      const failAt = failNext === step.id ? Math.floor(ticks / 2) : -1;

      for (let tick = 1; tick <= ticks; tick += 1) {
        await sleep(tickMs);
        if (!device || operation.disconnected) {
          fail(step, {
            code: "deviceDisconnected",
            recoverable: true,
            retryFromStepId: kind === "preparation" ? "enterDfu" : "resolve",
          });
          return;
        }
        if (operation.cancelRequested && step.cancellable) {
          operation.finished = true;
          operationEvents.emit({
            type: "stepChanged",
            operationId,
            stepId: step.id,
            status: "cancelled",
          });
          operationEvents.emit({
            type: "cancelled",
            operationId,
            stepId: step.id,
          });
          log(
            "warn",
            source,
            `log.${kind}.cancelled`,
            `${kind} cancelled during ${step.id}`,
            { step: step.id },
            operationId,
          );
          return;
        }
        if (tick === failAt) {
          failNext = null;
          fail(
            step,
            kind === "preparation"
              ? preparationError(step.id as PreparationStepId)
              : installError(step.id as InstallStepId),
          );
          return;
        }
        operationEvents.emit({
          type: "progress",
          operationId,
          stepId: step.id,
          percent: Math.round((tick / ticks) * 100),
        });
      }

      if (step.id === "rebootDevice" && device) {
        device = { ...device, mode: "normal" };
        deviceEvents.emit({
          type: "modeChanged",
          deviceId: device.id,
          mode: "normal",
        });
        log(
          "info",
          "device",
          "log.device.modeChanged",
          "device rebooted to normal mode",
          { mode: "normal" },
          operationId,
        );
      }

      operationEvents.emit({
        type: "stepChanged",
        operationId,
        stepId: step.id,
        status: "done",
      });
      log(
        "debug",
        source,
        "log.step.done",
        `step ${step.id} done`,
        { step: step.id },
        operationId,
      );
    }

    operation.finished = true;
    onSuccess();
    operationEvents.emit({ type: "finished", operationId });
    log(
      "info",
      source,
      `log.${kind}.finished`,
      `${kind} finished successfully`,
      undefined,
      operationId,
    );
  }

  function newOperation(
    kind: OperationKind,
    steps: PlanStep[],
    subject?: string,
  ): RunningOperation {
    operationSequence += 1;
    const handle: OperationHandle = {
      operationId: `op-${sessionId}-${String(operationSequence).padStart(3, "0")}`,
      kind,
      steps,
      subject,
    };
    const operation: RunningOperation = {
      handle,
      cancelRequested: false,
      finished: false,
    };
    operations.set(handle.operationId, operation);
    return operation;
  }

  // The paired console is managed through instance-bound plans, like the
  // native layer; the legacy queue below only serves the iPod demo.
  const threeDs = createThreeDsDemo({
    device: () => device,
    cfwInstalled: () => cfwInstalled,
    attach: async () => {
      // One simulated device at a time: the console replaces the iPod.
      if (device && device.platform !== "3ds")
        await gateway.demo.detachDevice();
      await gateway.demo.attachDevice("n3dsll");
    },
    snapshot: () => gateway.devices.list(),
  });

  const gateway: StudioGateway = {
    flavor: "browser",
    capabilities: {
      demo: true,
      preparation: true,
      catalog: true,
      packages: true,
      installed: true,
    },
    devices: {
      async checkAppSync(deviceId) {
        return readiness(requireDevice(deviceId));
      },
      async list() {
        return {
          revision: 0,
          devices: device ? [device] : [],
          reports: device ? [readiness(device)] : [],
          issues: [],
          checkedAt: Date.now(),
        };
      },
      async checkReadiness(deviceId) {
        const current = requireDevice(deviceId);
        await sleep(600);
        const report = readiness(current);
        log(
          "info",
          "device",
          "log.device.readinessChecked",
          `readiness: ${report.status}`,
          { status: report.status },
        );
        return report;
      },
      onEvent: (handler) => deviceEvents.subscribe(handler),
    },
    setup: threeDs.setup,
    preparation: {
      async plan(deviceId) {
        const current = requireDevice(deviceId);
        if (current.platform !== "ios")
          throw new GatewayError(
            "unsupportedDevice",
            "Guided preparation exists for iOS devices only",
          );
        planSequence += 1;
        const plan = buildJailbreakPlan(
          deviceId,
          planSequence,
          current.mode === "dfu" ? "dfu" : "normal",
        );
        const appSyncSteps = [
          {
            id: "connectAppSync" as const,
            cancellable: true,
            pointOfNoReturn: false,
            estimatedSeconds: 10,
          },
          {
            id: "installAppSync" as const,
            cancellable: false,
            pointOfNoReturn: true,
            estimatedSeconds: 20,
          },
          {
            id: "activateAppSync" as const,
            cancellable: false,
            pointOfNoReturn: false,
            estimatedSeconds: 5,
          },
          {
            id: "verifyAppSync" as const,
            cancellable: false,
            pointOfNoReturn: false,
            estimatedSeconds: 5,
          },
        ];
        if (jailbroken) {
          plan.workflow = "appSync";
          plan.method = "ssh";
          plan.exploit = null;
          plan.tether = null;
          plan.prerequisites = [
            "backupCompleted",
            "stableCable",
            "computerAwake",
          ];
          plan.steps = [
            {
              id: "fetchResources",
              cancellable: true,
              pointOfNoReturn: false,
              estimatedSeconds: 5,
            },
            ...appSyncSteps,
          ];
        } else plan.steps = [...plan.steps, ...appSyncSteps];
        plans.set(plan.id, plan);
        log(
          "debug",
          "preparation",
          "log.preparation.planned",
          `plan ${plan.id} created`,
          { plan: plan.id },
        );
        return plan;
      },
      async start(consent) {
        const plan = plans.get(consent.planId);
        if (!plan) throw new GatewayError("unknownPlan", "plan not found");
        requireDevice(plan.deviceId);
        validateConsent(plan, consent, Date.now());
        assertIdle();
        const operation = newOperation("preparation", plan.steps);
        log(
          "info",
          "preparation",
          "log.preparation.consentRecorded",
          "consent recorded for plan",
          {
            plan: plan.id,
            risks: consent.acknowledgedRiskIds.join(","),
            riskReadingSeconds: String(consent.riskReadingSeconds),
            risksAcknowledgedAt: new Date(
              consent.risksAcknowledgedAt,
            ).toISOString(),
            disclaimerVersion: consent.disclaimerVersion,
            disclaimerReadingSeconds: String(consent.disclaimerReadingSeconds),
            disclaimerAcceptedAt: new Date(
              consent.disclaimerAcceptedAt,
            ).toISOString(),
            prerequisites: consent.prerequisitesConfirmed.join(","),
            mode: "simulation",
          },
          operation.handle.operationId,
        );
        void runOperation(
          "preparation",
          plan.steps,
          "preparation",
          operation,
          () => {
            jailbroken = true;
            appsyncInstalled = true;
          },
        );
        return operation.handle;
      },
    },
    store: {
      async plan(request) {
        if (requireDevice(request.deviceId).platform === "3ds")
          return threeDs.plan(request);
        throw new GatewayError(
          "operationUnavailable",
          "Native package planning is unavailable in the browser demo",
        );
      },
      async start(consent) {
        if (device?.platform === "3ds") return threeDs.start(consent);
        throw new GatewayError(
          "operationUnavailable",
          "Native package planning is unavailable in the browser demo",
        );
      },
      async jobs() {
        return threeDs.jobs();
      },
      async verify() {
        throw new GatewayError(
          "operationUnavailable",
          "Native verification is unavailable in the browser demo",
        );
      },
      async uninstall(deviceId, packageId) {
        if (requireDevice(deviceId).platform === "3ds")
          throw new GatewayError(
            "invalidAction",
            "3DS instances are removed through a plan",
          );
        assertIdle();
        const dependent = installed.find((item) =>
          demoCatalog
            .find((entry) => entry.id === item.packageId)
            ?.dependencies.includes(packageId),
        );
        if (dependent)
          throw new GatewayError(
            "packageInUse",
            "Installed packages depend on this package",
          );
        const index = installed.findIndex(
          (item) => item.packageId === packageId,
        );
        if (index === -1)
          throw new GatewayError("unknownPackage", "Package is not installed");
        installed.splice(index, 1);
        log(
          "info",
          "store",
          "log.store.uninstalled",
          `Uninstalled ${packageId} (simulation)`,
          { package: packageId },
        );
      },
      async catalog() {
        await sleep(300);
        return {
          entries: [...demoCatalog, ...threeDs.catalog()],
          source: "demo",
          sourceLabel: null,
          sequence: null,
          expiresAt: null,
          checkedAt: null,
          expired: false,
          verified: false,
          issue: null,
        };
      },
      async media() {
        throw new GatewayError(
          "operationUnavailable",
          "Media downloads are unavailable in the browser demo",
        );
      },
      async installed(deviceId) {
        const current = requireDevice(deviceId);
        return {
          deviceId,
          entries:
            current.platform === "3ds"
              ? threeDs.installed()
              : installed.slice(),
          state: "fresh",
          observedAt: Date.now(),
          issue: null,
        };
      },
      async install(deviceId, packageId) {
        const current = requireDevice(deviceId);
        if (current.platform === "3ds")
          throw new GatewayError(
            "invalidAction",
            "3DS installs are planned per instance",
          );
        assertIdle();
        const entry: CatalogEntry | undefined = demoCatalog.find(
          (item) => item.id === packageId,
        );
        if (!entry)
          throw new GatewayError("unknownPackage", "package not found");
        if (entry.compatibility.requiresJailbreak && !jailbroken) {
          throw new GatewayError(
            "deviceNotReady",
            "package requires a prepared device environment",
          );
        }
        if (current.mode !== "normal")
          throw new GatewayError(
            "deviceNotReady",
            "A normal-mode device is required",
          );
        if (
          !entry.compatibility.models.includes(current.modelIdentifier ?? "") ||
          compareVersions(
            current.osVersion ?? "",
            entry.compatibility.minOsVersion,
          ) < 0 ||
          compareVersions(
            current.osVersion ?? "",
            entry.compatibility.maxOsVersion ?? current.osVersion ?? "",
          ) > 0
        )
          throw new GatewayError(
            "incompatiblePackage",
            "This package is not compatible with the device",
          );
        if (
          entry.dependencies.some(
            (id) => !installed.some((item) => item.packageId === id),
          )
        )
          throw new GatewayError(
            "missingDependencies",
            "Required dependencies are not installed",
          );
        const operation = newOperation("install", installSteps, packageId);
        void runOperation("install", installSteps, "store", operation, () => {
          const existing = installed.findIndex(
            (item) => item.packageId === packageId,
          );
          const record = {
            installationId: `ios:${packageId}`,
            packageId,
            version: entry.version,
            installedAt: Date.now(),
          };
          if (existing >= 0) installed[existing] = record;
          else installed.push(record);
        });
        return operation.handle;
      },
    },
    operations: {
      async cancel(operationId) {
        const operation = operations.get(operationId);
        if (!operation)
          throw new GatewayError("unknownOperation", "operation not found");
        if (operation.finished)
          throw new GatewayError(
            "alreadyFinished",
            "operation already finished",
          );
        if (operation.currentStep && !operation.currentStep.cancellable) {
          throw new GatewayError(
            "notCancellable",
            "current step cannot be cancelled",
          );
        }
        operation.cancelRequested = true;
        modeWaiters.notify(device?.mode ?? "normal");
      },
      onEvent: (handler) => operationEvents.subscribe(handler),
    },
    logs: {
      async list() {
        return logs.slice();
      },
      async export() {
        await sleep(400);
        const path = `~/Library/Logs/PocketStudio/session-${new Date().toISOString().slice(0, 10)}.log`;
        log(
          "info",
          "system",
          "log.system.exported",
          `log exported to ${path}`,
          { path },
        );
        return path;
      },
      onEntry: (handler) => logEvents.subscribe(handler),
    },
    demo: {
      async attachDevice(model = "ipod4") {
        if (device) return;
        device = { ...(model === "n3dsll" ? demoDevice3ds : demoDevice) };
        deviceEvents.emit({ type: "attached", device });
        log(
          "info",
          "device",
          "log.device.attached",
          `device attached over ${device.transport}`,
          { model: device.modelIdentifier ?? model },
        );
      },
      async detachDevice() {
        if (!device) return;
        const id = device.id;
        plans.clear();
        for (const operation of operations.values()) {
          if (!operation.finished) operation.disconnected = true;
        }
        device = null;
        modeWaiters.notify("normal");
        deviceEvents.emit({ type: "detached", deviceId: id });
        log("warn", "device", "log.device.detached", "device detached");
      },
      async setDeviceMode(mode) {
        if (!device || device.mode === mode) return;
        device = { ...device, mode };
        deviceEvents.emit({ type: "modeChanged", deviceId: device.id, mode });
        modeWaiters.notify(mode);
        log(
          "info",
          "device",
          "log.device.modeChanged",
          `device mode is now ${mode}`,
          { mode },
        );
      },
      async setJailbroken(value) {
        jailbroken = value;
        log(
          "debug",
          "system",
          "log.demo.jailbrokenSet",
          `demo: jailbroken=${value}`,
          { value: String(value) },
        );
      },
      async setCfwInstalled(value) {
        cfwInstalled = value;
        log(
          "debug",
          "system",
          "log.demo.cfwSet",
          `demo: cfwInstalled=${value}`,
          { value: String(value) },
        );
      },
      async failNextStep(stepId) {
        failNext = stepId;
        log(
          "debug",
          "system",
          "log.demo.failureArmed",
          `demo: failure armed for ${stepId ?? "none"}`,
          { step: stepId ?? "none" },
        );
      },
    },
  };

  return gateway;
}
