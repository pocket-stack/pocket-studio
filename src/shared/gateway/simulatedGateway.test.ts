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
      gateway.preparation.start({ ...consentFor(plan), riskReadingSeconds: 0 }),
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
    const handle = await gateway.preparation.start(consentFor(plan));
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
    const handle = await gateway.preparation.start(consentFor(plan));
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
    await gateway.preparation.start(consentFor(plan));
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
    expect(await gateway.store.installed(id)).toEqual([]);
  });
  it("registers a successful installation and its version", async () => {
    const id = await connect();
    await gateway.store.install(id, "pocket-reader");
    await vi.advanceTimersByTimeAsync(12_000);
    expect(await gateway.store.installed(id)).toEqual([
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
    expect(await gateway.store.installed(id)).toEqual([]);
  });
});
