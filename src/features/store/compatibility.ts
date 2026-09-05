import type {
  CatalogEntry,
  DeviceSummary,
  ReadinessReport,
} from "../../shared/gateway";

export type CompatibilityVerdict =
  | "compatible"
  | "requiresPreparation"
  | "unsupportedModel"
  | "unsupportedOs"
  | "noDevice";

export function compareVersions(a: string, b: string): number {
  const left = a.split(".").map(Number);
  const right = b.split(".").map(Number);
  const length = Math.max(left.length, right.length);
  for (let index = 0; index < length; index += 1) {
    const diff = (left[index] ?? 0) - (right[index] ?? 0);
    if (diff !== 0) return Math.sign(diff);
  }
  return 0;
}

export function evaluateCompatibility(
  entry: CatalogEntry,
  device: DeviceSummary | null,
  readiness: ReadinessReport | null,
): CompatibilityVerdict {
  if (!device) return "noDevice";
  const { compatibility } = entry;
  if (
    compatibility.platform !== device.platform ||
    !compatibility.models.includes(device.modelIdentifier)
  ) {
    return "unsupportedModel";
  }
  if (
    compareVersions(device.osVersion, compatibility.minOsVersion) < 0 ||
    compareVersions(device.osVersion, compatibility.maxOsVersion) > 0
  ) {
    return "unsupportedOs";
  }
  if (compatibility.requiresJailbreak && readiness?.status !== "ready") {
    return "requiresPreparation";
  }
  return "compatible";
}

export function formatBytes(bytes: number): string {
  if (bytes >= 1_000_000) return `${(bytes / 1_000_000).toFixed(1)} MB`;
  if (bytes >= 1_000) return `${Math.round(bytes / 1_000)} KB`;
  return `${bytes} B`;
}
