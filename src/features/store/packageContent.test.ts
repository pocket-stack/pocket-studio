import { expect, it } from "vitest";
import fixture from "../../../contracts/store-v1/fixture-v1.json";
import { demoCatalog } from "../../shared/gateway/fixtures";
import type { CatalogEntry, SignedCatalog } from "../../shared/gateway";
import { packageMedia, packageText } from "./packageContent";

function entry(): CatalogEntry {
  const catalog = JSON.parse(fixture.catalog_utf8) as SignedCatalog;
  const app = catalog.apps[0]!,
    release = catalog.releases[0]!,
    artifact = release.artifacts[0];
  return {
    ...demoCatalog[0]!,
    id: app.id,
    details: {
      candidates: [],
      app,
      releaseId: release.id,
      artifactId: artifact.id,
      targetId: artifact.targets[0].target_id,
      revision: release.revision,
      nativeIdentity: artifact.native_identity,
      verdict: "noDevice",
      history: catalog.releases,
    },
  };
}
it("displays new catalog applications without compiled application locale keys", () => {
  const app = entry();
  const fallback = () => {
    throw new Error(
      "catalog metadata must not use compiled application strings",
    );
  };
  expect(packageText(app, "name", "zh-CN", fallback)).toBe("笔记");
  expect(packageText(app, "summary", "fr", fallback)).toBe("Take notes");
  app.id = "dev.example.another";
  app.details!.app.locales.en!.name = "Another application";
  expect(packageText(app, "name", "en", fallback)).toBe("Another application");
});
it("selects target and locale variants without duplicating screenshot positions", () => {
  const app = entry(),
    target = app.details!.targetId;
  const media = (
    sha: string,
    locale: string | null,
    target_id: string | null,
    sort_order: number,
  ) => ({
    role: "screenshot" as const,
    locale,
    target_id,
    sort_order,
    blob: { sha256: sha.repeat(64), size_bytes: 8, content_type: "image/png" },
  });
  app.details!.app.media = [
    media("a", "en", null, 0),
    media("b", "zh-CN", target, 0),
    media("c", "en", target, 1),
    media("d", "zh-CN", "another-target", 2),
  ];
  expect(
    packageMedia(app, "screenshot", "zh-CN").map((m) => m.blob.sha256),
  ).toEqual(["b".repeat(64), "c".repeat(64)]);
});
