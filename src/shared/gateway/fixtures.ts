import type {
  CatalogEntry,
  DeviceSummary,
  PlanStep,
  PreparationPlan,
  PreparationStepId,
  Risk,
} from "./types";

/**
 * Browser-only fixtures. Native windows use the real Legacy iOS Kit adapter.
 */

export const demoDevice = {
  id: "usb-ipod4-demo",
  platform: "ios",
  modelIdentifier: "iPod4,1",
  marketingName: "iPod touch (4th generation)",
  chip: "Apple A4 (S5L8930)",
  boardConfig: "N81AP",
  osVersion: "6.1.6",
  buildNumber: "10B500",
  udidMasked: "1a2b3c…9f0e",
  ecidMasked: "0000…A7F2",
  serialMasked: "C3T…P4",
  storageGb: 32,
  storageTotalBytes: 32_000_000_000,
  storageFreeBytes: 19_400_000_000,
  batteryPercent: 78,
  mode: "normal",
  transport: "usb",
} satisfies DeviceSummary;

export const jailbreakRisks: Risk[] = [
  { id: "bootloop", severity: "high" },
  { id: "dataLossOnRecovery", severity: "high" },
  { id: "interruptionHazard", severity: "high" },
  { id: "securityPosture", severity: "medium" },
  { id: "communityTooling", severity: "medium" },
  { id: "untetherStability", severity: "low" },
  { id: "warrantyAndSupport", severity: "low" },
];

export const jailbreakSteps: PlanStep[] = [
  {
    id: "enterDfu",
    cancellable: true,
    pointOfNoReturn: false,
    estimatedSeconds: 60,
    requiresAction: "enterDfu",
  },
  {
    id: "exploitBootrom",
    cancellable: true,
    pointOfNoReturn: false,
    estimatedSeconds: 10,
  },
  {
    id: "fetchResources",
    cancellable: true,
    pointOfNoReturn: false,
    estimatedSeconds: 25,
  },
  {
    id: "buildRamdisk",
    cancellable: true,
    pointOfNoReturn: false,
    estimatedSeconds: 15,
  },
  {
    id: "bootRamdisk",
    cancellable: false,
    pointOfNoReturn: false,
    estimatedSeconds: 15,
  },
  {
    id: "mountFilesystem",
    cancellable: false,
    pointOfNoReturn: false,
    estimatedSeconds: 8,
  },
  {
    id: "installUntether",
    cancellable: false,
    pointOfNoReturn: true,
    estimatedSeconds: 40,
  },
  {
    id: "rebootDevice",
    cancellable: false,
    pointOfNoReturn: false,
    estimatedSeconds: 20,
  },
  {
    id: "verifyJailbreak",
    cancellable: false,
    pointOfNoReturn: false,
    estimatedSeconds: 20,
  },
];

export const installSteps: PlanStep[] = [
  {
    id: "resolve",
    cancellable: true,
    pointOfNoReturn: false,
    estimatedSeconds: 2,
  },
  {
    id: "download",
    cancellable: true,
    pointOfNoReturn: false,
    estimatedSeconds: 12,
  },
  {
    id: "verify",
    cancellable: true,
    pointOfNoReturn: false,
    estimatedSeconds: 3,
  },
  {
    id: "transfer",
    cancellable: true,
    pointOfNoReturn: false,
    estimatedSeconds: 8,
  },
  {
    id: "install",
    cancellable: false,
    pointOfNoReturn: true,
    estimatedSeconds: 10,
  },
  {
    id: "verifyInstall",
    cancellable: false,
    pointOfNoReturn: false,
    estimatedSeconds: 3,
  },
];

export const disclaimerVersion = "2026-09-07-appsync-1";

export function buildJailbreakPlan(
  deviceId: string,
  sequence: number,
  entryMode: PreparationPlan["entryMode"] = "normal",
): PreparationPlan {
  return {
    id: `plan-${deviceId}-${sequence}`,
    deviceId,
    entryMode,
    workflow: "jailbreak",
    method: "ramdisk",
    exploit: "limera1n",
    targetOsVersion: demoDevice.osVersion,
    dataLoss: "none",
    tether: "untethered",
    prerequisites: [
      "batteryAbove50",
      ...(entryMode === "normal" ? (["workingButtons"] as const) : []),
      "backupCompleted",
      "stableCable",
      "computerAwake",
    ],
    risks: jailbreakRisks,
    disclaimerVersion,
    minimumReadingSeconds: { risks: 5, disclaimer: 5 },
    steps: jailbreakSteps.filter(
      (step) => entryMode !== "dfu" || step.id !== "enterDfu",
    ),
  };
}

export const preparationFailureCodes: Record<
  PreparationStepId,
  {
    code: import("./types").OperationErrorCode;
    recoverable: boolean;
    retryFrom?: PreparationStepId;
  }
> = {
  connectAppSync: { code: "sshAuthenticationFailed", recoverable: false },
  installAppSync: { code: "appSyncInstallFailed", recoverable: false },
  activateAppSync: { code: "appSyncRestartRequired", recoverable: false },
  verifyAppSync: { code: "appSyncVerificationFailed", recoverable: false },

  enterDfu: { code: "dfuTimeout", recoverable: true, retryFrom: "enterDfu" },
  exploitBootrom: {
    code: "exploitFailed",
    recoverable: true,
    retryFrom: "enterDfu",
  },
  fetchResources: {
    code: "downloadFailed",
    recoverable: true,
    retryFrom: "fetchResources",
  },
  buildRamdisk: {
    code: "buildFailed",
    recoverable: true,
    retryFrom: "fetchResources",
  },
  bootRamdisk: {
    code: "ramdiskBootFailed",
    recoverable: true,
    retryFrom: "enterDfu",
  },
  mountFilesystem: {
    code: "sshUnavailable",
    recoverable: true,
    retryFrom: "enterDfu",
  },
  installUntether: { code: "writeFailed", recoverable: false },
  rebootDevice: {
    code: "deviceDisconnected",
    recoverable: true,
    retryFrom: "verifyJailbreak",
  },
  verifyJailbreak: {
    code: "verificationFailed",
    recoverable: true,
    retryFrom: "enterDfu",
  },
};

const ios6Compat = {
  platform: "ios" as const,
  models: ["iPod4,1", "iPhone3,1", "iPhone3,3", "iPad1,1"],
  minOsVersion: "6.0",
  maxOsVersion: "6.1.6",
};

export const demoCatalog: CatalogEntry[] = [
  {
    id: "pocket-runtime",
    version: "0.9.3",
    developer: "PocketJS",
    category: "runtime",
    sizeBytes: 6_720_000,
    installPolicy: "deb",
    checksumSha256: "3f9c…b21e",
    signed: true,
    compatibility: { ...ios6Compat, requiresJailbreak: true },
    dependencies: [],
    publishedAt: Date.UTC(2026, 7, 21),
  },
  {
    id: "pocket-agent",
    version: "0.4.0",
    developer: "PocketJS",
    category: "tool",
    sizeBytes: 1_180_000,
    installPolicy: "deb",
    checksumSha256: "9a71…04cd",
    signed: true,
    compatibility: { ...ios6Compat, requiresJailbreak: true },
    dependencies: ["pocket-runtime"],
    publishedAt: Date.UTC(2026, 7, 28),
  },
  {
    id: "openssh",
    version: "6.7p1-13",
    developer: "Community (Cydia/Telesphoreo)",
    category: "tool",
    sizeBytes: 2_310_000,
    installPolicy: "deb",
    checksumSha256: "c0de…77aa",
    signed: false,
    compatibility: {
      ...ios6Compat,
      minOsVersion: "3.1.3",
      requiresJailbreak: true,
    },
    dependencies: [],
    publishedAt: Date.UTC(2024, 2, 3),
  },
  {
    id: "legacy-ca-bundle",
    version: "2026.08",
    developer: "Community",
    category: "tool",
    sizeBytes: 410_000,
    installPolicy: "deb",
    checksumSha256: "88ef…1a90",
    signed: true,
    compatibility: {
      ...ios6Compat,
      minOsVersion: "5.0",
      requiresJailbreak: true,
    },
    dependencies: [],
    publishedAt: Date.UTC(2026, 7, 2),
  },
  {
    id: "pocket-reader",
    version: "1.2.0",
    developer: "PocketJS",
    category: "app",
    sizeBytes: 14_500_000,
    installPolicy: "ipa",
    checksumSha256: "51aa…e3f7",
    signed: true,
    compatibility: { ...ios6Compat, requiresJailbreak: false },
    dependencies: [],
    publishedAt: Date.UTC(2026, 6, 15),
  },
  {
    id: "pocket-notes",
    version: "0.8.1",
    developer: "PocketJS",
    category: "app",
    sizeBytes: 9_200_000,
    installPolicy: "ipa",
    checksumSha256: "d4d4…9b0c",
    signed: true,
    compatibility: { ...ios6Compat, requiresJailbreak: false },
    dependencies: ["pocket-runtime"],
    publishedAt: Date.UTC(2026, 7, 9),
  },
  {
    id: "pocket-arcade",
    version: "2.1.4",
    developer: "Retro Pocket Collective",
    category: "game",
    sizeBytes: 38_000_000,
    installPolicy: "deb",
    checksumSha256: "7b7b…c3c3",
    signed: true,
    compatibility: { ...ios6Compat, requiresJailbreak: true },
    dependencies: ["pocket-runtime"],
    publishedAt: Date.UTC(2026, 5, 30),
  },
  {
    id: "pocket-camera-pro",
    version: "3.0.0",
    developer: "PocketJS",
    category: "app",
    sizeBytes: 22_000_000,
    installPolicy: "ipa",
    checksumSha256: "e1e1…5f5f",
    signed: true,
    compatibility: {
      platform: "ios",
      models: ["iPhone4,1", "iPhone5,1", "iPod5,1"],
      minOsVersion: "7.0",
      maxOsVersion: "9.3.6",
      requiresJailbreak: false,
    },
    dependencies: [],
    publishedAt: Date.UTC(2026, 7, 30),
  },
];

// Additional local fixtures fill the two seven-column shelves in the design.
for (const [id, category, dependencies] of [
  ["tunnel-kit", "tool", []],
  ["sensor-tap", "tool", ["pocket-runtime"]],
  ["batt-guard", "tool", []],
  ["clip-sync", "app", ["pocket-runtime"]],
  ["photo-pull", "app", []],
  ["font-patch", "app", []],
] as const) {
  demoCatalog.push({
    id,
    category,
    version: "1.0.0",
    developer: "Pocket Labs",
    sizeBytes: 3_400_000,
    installPolicy: "deb",
    checksumSha256: "demo…0000",
    signed: true,
    compatibility: { ...ios6Compat, requiresJailbreak: true },
    dependencies: [...dependencies],
    publishedAt: Date.UTC(2026, 8, 1),
  });
}
