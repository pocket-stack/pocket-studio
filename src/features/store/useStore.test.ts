import { afterEach, expect, it, vi } from "vitest";
vi.mock("vue-i18n", () => ({
  useI18n: () => ({ t: (key: string) => key, locale: { value: "en" } }),
}));
afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});
it("serializes installs and makes queued apps visible without duplicate requests", async () => {
  vi.resetModules();
  vi.useFakeTimers();
  vi.stubGlobal("window", { setTimeout, clearTimeout });
  const { useGateway } = await import("../../shared/gateway");
  const { useDeviceSession } =
    await import("../../shared/composables/useDeviceSession");
  const { useStore } = await import("./useStore");
  const gateway = useGateway();
  const session = useDeviceSession();
  await session.initialize();
  await gateway.demo.attachDevice();
  await gateway.demo.setJailbroken(true);
  await vi.advanceTimersByTimeAsync(600);
  const store = useStore();
  const init = store.initialize();
  await vi.advanceTimersByTimeAsync(300);
  await init;
  await store.install("pocket-runtime");
  await store.install("pocket-reader");
  await store.install("pocket-reader");
  expect(store.queuedIds.value).toEqual(["pocket-reader"]);
  expect(
    store.allPackages.value.find((item) => item.entry.id === "pocket-reader")
      ?.queuePosition,
  ).toBe(1);
  await vi.advanceTimersByTimeAsync(20_000);
  expect(store.queuedIds.value).toEqual([]);
  expect(store.installed.value.map((item) => item.packageId)).toEqual([
    "pocket-runtime",
    "pocket-reader",
  ]);
});
