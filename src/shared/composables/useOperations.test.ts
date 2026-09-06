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
  const handle = await gateway.preparation.start({
    planId: plan.id,
    prerequisitesConfirmed: plan.prerequisites,
    acknowledgedRiskIds: plan.risks.map((risk) => risk.id),
    riskReadingSeconds: 15,
    risksAcknowledgedAt: Date.now() - 22_000,
    disclaimerVersion: plan.disclaimerVersion,
    disclaimerReadingSeconds: 20,
    disclaimerAcceptedAt: Date.now() - 1000,
  });
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
