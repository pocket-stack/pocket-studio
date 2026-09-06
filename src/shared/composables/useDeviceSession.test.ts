import { describe, expect, it, vi } from "vitest";
import { demoDevice } from "../gateway/fixtures";
import { createSimulatedGateway } from "../gateway/simulatedGateway";
import {
  GatewayError,
  type DeviceEvent,
  type DiscoverySnapshot,
  type StudioGateway,
} from "../gateway/types";
import { createDeviceSession } from "./useDeviceSession";

vi.mock("./useNotifications", () => ({ notify: vi.fn() }));

function snapshot(revision: number, ids = ["a"]): DiscoverySnapshot {
  return {
    revision,
    checkedAt: revision,
    issues: [],
    devices: ids.map((id) => ({ ...demoDevice, id })),
    reports: ids.map((id) => ({
      deviceId: id,
      status: "ready",
      checks: [],
      checkedAt: revision,
    })),
  };
}
function fixture() {
  let receive: ((event: DeviceEvent) => void) | undefined;
  const list = vi
    .fn<StudioGateway["devices"]["list"]>()
    .mockResolvedValue(snapshot(1));
  const subscribe = vi.fn<StudioGateway["devices"]["onEvent"]>(
    (handler: (event: DeviceEvent) => void) => {
      receive = handler;
      return Promise.resolve(vi.fn());
    },
  );
  const gateway: StudioGateway = {
    ...createSimulatedGateway(),
    flavor: "tauri",
    capabilities: { demo: false, preparation: false, packages: false },
    devices: { list, onEvent: subscribe, checkReadiness: vi.fn() },
  };
  return {
    gateway,
    list,
    subscribe,
    send: (value: DiscoverySnapshot) =>
      receive!({ type: "snapshot", snapshot: value }),
  };
}

describe("physical device sessions", () => {
  it("cannot resurrect a detached device from an older list response", async () => {
    const native = fixture();
    const session = createDeviceSession(native.gateway);
    await session.initialize();
    let resolve!: (value: DiscoverySnapshot) => void;
    native.list.mockImplementationOnce(
      () =>
        new Promise((done) => {
          resolve = done;
        }),
    );
    const pending = session.refresh();
    native.send(snapshot(3, []));
    resolve(snapshot(2));
    await pending;
    expect(session.device.value).toBeNull();
    expect(session.readiness.value).toBeNull();
    expect(session.isReady.value).toBe(false);
    session.dispose();
  });

  it("waits for the native subscription before asking for an inventory", async () => {
    const native = fixture();
    let finish!: (stop: () => void) => void;
    native.subscribe.mockImplementationOnce(
      () =>
        new Promise((done) => {
          finish = done;
        }),
    );
    const session = createDeviceSession(native.gateway);
    const initializing = session.initialize();
    expect(native.list).not.toHaveBeenCalled();
    finish(vi.fn());
    await initializing;
    expect(session.device.value?.id).toBe("a");
    session.dispose();
  });

  it("preserves device selection and invalidates readiness on a failed refresh", async () => {
    const native = fixture();
    native.list.mockResolvedValue(snapshot(1, ["a", "b"]));
    const session = createDeviceSession(native.gateway);
    await session.initialize();
    session.select("b");
    native.send(snapshot(2, ["b", "a"]));
    expect(session.device.value?.id).toBe("b");
    expect(session.readiness.value?.deviceId).toBe("b");
    native.list.mockRejectedValueOnce(
      new GatewayError("native", "read failed"),
    );
    await session.refresh();
    expect(session.isReady.value).toBe(false);
    expect(session.lastError.value).toBe("native");
    session.dispose();
  });

  it("can retry initialization after a failed event subscription", async () => {
    const native = fixture();
    native.subscribe.mockRejectedValueOnce(
      new GatewayError("eventSubscriptionFailed", "listen failed"),
    );
    const session = createDeviceSession(native.gateway);
    await session.initialize();
    expect(native.list).not.toHaveBeenCalled();
    await session.refresh();
    expect(session.device.value?.id).toBe("a");
    expect(session.lastError.value).toBeNull();
    expect(native.subscribe).toHaveBeenCalledTimes(2);
    session.dispose();
  });
});
