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

it("preserves failed observations and rejects late results after device detachment", async () => {
  vi.resetModules();
  vi.useFakeTimers();
  vi.stubGlobal("window", { setTimeout, clearTimeout });
  const { useGateway, GatewayError } = await import("../../shared/gateway");
  const { useDeviceSession } =
    await import("../../shared/composables/useDeviceSession");
  const { useStore } = await import("./useStore");
  const gateway = useGateway();
  const session = useDeviceSession();
  await session.initialize();
  await gateway.demo.attachDevice();
  await vi.advanceTimersByTimeAsync(600);
  const deviceId = session.device.value!.id;
  const observed = {
    deviceId,
    entries: [
      { packageId: "pocket-reader", version: "1.2.0", installedAt: null },
    ],
    state: "fresh" as const,
    observedAt: 123,
    issue: null,
  };
  const read = vi.spyOn(gateway.store, "installed").mockResolvedValue(observed);
  const store = useStore();
  const init = store.initialize();
  await vi.advanceTimersByTimeAsync(300);
  await init;
  read.mockRejectedValueOnce(
    new GatewayError("installedReadFailed", "unavailable"),
  );
  await store.refreshInstalled();
  expect(store.installed.value).toHaveLength(1);
  expect(store.installedSnapshot.value?.state).toBe("stale");
  let complete!: (value: typeof observed) => void;
  read.mockImplementationOnce(
    () =>
      new Promise((resolve) => {
        complete = resolve;
      }),
  );
  const pending = store.refreshInstalled();
  expect(store.installedLoading.value).toBe(true);
  await gateway.demo.detachDevice();
  await vi.advanceTimersByTimeAsync(0);
  expect(store.installedLoading.value).toBe(false);
  complete(observed);
  await pending;
  expect(store.installed.value).toEqual([]);
  expect(store.installedSnapshot.value).toBe(null);
});

it("native installation reviews a device-bound plan before starting and rejects a changed selection", async () => {
  vi.resetModules();
  vi.useFakeTimers();
  vi.stubGlobal("window", { setTimeout, clearTimeout });
  const { createSimulatedGateway } =
    await import("../../shared/gateway/simulatedGateway");
  const browser = createSimulatedGateway();
  const makePlan = (
    deviceId: string,
  ): import("../../shared/gateway").PackagePlan => ({
    id: "native-plan",
    deviceId,
    deviceName: "Bound device",
    appId: "pocket-reader",
    names: { en: "Reader" },
    action: "install",
    bundleId: "reader.native",
    previous: null,
    releaseId: null,
    artifact: null,
    target: null,
    version: "1.0.0",
    revision: 1,
    publicationId: "catalog",
    sequence: 1,
    catalogExpiresAt: Date.now() + 10000,
    expiresAt: Date.now() + 10000,
    appsync: "unknown",
    jailbreak: "unknown",
    steps: [],
  });
  const start = vi.fn().mockResolvedValue({
    operationId: "native-operation",
    kind: "install",
    steps: [],
    subject: "pocket-reader",
  });
  const plan = vi.fn(
    async (request: import("../../shared/gateway").PackageRequest) =>
      makePlan(request.deviceId),
  );
  const gateway = {
    ...browser,
    flavor: "tauri" as const,
    store: { ...browser.store, plan, start, jobs: async () => [] },
  };
  vi.doMock("../../shared/gateway", async (importOriginal) => ({
    ...(await importOriginal<typeof import("../../shared/gateway")>()),
    useGateway: () => gateway,
  }));
  try {
    const { useDeviceSession } =
      await import("../../shared/composables/useDeviceSession");
    const { useStore } = await import("./useStore");
    const session = useDeviceSession();
    await session.initialize();
    await browser.demo.attachDevice();
    await vi.advanceTimersByTimeAsync(600);
    const store = useStore();
    const init = store.initialize();
    await vi.advanceTimersByTimeAsync(300);
    await init;
    await store.install("pocket-reader");
    expect(plan).toHaveBeenCalledTimes(1);
    expect(start).not.toHaveBeenCalled();
    expect(store.planOpen.value).toBe(true);
    expect(store.queuedIds.value).toEqual([]);
    await browser.demo.detachDevice();
    await vi.advanceTimersByTimeAsync(0);
    await store.confirmPlan(false);
    expect(start).not.toHaveBeenCalled();
    await browser.demo.attachDevice();
    await vi.advanceTimersByTimeAsync(600);
    await store.install("pocket-reader");
    await store.confirmPlan(false);
    expect(start).toHaveBeenCalledWith({
      planId: "native-plan",
      deleteData: false,
    });
    expect(store.planOpen.value).toBe(false);
  } finally {
    vi.doUnmock("../../shared/gateway");
  }
});
