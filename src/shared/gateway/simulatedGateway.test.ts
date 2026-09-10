import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createSimulatedGateway } from "./simulatedGateway";
import type {
  ConsentRecord,
  OperationEvent,
  PreparationPlan,
  StudioGateway,
} from "./types";
import { validateConsent } from "./consent";
import { buildJailbreakPlan } from "./fixtures";

function consentFor(plan: PreparationPlan): ConsentRecord {
  return {
    planId: plan.id,
    prerequisitesConfirmed: [...plan.prerequisites],
    acknowledgedRiskIds: plan.risks.map((risk) => risk.id),
    riskReadingSeconds: 5,
    risksAcknowledgedAt: Date.now() - 6_000,
    disclaimerVersion: plan.disclaimerVersion,
    disclaimerReadingSeconds: 5,
    disclaimerAcceptedAt: Date.now() - 1_000,
  };
}
beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-09-06T09:00:00Z"));
});
afterEach(() => {
  vi.useRealTimers();
});

describe("separate consent gates", () => {
  it("requires every risk and prerequisite, as well as both reading periods", () => {
    const plan = buildJailbreakPlan("demo", 1);
    for (const patch of [
      { acknowledgedRiskIds: [] },
      { prerequisitesConfirmed: [] },
      { riskReadingSeconds: 4 },
      { disclaimerReadingSeconds: 4 },
      { disclaimerVersion: "old" },
    ]) {
      expect(() =>
        validateConsent(plan, { ...consentFor(plan), ...patch }, Date.now()),
      ).toThrow();
    }
    expect(() =>
      validateConsent(plan, consentFor(plan), Date.now()),
    ).not.toThrow();
  });
  it("rejects reversed, simultaneous, future, and expired confirmation times", () => {
    const plan = buildJailbreakPlan("demo", 1);
    for (const patch of [
      { risksAcknowledgedAt: Date.now() },
      { risksAcknowledgedAt: Date.now() - 1_000 },
      { disclaimerAcceptedAt: Date.now() + 1000 },
      {
        risksAcknowledgedAt: Date.now() - 2_000_000,
        disclaimerAcceptedAt: Date.now() - 1_900_000,
      },
    ]) {
      expect(() =>
        validateConsent(plan, { ...consentFor(plan), ...patch }, Date.now()),
      ).toThrow();
    }
  });
});

describe("device simulation", () => {
  let gateway: StudioGateway;
  let events: OperationEvent[];
  beforeEach(() => {
    gateway = createSimulatedGateway();
    events = [];
    gateway.operations.onEvent((event) => events.push(event));
  });
  async function connect(): Promise<string> {
    await gateway.demo.attachDevice();
    return (await gateway.devices.list()).devices[0]!.id;
  }
  it("discovery and readiness never start a preparation", async () => {
    const id = await connect();
    const report = gateway.devices.checkReadiness(id);
    await vi.advanceTimersByTimeAsync(600);
    expect((await report).status).toBe("needsPreparation");
    expect(events).toHaveLength(0);
  });
  it("does not start or record successful consent when a reading gate is incomplete", async () => {
    const plan = await gateway.preparation.plan(await connect());
    await expect(
      gateway.preparation.start(
        { ...consentFor(plan), riskReadingSeconds: 0 },
        "alpine",
      ),
    ).rejects.toMatchObject({ code: "readingTooShort" });
    expect(events).toHaveLength(0);
    expect(
      (await gateway.logs.list()).some(
        (entry) => entry.code === "log.preparation.consentRecorded",
      ),
    ).toBe(false);
  });
  it("waits for a DFU event, then finishes with complete consent audit fields", async () => {
    const plan = await gateway.preparation.plan(await connect());
    const handle = await gateway.preparation.start(consentFor(plan), "alpine");
    await vi.advanceTimersByTimeAsync(1000);
    expect(events).toContainEqual(
      expect.objectContaining({ type: "actionRequired", action: "enterDfu" }),
    );
    expect(events.some((event) => event.type === "finished")).toBe(false);
    await gateway.demo.setDeviceMode("dfu");
    await vi.advanceTimersByTimeAsync(40_000);
    expect(events).toContainEqual({
      type: "finished",
      operationId: handle.operationId,
    });
    const audit = (await gateway.logs.list()).find(
      (entry) => entry.code === "log.preparation.consentRecorded",
    );
    expect(audit?.params).toMatchObject({
      riskReadingSeconds: "5",
      disclaimerReadingSeconds: "5",
      disclaimerVersion: plan.disclaimerVersion,
      mode: "simulation",
    });
  });
  it("cancels a DFU wait without executing later steps", async () => {
    const plan = await gateway.preparation.plan(await connect());
    const handle = await gateway.preparation.start(consentFor(plan), "alpine");
    await gateway.operations.cancel(handle.operationId);
    await vi.advanceTimersByTimeAsync(1000);
    expect(events.some((event) => event.type === "cancelled")).toBe(true);
    expect(
      events.some(
        (event) =>
          event.type === "stepChanged" &&
          event.stepId === "exploitBootrom" &&
          event.status === "running",
      ),
    ).toBe(false);
  });

  it("starts an already-DFU device without requesting the button sequence", async () => {
    const id = await connect();
    await gateway.demo.setDeviceMode("dfu");
    const plan = await gateway.preparation.plan(id);
    expect(plan.entryMode).toBe("dfu");
    expect(plan.steps.some((step) => step.id === "enterDfu")).toBe(false);
    expect(plan.prerequisites).not.toContain("workingButtons");
    await gateway.preparation.start(consentFor(plan), "alpine");
    await vi.advanceTimersByTimeAsync(40_000);
    expect(events.some((event) => event.type === "actionRequired")).toBe(false);
    expect(events.some((event) => event.type === "finished")).toBe(true);
  });
  it("rejects unsupported apps and missing dependencies before starting", async () => {
    const id = await connect();
    await gateway.demo.setJailbroken(true);
    await expect(
      gateway.store.install(id, "pocket-camera-pro"),
    ).rejects.toMatchObject({ code: "incompatiblePackage" });
    await expect(
      gateway.store.install(id, "pocket-agent"),
    ).rejects.toMatchObject({ code: "missingDependencies" });
    expect(events).toHaveLength(0);
  });
  it("does not register an app when integrity verification fails", async () => {
    const id = await connect();
    await gateway.demo.failNextStep("verify");
    await gateway.store.install(id, "pocket-reader");
    await vi.advanceTimersByTimeAsync(12_000);
    expect(events).toContainEqual(
      expect.objectContaining({
        type: "failed",
        error: expect.objectContaining({ code: "checksumMismatch" }),
      }),
    );
    expect((await gateway.store.installed(id)).entries).toEqual([]);
  });
  it("registers a successful installation and its version", async () => {
    const id = await connect();
    await gateway.store.install(id, "pocket-reader");
    await vi.advanceTimersByTimeAsync(12_000);
    expect((await gateway.store.installed(id)).entries).toEqual([
      expect.objectContaining({ packageId: "pocket-reader", version: "1.2.0" }),
    ]);
  });
  it("a rapid disconnect/reconnect still fails the old operation", async () => {
    const id = await connect();
    await gateway.store.install(id, "pocket-reader");
    await gateway.demo.detachDevice();
    await gateway.demo.attachDevice();
    await vi.advanceTimersByTimeAsync(12_000);
    expect(events).toContainEqual(
      expect.objectContaining({
        type: "failed",
        error: expect.objectContaining({ code: "deviceDisconnected" }),
      }),
    );
    expect((await gateway.store.installed(id)).entries).toEqual([]);
  });
});

describe("New 3DS LL simulation", () => {
  let gateway: StudioGateway;
  beforeEach(() => {
    gateway = createSimulatedGateway();
  });
  async function connect(): Promise<string> {
    await gateway.demo.attachDevice("n3dsll");
    return (await gateway.devices.list()).devices[0]!.id;
  }
  async function catalog(id: string) {
    const pending = gateway.store.catalog(id);
    await vi.advanceTimersByTimeAsync(300);
    return (await pending).entries;
  }
  it("arrives over the network with the native readiness checks and no guided preparation", async () => {
    const id = await connect();
    const snapshot = await gateway.devices.list();
    expect(snapshot.devices[0]).toMatchObject({
      platform: "3ds",
      transport: "network",
      modelIdentifier: "RED",
    });
    const report = snapshot.reports[0]!;
    expect(report.status).toBe("ready");
    expect(report.requiredWorkflow).toBeUndefined();
    expect(report.checks.map((check) => check.id)).toEqual([
      "platformSupported",
      "modelSupported",
      "pairingTrusted",
      "cfwInstalled",
      "runtimeAvailable",
    ]);
    await expect(gateway.preparation.plan(id)).rejects.toMatchObject({
      code: "unsupportedDevice",
    });
  });
  it("pairs through the preparation flow and lands on the same console", async () => {
    const setup = await gateway.setup.plan({
      destination: { kind: "sd", path: "/demo/card" },
      format: "cia",
    });
    expect(setup.files).toEqual([
      "pocketjs/runtime/dev.key",
      "cias/pocket-runtime-ctr.cia",
    ]);
    const paired = await gateway.setup.execute(setup.id);
    const connected = await gateway.setup.connect(paired.pairingId);
    expect(connected.devices.map((device) => device.platform)).toEqual(["3ds"]);
    await expect(gateway.setup.execute(setup.id)).rejects.toMatchObject({
      code: "planExpired",
    });
  });
  it("only reports a missing CFW and refuses plans until it is back", async () => {
    const id = await connect();
    await gateway.demo.setCfwInstalled(false);
    const report = gateway.devices.checkReadiness(id);
    await vi.advanceTimersByTimeAsync(600);
    expect(await report).toMatchObject({ status: "needsPreparation" });
    expect((await report).requiredWorkflow).toBeUndefined();
    const request = {
      deviceId: id,
      appId: "ctr-dual-notebook",
      action: "install" as const,
      delivery: "shared" as const,
      format: "pocket",
    };
    await expect(gateway.store.plan(request)).rejects.toMatchObject({
      code: "deviceNotReady",
    });
    expect(
      (await catalog(id)).find((entry) => entry.id === request.appId)?.details
        ?.verdict,
    ).toBe("requiresPreparation");
    await gateway.demo.setCfwInstalled(true);
    const plan = await gateway.store.plan(request);
    expect(plan.installation).toMatchObject({
      platform: "3ds",
      delivery: "shared",
      format: "pocket",
    });
  });
  it("keeps delivery instances separate and consumes consent once", async () => {
    const id = await connect();
    const entry = (await catalog(id)).find(
      (item) => item.id === "ctr-dual-notebook",
    )!;
    expect(
      entry.details?.candidates.map((c) => `${c.delivery}/${c.format}`),
    ).toEqual([
      "shared/pocket",
      "bundled/pocket",
      "bundled/cia",
      "bundled/3dsx",
    ]);
    const shared = await gateway.store.plan({
      deviceId: id,
      appId: entry.id,
      action: "install",
      delivery: "shared",
      format: "pocket",
    });
    const bundled = await gateway.store.plan({
      deviceId: id,
      appId: entry.id,
      action: "install",
      delivery: "bundled",
      format: "cia",
    });
    await gateway.store.start({ planId: shared.id, deleteData: false });
    await gateway.store.start({ planId: bundled.id, deleteData: false });
    await expect(
      gateway.store.start({ planId: shared.id, deleteData: false }),
    ).rejects.toMatchObject({ code: "planExpired" });
    const installed = (await gateway.store.installed(id)).entries;
    expect(installed).toHaveLength(2);
    expect(installed[0]!.installationId).not.toBe(installed[1]!.installationId);
    // The launcher is prepared through setup and never planned as an app.
    await expect(
      gateway.store.plan({
        deviceId: id,
        appId: "pocket-runtime-ctr",
        action: "install",
        delivery: "bundled",
        format: "cia",
      }),
    ).rejects.toMatchObject({ code: "invalidAction" });
    const remove = await gateway.store.plan({
      deviceId: id,
      appId: entry.id,
      action: "uninstall",
      installationId: installed[0]!.installationId,
    });
    expect(remove.deleteData).toBe(false);
    await expect(
      gateway.store.start({ planId: remove.id, deleteData: true }),
    ).rejects.toMatchObject({ code: "consentRequired" });
    const wipe = await gateway.store.plan({
      deviceId: id,
      appId: entry.id,
      action: "uninstall",
      installationId: installed[0]!.installationId,
      deleteData: true,
    });
    expect(wipe.deleteData).toBe(true);
    await gateway.store.start({ planId: wipe.id, deleteData: true });
    expect(
      (await gateway.store.installed(id)).entries.map(
        (item) => item.managed!.delivery,
      ),
    ).toEqual(["bundled"]);
  });
});
