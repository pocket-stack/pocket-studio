import { mkdir, readFile, writeFile, copyFile } from "node:fs/promises";
import { resolve } from "node:path";
import { parseArgs } from "node:util";
import { URL } from "node:url";
import { compile } from "json-schema-to-typescript";
import { format } from "prettier";

const { values } = parseArgs({ options: { source: { type: "string" } } });
const directory = new URL("../contracts/store-v1/", import.meta.url);
await mkdir(directory, { recursive: true });
if (values.source) {
  for (const file of [
    "catalog-v1.schema.json",
    "pointer-v1.schema.json",
    "trust-v1.schema.json",
    "fixture-v1.json",
  ]) {
    await copyFile(resolve(values.source, file), new URL(file, directory));
  }
}
const schema = JSON.parse(
  await readFile(new URL("catalog-v1.schema.json", directory), "utf8"),
);
const types = await compile(schema, "SignedCatalog", {
  bannerComment:
    "/** Generated from contracts/store-v1. Run pnpm sync:store-protocol. */",
  unknownAny: true,
});
await writeFile(
  new URL("../src/shared/gateway/storeProtocol.generated.ts", import.meta.url),
  await format(types, { parser: "typescript", printWidth: 80 }),
);
