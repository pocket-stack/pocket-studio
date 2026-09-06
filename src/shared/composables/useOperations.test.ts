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
