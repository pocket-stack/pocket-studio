import type { CatalogEntry, StoreApplication } from "../../shared/gateway";
import type { DeepReadonly } from "vue";

export type PackageTextField = "name" | "summary" | "description";
export function packageText(
  entry: DeepReadonly<CatalogEntry>,
  field: PackageTextField,
  locale: string,
  fallback: (key: string) => string,
): string {
  const content = entry.details?.app.locales;
  if (content) return (content[locale] ?? content.en)?.[field] ?? entry.id;
  return fallback(`catalog.${entry.id}.${field}`);
}
export function packageMedia(
  entry: DeepReadonly<CatalogEntry>,
  role: "icon" | "screenshot",
  locale: string,
): StoreApplication["media"] {
  const details = entry.details;
  if (!details) return [];
  const target = details.targetId;
  const selected = new Map<number, StoreApplication["media"][number]>();
  const score = (media: StoreApplication["media"][number]) =>
    Number(media.target_id === target) * 8 +
    (media.locale === locale ? 4 : media.locale === "en" ? 2 : 1);
  for (const media of details.app.media) {
    if (
      media.role !== role ||
      (media.locale && media.locale !== locale && media.locale !== "en") ||
      (media.target_id && media.target_id !== target)
    )
      continue;
    const previous = selected.get(media.sort_order);
    if (!previous || score(media) > score(previous))
      selected.set(media.sort_order, media);
  }
  return [...selected.values()].sort((a, b) => a.sort_order - b.sort_order);
}
