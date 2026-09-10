import { compareVersions } from "../../shared/versions";
export { compareVersions } from "../../shared/versions";
import type { DeepReadonly } from "vue";
import type {
  CatalogCandidate,
  CatalogEntry,
  DeviceSummary,
  ReadinessReport,
  StoreVerdict,
} from "../../shared/gateway";

export type CompatibilityVerdict = StoreVerdict;

export function evaluateCompatibility(
  entry: CatalogEntry,
  device: DeviceSummary | null,
  readiness: ReadinessReport | null,
): CompatibilityVerdict {
  if (entry.details) return entry.details.verdict;
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
    (compatibility.maxOsVersion !== null &&
      compareVersions(device.osVersion, compatibility.maxOsVersion) > 0)
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

/**
 * Installation forms a 3DS title can be installed in from scratch: one entry
 * per (delivery, format) pair that is still published. A bundled `.pocket`
 * only updates an existing standalone host, so it is not a first-install form.
 */
export function installationForms(
  entry: DeepReadonly<CatalogEntry> | undefined,
): DeepReadonly<CatalogCandidate>[] {
  const forms: DeepReadonly<CatalogCandidate>[] = [];
  for (const candidate of entry?.details?.candidates ?? []) {
    if (
      candidate.requiresExistingHost ||
      candidate.verdict === "withdrawn" ||
      candidate.verdict === "catalogExpired" ||
      forms.some(
        (form) =>
          form.delivery === candidate.delivery &&
          form.format === candidate.format,
      )
    )
      continue;
    forms.push(candidate);
  }
  return forms;
}
