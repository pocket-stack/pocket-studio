import { expect, it, vi } from "vitest";
it("retains initial DFU events emitted before the command returns its handle", async () => {
  vi.resetModules();
  const { useGateway } = await import("../gateway");
  const { useOperations, trackOperation } = await import("./useOperations");
  const gateway = useGateway();
  const operations = useOperations();
  await gateway.demo.attachDevice();
  const id = (await gateway.devices.list()).devices[0]!.id;
  const plan = await gateway.preparation.plan(id);
  const handle = await gateway.preparation.start(
    {
      planId: plan.id,
      prerequisitesConfirmed: plan.prerequisites,
      acknowledgedRiskIds: plan.risks.map((risk) => risk.id),
      riskReadingSeconds: 5,
      risksAcknowledgedAt: Date.now() - 6_000,
      disclaimerVersion: plan.disclaimerVersion,
      disclaimerReadingSeconds: 5,
      disclaimerAcceptedAt: Date.now() - 1000,
    },
    "alpine",
  );
  trackOperation(handle);
  expect(operations.get(handle.operationId)?.pendingAction).toBe("enterDfu");
  expect(operations.get(handle.operationId)?.currentStepId).toBe("enterDfu");
  await gateway.operations.cancel(handle.operationId);
});

it("waits for native event subscription and surfaces failures so a write cannot start unseen", async () => {
  vi.resetModules();
  let release: (() => void) | undefined;
  const onEvent = vi
    .fn()
    .mockRejectedValueOnce(new Error("subscription unavailable"))
    .mockImplementationOnce(
      () =>
        new Promise<() => void>((resolve) => {
          release = () => resolve(() => {});
        }),
    );
  vi.doMock("../gateway", () => ({
    useGateway: () => ({ operations: { onEvent } }),
  }));
  const { useOperations } = await import("./useOperations");
  const operations = useOperations();
  await expect(operations.ready()).rejects.toThrow("subscription unavailable");
  let ready = false;
  const pending = operations.ready().then(() => {
    ready = true;
  });
  await Promise.resolve();
  expect(ready).toBe(false);
  release!();
  await pending;
  expect(ready).toBe(true);
  expect(onEvent).toHaveBeenCalledTimes(2);
  vi.doUnmock("../gateway");
});

it("hydrates submitted jobs as read-only follow-up state and preserves their bound device", async () => {
  vi.resetModules();
  const { hydratePackageJobs, useOperations } = await import("./useOperations");
  const plan: import("../gateway").PackagePlan = {
    id: "plan",
    deviceId: "original-device",
    deviceName: "Original device",
    appId: "app",
    names: { en: "Notes" },
    action: "uninstall",
    bundleId: "native.notes",
    previous: null,
    releaseId: null,
    artifact: null,
    target: null,
    version: null,
    revision: null,
    publicationId: "publication",
    sequence: 1,
    catalogExpiresAt: 100,
    expiresAt: 100,
    appsync: "unknown",
    jailbreak: "unknown",
    steps: [
      {
        id: "uninstall",
        cancellable: false,
        pointOfNoReturn: true,
        estimatedSeconds: 5,
      },
    ],
  };
  const job: import("../gateway").PackageJob = {
    queueOrder: 1,
    handle: {
      operationId: "saved",
      kind: "uninstall",
      steps: plan.steps,
      subject: "app",
    },
    plan,
    phase: "unverified",
    step: "uninstall",
    completedSteps: [],
    percent: 0,
    submitted: true,
    cleanupComplete: null,
    stagingPath: null,
    error: { code: "verificationUnavailable", recoverable: true },
    updatedAt: 123,
  };
  hydratePackageJobs([job]);
  const state = useOperations().get("saved")!;
  expect(state.status).toBe("failed");
  expect(state.packagePlan?.deviceId).toBe("original-device");
  hydratePackageJobs([{ ...job, phase: "running", error: null }]);
  expect(state.status).toBe("failed");
  hydratePackageJobs([
    { ...job, phase: "verified", completedSteps: ["uninstall"], error: null },
  ]);
  expect(state.status).toBe("finished");
  expect(state.steps[0]?.status).toBe("done");
});

it("reports the percent a running step will reach so progress can trickle toward it", async () => {
  vi.resetModules();
  const { operationProgress, operationStepCeiling } =
    await import("./useOperations");
  const step = (id: string, estimatedSeconds: number) => ({
    id: id as never,
    estimatedSeconds,
    cancellable: true,
    pointOfNoReturn: false,
    percent: 0,
    status: "pending" as const,
  });
  const operation = {
    id: "op",
    kind: "preparation" as const,
    status: "running" as const,
    startedAt: 0,
    steps: [
      { ...step("fetchResources", 30), status: "done" as const, percent: 100 },
      { ...step("buildRamdisk", 50), status: "running" as const, percent: 20 },
      step("bootRamdisk", 20),
    ],
  };
  expect(operationProgress(operation)).toBe(40);
  expect(operationStepCeiling(operation)).toBe(80);
});
