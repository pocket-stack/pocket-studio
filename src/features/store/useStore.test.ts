import { afterEach, expect, it, vi } from "vitest";
vi.mock("vue-i18n", () => ({
  useI18n: () => ({
    t: (key: string) => key,
    te: (key: string) => key.startsWith("store.actions.errors."),
    locale: { value: "en" },
  }),
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
      {
        installationId: "ios:pocket-reader",
        packageId: "pocket-reader",
        version: "1.2.0",
        installedAt: null,
      },
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

it("native installation submits the device-bound plan directly and refuses a changed device", async () => {
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
    installation: {
      platform: "ios",
      bundleId: "reader.native",
      previous: null,
      appsync: "unknown",
      jailbreak: "unknown",
    },
    deleteData: false,
    releaseId: null,
    artifact: null,
    target: null,
    version: "1.0.0",
    revision: 1,
    publicationId: "catalog",
    sequence: 1,
    catalogExpiresAt: Date.now() + 10000,
    expiresAt: Date.now() + 10000,
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
    const { useNotifications } =
      await import("../../shared/composables/useNotifications");
    const { useStore } = await import("./useStore");
    const session = useDeviceSession();
    await session.initialize();
    await browser.demo.attachDevice();
    await vi.advanceTimersByTimeAsync(600);
    const store = useStore();
    const init = store.initialize();
    await vi.advanceTimersByTimeAsync(300);
    await init;
    // No confirmation step: the plan is resolved and started in one go.
    await store.install("pocket-reader");
    expect(plan).toHaveBeenCalledTimes(1);
    expect(start).toHaveBeenCalledWith({
      planId: "native-plan",
      deleteData: false,
    });
    expect(store.pendingIds.value).toEqual([]);
    expect(store.queuedIds.value).toEqual([]);
    // A plan bound to another device never starts.
    plan.mockImplementationOnce(async () => makePlan("someone-else"));
    await store.install("pocket-reader");
    expect(plan).toHaveBeenCalledTimes(2);
    expect(start).toHaveBeenCalledTimes(1);
    expect(useNotifications().items.value.at(-1)).toMatchObject({
      tone: "error",
      key: "store.actions.errors.deviceChanged",
    });
    // Consent mirrors the plan: removing an iOS app takes its data along.
    plan.mockImplementationOnce(async () => ({
      ...makePlan(session.device.value!.id),
      action: "uninstall",
      deleteData: true,
    }));
    await store.uninstall("pocket-reader", "ios:reader.native");
    expect(plan).toHaveBeenLastCalledWith(
      expect.objectContaining({
        action: "uninstall",
        installationId: "ios:reader.native",
      }),
    );
    expect(start).toHaveBeenLastCalledWith({
      planId: "native-plan",
      deleteData: true,
    });
  } finally {
    vi.doUnmock("../../shared/gateway");
  }
});

it("plans 3DS titles per instance: a single published form installs directly, several ask for the form", async () => {
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
  await gateway.demo.attachDevice("n3dsll");
  await vi.advanceTimersByTimeAsync(600);
  const store = useStore();
  const init = store.initialize();
  await vi.advanceTimersByTimeAsync(300);
  await init;
  const start = vi.spyOn(gateway.store, "start");
  // Theme Forge is published as a CIA only: no choice to make.
  await store.install("ctr-theme-forge");
  expect(start).toHaveBeenCalledTimes(1);
  expect(store.installed.value.map((item) => item.managed?.format)).toEqual([
    "cia",
  ]);
  // Dual Notebook ships as .pocket, CIA and 3DSX: the form is the user's call.
  await store.install("ctr-dual-notebook");
  expect(store.deliveryAppId.value).toBe("ctr-dual-notebook");
  expect(start).toHaveBeenCalledTimes(1);
  await store.chooseDelivery("shared", "pocket");
  expect(store.deliveryAppId.value).toBe(null);
  expect(store.installed.value).toHaveLength(2);
  const shared = store.installed.value.find(
    (item) => item.packageId === "ctr-dual-notebook",
  )!;
  expect(shared.managed).toMatchObject({
    delivery: "shared",
    format: "pocket",
  });
  // Removal keeps app data unless the wipe is requested explicitly.
  await store.uninstall("ctr-dual-notebook", shared.installationId);
  expect(start).toHaveBeenLastCalledWith(
    expect.objectContaining({ deleteData: false }),
  );
  expect(store.installed.value.map((item) => item.packageId)).toEqual([
    "ctr-theme-forge",
  ]);
  const cia = store.installed.value[0]!;
  await store.uninstall("ctr-theme-forge", cia.installationId, true);
  expect(start).toHaveBeenLastCalledWith(
    expect.objectContaining({ deleteData: true }),
  );
  expect(store.installed.value).toEqual([]);
});
