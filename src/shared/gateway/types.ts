/**
 * Data contracts shared with the native layer.
 *
 * Every type here mirrors a serde model in `src-tauri/src/domain`. Field names
 * use camelCase on the wire. Keep the two sides in sync when changing a shape.
 */

export type Platform = "ios" | "3ds";
export type DeviceMode = "normal" | "recovery" | "dfu" | "wtf" | "kis";
export type Transport = "usb" | "network";

export interface DeviceSummary {
  id: string;
  platform: Platform;
  threeDs?: ThreeDsDetails | null;
  modelIdentifier: string | null;
  marketingName: string;
  chip: string | null;
  boardConfig: string | null;
  osVersion: string | null;
  buildNumber: string | null;
  udidMasked: string | null;
  ecidMasked: string | null;
  serialMasked: string | null;
  storageGb: number | null;
  storageTotalBytes: number | null;
  storageFreeBytes: number | null;
  batteryPercent: number | null;
  mode: DeviceMode;
  transport: Transport;
}

export type DiscoveryIssueCode =
  | "usbUnavailable"
  | "macosMuxUnavailable"
  | "linuxMuxUnavailable"
  | "windowsMuxUnavailable"
  | "deviceInfoUnavailable"
  | "pairingUnavailable"
  | "pairingSessionFailed"
  | "probeTimeout";
export interface DiscoveryIssue {
  code: DiscoveryIssueCode;
  deviceId: string | null;
}
export interface DiscoverySnapshot {
  revision: number;
  devices: DeviceSummary[];
  reports: ReadinessReport[];
  issues: DiscoveryIssue[];
  checkedAt: number;
}

export type DeviceEvent =
  | { type: "snapshot"; snapshot: DiscoverySnapshot }
  | { type: "attached"; device: DeviceSummary }
  | { type: "detached"; deviceId: string }
  | { type: "modeChanged"; deviceId: string; mode: DeviceMode };

export type ReadinessCheckId =
  | "platformSupported"
  | "modelSupported"
  | "osVersionSupported"
  | "pairingTrusted"
  | "jailbroken"
  | "appSyncInstalled"
  | "sshAvailable"
  | "batteryLevel"
  | "physicalButtons"
  | "normalMode"
  | "cfwInstalled"
  | "runtimeAvailable";

export type CheckStatus = "pass" | "fail" | "warn" | "unknown";

export interface ReadinessCheck {
  previousObservation?: { installed: boolean; observedAt: number };
  id: ReadinessCheckId;
  status: CheckStatus;
  /** Free-form value rendered next to the check, e.g. a version or percentage. */
  value?: string;
}

export type ReadinessStatus =
  "ready" | "needsPreparation" | "needsAttention" | "unsupported";
export type WorkflowKind = "jailbreak" | "appSync";

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
  | "verifyJailbreak"
  | "connectAppSync"
  | "installAppSync"
  | "activateAppSync"
  | "verifyAppSync";

export type InstallStepId =
  | "resolve"
  | "download"
  | "verify"
  | "transfer"
  | "install"
  | "uninstall"
  | "verifyInstall";

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
  systemPackages?: { name: string; version: string }[];
  id: string;
  deviceId: string;
  entryMode: "normal" | "dfu";
  workflow: WorkflowKind;
  method: "ramdisk" | "ssh";
  exploit: "limera1n" | null;
  targetOsVersion: string;
  dataLoss: "none" | "full";
  tether: "untethered" | "semiTethered" | "tethered" | null;
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

export type OperationKind = "preparation" | "install" | "uninstall";
export type StepStatus =
  "pending" | "running" | "done" | "failed" | "skipped" | "cancelled";

export type OperationErrorCode =
  | "deviceChanged"
  | "alreadyJailbroken"
  | "rebootTimeout"
  | "dfuTimeout"
  | "exploitFailed"
  | "downloadFailed"
  | "buildFailed"
  | "ramdiskBootFailed"
  | "sshUnavailable"
  | "writeFailed"
  | "deviceDisconnected"
  | "verificationFailed"
  | "verificationUnavailable"
  | "checksumMismatch"
  | "transferFailed"
  | "appSyncInstallFailed"
  | "appSyncRestartRequired"
  | "appSyncDependencies"
  | "appSyncVerificationFailed"
  | "sshAuthenticationFailed"
  | "installRejected"
  | "cancelled";

export interface OperationError {
  code: OperationErrorCode;
  recoverable: boolean;
  retryFromStepId?: StepId;
  diagnostic?: { stage: string; reason: string };
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
export type InstallPolicy =
  "deb" | "ipa" | "bootstrap" | "unsupported" | "cia" | "threeDsx" | "pocket";
export type SignedCatalog = import("./storeProtocol.generated").SignedCatalog;
export type StoreApplication = SignedCatalog["apps"][number];
export type StoreRelease = SignedCatalog["releases"][number];
export type StoreArtifact = StoreRelease["artifacts"][number];
export type StoreVerdict =
  | "compatible"
  | "requiresPreparation"
  | "unsupportedModel"
  | "unsupportedOs"
  | "unsupportedInstaller"
  | "unknownDevice"
  | "noDevice"
  | "withdrawn"
  | "catalogExpired";
export interface CatalogDetails {
  candidates: CatalogCandidate[];
  app: StoreApplication;
  releaseId: string;
  artifactId: string;
  targetId: string;
  revision: number;
  nativeIdentity: StoreArtifact["native_identity"];
  verdict: StoreVerdict;
  history: StoreRelease[];
}
export interface CatalogSnapshot {
  entries: CatalogEntry[];
  source: "network" | "cache" | "unconfigured" | "demo";
  sourceLabel: string | null;
  sequence: number | null;
  expiresAt: number | null;
  checkedAt: number | null;
  expired: boolean;
  verified: boolean;
  issue: string | null;
}

export interface PackageCompatibility {
  platform: string;
  models: string[];
  minOsVersion: string;
  maxOsVersion: string | null;
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
  details?: CatalogDetails;
}

export interface InstalledPackage {
  installationId: string;
  managed?: ThreeDsInstallation | null;
  packageId: string;
  version: string;
  installedAt: number | null;
  native?: {
    bundleId: string;
    productVersion: string | null;
    buildNumber: string | null;
    applicationType: string | null;
    receiptBuildId: string | null;
  } | null;
  releaseId?: string | null;
  artifactId?: string | null;
  revision?: number | null;
}

export interface InstalledSnapshot {
  deviceId: string;
  entries: InstalledPackage[];
  state: "fresh" | "stale" | "unavailable";
  observedAt: number | null;
  issue: string | null;
}

export type PackageAction = "install" | "update" | "reinstall" | "uninstall";
export type RequirementState = "satisfied" | "missing" | "unknown";
export interface PackageRequest {
  deviceId: string;
  appId: string;
  action: PackageAction;
  installationId?: string | null;
  delivery?: "shared" | "bundled" | null;
  format?: string | null;
  deleteData?: boolean | null;
}
export interface PackageConsent {
  planId: string;
  deleteData: boolean;
}
export interface PackagePlan {
  id: string;
  deviceId: string;
  deviceName: string;
  appId: string;
  names: Record<string, string>;
  action: PackageAction;
  installation: PackageInstallation;
  deleteData: boolean;
  releaseId: string | null;
  artifact: StoreArtifact | null;
  target: StoreArtifact["targets"][number] | null;
  version: string | null;
  revision: number | null;
  publicationId: string;
  sequence: number;
  catalogExpiresAt: number;
  expiresAt: number;
  steps: PlanStep[];
}
export interface PackageJob {
  queueOrder: number;
  handle: OperationHandle;
  plan: PackagePlan;
  phase:
    | "queued"
    | "running"
    | "verifying"
    | "verified"
    | "unverified"
    | "failed"
    | "cancelled"
    | "interrupted";
  step: StepId;
  completedSteps: StepId[];
  percent: number;
  submitted: boolean;
  cleanupComplete: boolean | null;
  stagingPath: string | null;
  error: OperationError | null;
  updatedAt: number;
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

export type DemoDeviceModel = "ipod4" | "n3dsll";
export interface DemoControls {
  /** One simulated device at a time; the iPod touch is the default. */
  attachDevice(model?: DemoDeviceModel): Promise<void>;
  detachDevice(): Promise<void>;
  setDeviceMode(mode: DeviceMode): Promise<void>;
  setJailbroken(jailbroken: boolean): Promise<void>;
  /** 3DS: whether custom firmware (Luma3DS) is detected. */
  setCfwInstalled(installed: boolean): Promise<void>;
  /** Make the next run of `stepId` fail; `null` clears the injection. */
  failNextStep(stepId: StepId | null): Promise<void>;
}

export interface StudioGateway {
  readonly flavor: "tauri" | "browser";
  readonly capabilities: {
    demo: boolean;
    preparation: boolean;
    catalog: boolean;
    packages: boolean;
    installed: boolean;
  };
  devices: {
    list(): Promise<DiscoverySnapshot>;
    checkReadiness(deviceId: string): Promise<ReadinessReport>;
    checkAppSync(
      deviceId: string,
      sshPassword: string,
    ): Promise<ReadinessReport>;
    onEvent(
      handler: (event: DeviceEvent) => void,
    ): Unsubscribe | Promise<Unsubscribe>;
  };
  setup: {
    plan(request: SetupRequest): Promise<SetupPlan>;
    execute(planId: string): Promise<SetupResult>;
    connect(pairingId: string, address?: string): Promise<DiscoverySnapshot>;
    /** Console address from Preferences; every scan asks it directly. */
    hint(address: string | null): Promise<void>;
  };
  preparation: {
    plan(deviceId: string): Promise<PreparationPlan>;
    start(
      consent: ConsentRecord,
      sshPassword: string,
    ): Promise<OperationHandle>;
  };
  store: {
    catalog(deviceId?: string, refresh?: boolean): Promise<CatalogSnapshot>;
    media(sha256: string): Promise<string>;
    installed(deviceId: string): Promise<InstalledSnapshot>;
    plan(request: PackageRequest): Promise<PackagePlan>;
    start(consent: PackageConsent): Promise<OperationHandle>;
    jobs(): Promise<PackageJob[]>;
    verify(operationId: string, deviceId: string): Promise<OperationHandle>;
    uninstall(deviceId: string, packageId: string): Promise<void>;
    install(deviceId: string, packageId: string): Promise<OperationHandle>;
  };
  operations: {
    cancel(operationId: string): Promise<void>;
    onEvent(
      handler: (event: OperationEvent) => void,
    ): Unsubscribe | Promise<Unsubscribe>;
  };
  logs: {
    list(): Promise<LogEntry[]>;
    export(): Promise<string>;
    onEntry(
      handler: (entry: LogEntry) => void,
    ): Unsubscribe | Promise<Unsubscribe>;
  };
  demo: DemoControls;
}

export type StoreTarget = StoreArtifact["targets"][number];
export type RuntimeDelivery = "shared" | "bundled";
export interface ThreeDsDetails {
  region: string | null;
  firmwareRevision: number | null;
  firmware: string | null;
  runtime: NonNullable<StoreTarget["runtime_provides"]>;
  hostAbi: number;
  hostAppId: string;
  launcher: boolean;
  busy: boolean;
  nativeManagement: boolean;
  hardwareVerified: boolean;
}
export interface ThreeDsInstallation {
  installationId: string;
  appId: string;
  containerId: string;
  generation: number;
  format: string;
  delivery: RuntimeDelivery;
  installed: boolean;
  health: "untested" | "accepted" | "rejected";
  title: string;
  version: string;
  revision: number | null;
  buildId: string | null;
  guestSha256: string;
  nativeVersion: string | null;
  nativeBuildId: string | null;
  nativeIdentity: StoreArtifact["native_identity"];
  runtimeId: string | null;
  runtimeVersion: string | null;
  hostAbi: number | null;
  unavailable: boolean;
}
export type PackageInstallation =
  | {
      platform: "ios";
      bundleId: string;
      previous: NonNullable<InstalledPackage["native"]> | null;
      appsync: RequirementState;
      jailbreak: RequirementState;
    }
  | {
      platform: "3ds";
      installationId: string;
      delivery: RuntimeDelivery;
      format: string;
      expectedGeneration: number;
      previous: ThreeDsInstallation | null;
      nativeIdentity: StoreArtifact["native_identity"];
      updatesHost: boolean;
    };
export interface CatalogCandidate {
  delivery: RuntimeDelivery;
  format: string;
  version: string;
  revision: number;
  releaseId: string;
  artifactId: string;
  targetId: string;
  verdict: StoreVerdict;
  requiresExistingHost: boolean;
  runtimeRequirement: StoreTarget["runtime_requirement"];
  hostAbi: number | null;
}
export type SetupDestination =
  | { kind: "sd"; path: string }
  | {
      kind: "ftp";
      address: string;
      port: number;
      username: string;
      password: string;
    };
export interface SetupRequest {
  runtimeRequirement?: StoreTarget["runtime_requirement"];
  hostAbi?: number | null;
  destination: SetupDestination;
  address?: string;
  format?: "cia" | "3dsx";
  /** Copy this listed title to the card instead of the Pocket launcher. */
  appId?: string;
}
export interface SetupPlan {
  appId: string;
  id: string;
  destination: string;
  existingPairing: boolean;
  artifact: StoreArtifact | null;
  version: string | null;
  files: string[];
  expiresAt: number;
}
export interface SetupResult {
  pairingId: string;
  filesVerified: boolean;
  restartRequired: boolean;
}
