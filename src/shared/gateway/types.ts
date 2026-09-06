/**
 * Data contracts shared with the native layer.
 *
 * Every type here mirrors a serde model in `src-tauri/src/domain`. Field names
 * use camelCase on the wire. Keep the two sides in sync when changing a shape.
 */

export type Platform = "ios";
export type DeviceMode = "normal" | "recovery" | "dfu";
export type Transport = "usb" | "network";

export interface DeviceSummary {
  id: string;
  platform: Platform;
  modelIdentifier: string;
  marketingName: string;
  chip: string;
  boardConfig: string;
  osVersion: string;
  buildNumber: string;
  udidMasked: string;
  ecidMasked: string;
  serialMasked: string;
  storageGb: number;
  batteryPercent: number;
  mode: DeviceMode;
  transport: Transport;
}

export type DeviceEvent =
  | { type: "attached"; device: DeviceSummary }
  | { type: "detached"; deviceId: string }
  | { type: "modeChanged"; deviceId: string; mode: DeviceMode };

export type ReadinessCheckId =
  | "platformSupported"
  | "modelSupported"
  | "osVersionSupported"
  | "pairingTrusted"
  | "jailbroken"
  | "sshAvailable"
  | "batteryLevel"
  | "physicalButtons";

export type CheckStatus = "pass" | "fail" | "warn" | "unknown";

export interface ReadinessCheck {
  id: ReadinessCheckId;
  status: CheckStatus;
  /** Free-form value rendered next to the check, e.g. a version or percentage. */
  value?: string;
}

export type ReadinessStatus = "ready" | "needsPreparation" | "unsupported";
export type WorkflowKind = "jailbreak";

export interface ReadinessReport {
  deviceId: string;
  status: ReadinessStatus;
  checks: ReadinessCheck[];
  requiredWorkflow?: WorkflowKind;
  checkedAt: number;
}

export type RiskSeverity = "high" | "medium" | "low";

export type RiskId =
  | "bootloop"
  | "dataLossOnRecovery"
  | "interruptionHazard"
  | "securityPosture"
  | "untetherStability"
  | "communityTooling"
  | "warrantyAndSupport";

export interface Risk {
  id: RiskId;
  severity: RiskSeverity;
}

export type PrerequisiteId =
  | "batteryAbove50"
  | "workingButtons"
  | "backupCompleted"
  | "stableCable"
  | "computerAwake";

export type PreparationStepId =
  | "enterDfu"
  | "exploitBootrom"
  | "fetchResources"
  | "buildRamdisk"
  | "bootRamdisk"
  | "mountFilesystem"
  | "installUntether"
  | "rebootDevice"
  | "verifyJailbreak";

export type InstallStepId =
  "resolve" | "download" | "verify" | "transfer" | "install" | "verifyInstall";

export type StepId = PreparationStepId | InstallStepId;

export type RequiredAction = "enterDfu" | "reconnect" | "trustComputer";

export interface PlanStep {
  id: StepId;
  cancellable: boolean;
  pointOfNoReturn: boolean;
  estimatedSeconds: number;
  requiresAction?: RequiredAction;
}

export interface PreparationPlan {
  id: string;
  deviceId: string;
  workflow: WorkflowKind;
  method: "ramdisk";
  exploit: "limera1n";
  targetOsVersion: string;
  dataLoss: "none" | "full";
  tether: "untethered" | "semiTethered" | "tethered";
  prerequisites: PrerequisiteId[];
  risks: Risk[];
  disclaimerVersion: string;
  minimumReadingSeconds: { risks: number; disclaimer: number };
  steps: PlanStep[];
}

export interface ConsentRecord {
  planId: string;
  prerequisitesConfirmed: PrerequisiteId[];
  acknowledgedRiskIds: RiskId[];
  riskReadingSeconds: number;
  risksAcknowledgedAt: number;
  disclaimerVersion: string;
  disclaimerReadingSeconds: number;
  disclaimerAcceptedAt: number;
}

export type OperationKind = "preparation" | "install";
export type StepStatus =
  "pending" | "running" | "done" | "failed" | "skipped" | "cancelled";

export type OperationErrorCode =
  | "dfuTimeout"
  | "exploitFailed"
  | "downloadFailed"
  | "buildFailed"
  | "ramdiskBootFailed"
  | "sshUnavailable"
  | "writeFailed"
  | "deviceDisconnected"
  | "verificationFailed"
  | "checksumMismatch"
  | "transferFailed"
  | "installRejected"
  | "cancelled";

export interface OperationError {
  code: OperationErrorCode;
  recoverable: boolean;
  retryFromStepId?: StepId;
}

export interface OperationHandle {
  operationId: string;
  kind: OperationKind;
  steps: PlanStep[];
  /** Package id for install operations. */
  subject?: string;
}

export type OperationEvent =
  | { type: "started"; operationId: string; kind: OperationKind }
  | {
      type: "stepChanged";
      operationId: string;
      stepId: StepId;
      status: StepStatus;
    }
  | {
      type: "progress";
      operationId: string;
      stepId: StepId;
      percent: number;
      detail?: string;
    }
  | {
      type: "actionRequired";
      operationId: string;
      stepId: StepId;
      action: RequiredAction;
    }
  | { type: "actionResolved"; operationId: string; stepId: StepId }
  | { type: "finished"; operationId: string }
  | {
      type: "failed";
      operationId: string;
      stepId: StepId;
      error: OperationError;
    }
  | { type: "cancelled"; operationId: string; stepId: StepId };

export type PackageCategory = "runtime" | "tool" | "app" | "game";
export type InstallPolicy = "deb" | "ipa" | "bootstrap";

export interface PackageCompatibility {
  platform: Platform;
  models: string[];
  minOsVersion: string;
  maxOsVersion: string;
  requiresJailbreak: boolean;
}

export interface CatalogEntry {
  id: string;
  version: string;
  developer: string;
  category: PackageCategory;
  sizeBytes: number;
  installPolicy: InstallPolicy;
  checksumSha256: string;
  signed: boolean;
  compatibility: PackageCompatibility;
  dependencies: string[];
  publishedAt: number;
}

export interface InstalledPackage {
  packageId: string;
  version: string;
  installedAt: number;
}

export type LogLevel = "error" | "warn" | "info" | "debug";
export type LogSource = "device" | "preparation" | "store" | "system";

export interface LogEntry {
  id: string;
  timestamp: number;
  level: LogLevel;
  source: LogSource;
  /** Stable code the UI translates; `message` is the English fallback. */
  code: string;
  message: string;
  params?: Record<string, string>;
  operationId?: string;
}

export type CancelErrorCode =
  "notCancellable" | "unknownOperation" | "alreadyFinished";

export class GatewayError extends Error {
  constructor(
    readonly code: string,
    message: string,
  ) {
    super(message);
    this.name = "GatewayError";
  }
}

export type Unsubscribe = () => void;

export interface DemoControls {
  attachDevice(): Promise<void>;
  detachDevice(): Promise<void>;
  setDeviceMode(mode: DeviceMode): Promise<void>;
  setJailbroken(jailbroken: boolean): Promise<void>;
  /** Make the next run of `stepId` fail; `null` clears the injection. */
  failNextStep(stepId: StepId | null): Promise<void>;
}

export interface StudioGateway {
  readonly flavor: "tauri" | "browser";
  devices: {
    list(): Promise<DeviceSummary[]>;
    checkReadiness(deviceId: string): Promise<ReadinessReport>;
    onEvent(handler: (event: DeviceEvent) => void): Unsubscribe;
  };
  preparation: {
    plan(deviceId: string): Promise<PreparationPlan>;
    start(consent: ConsentRecord): Promise<OperationHandle>;
  };
  store: {
    catalog(): Promise<CatalogEntry[]>;
    installed(deviceId: string): Promise<InstalledPackage[]>;
    uninstall(deviceId: string, packageId: string): Promise<void>;
    install(deviceId: string, packageId: string): Promise<OperationHandle>;
  };
  operations: {
    cancel(operationId: string): Promise<void>;
    onEvent(handler: (event: OperationEvent) => void): Unsubscribe;
  };
  logs: {
    list(): Promise<LogEntry[]>;
    export(): Promise<string>;
    onEntry(handler: (entry: LogEntry) => void): Unsubscribe;
  };
  demo: DemoControls;
}
