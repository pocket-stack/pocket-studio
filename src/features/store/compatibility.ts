import { compareVersions } from "../../shared/versions";
export { compareVersions } from "../../shared/versions";
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

export function evaluateCompatibility(
  entry: CatalogEntry,
  device: DeviceSummary | null,
  readiness: ReadinessReport | null,
): CompatibilityVerdict {
  if (!device || !device.modelIdentifier || !device.osVersion)
    return "noDevice";
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
