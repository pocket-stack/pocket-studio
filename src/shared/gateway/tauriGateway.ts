import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import {
  GatewayError,
  type DeviceEvent,
  type LogEntry,
  type OperationEvent,
  type StudioGateway,
  type Unsubscribe,
} from "./types";

interface NativeError {
  code: string;
  message: string;
}

function isNativeError(value: unknown): value is NativeError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value
  );
}

async function call<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    if (isNativeError(error)) {
      throw new GatewayError(error.code, error.message);
    }
    throw new GatewayError("native", String(error));
  }
}

function subscribe<T>(
  eventName: string,
  handler: (payload: T) => void,
): Unsubscribe {
  let disposed = false;
  let unlisten: (() => void) | undefined;

  void listen<T>(eventName, (event) => handler(event.payload)).then((stop) => {
    if (disposed) stop();
    else unlisten = stop;
  });

  return () => {
    disposed = true;
    unlisten?.();
  };
}

export function createTauriGateway(): StudioGateway {
  return {
    flavor: "tauri",
    devices: {
      list: () => call("list_devices"),
      checkReadiness: (deviceId) => call("check_readiness", { deviceId }),
      onEvent: (handler) => subscribe<DeviceEvent>("studio://device", handler),
    },
    preparation: {
      plan: (deviceId) => call("plan_preparation", { deviceId }),
      start: (consent) => call("start_preparation", { consent }),
    },
    store: {
      uninstall: async () => {
        throw new GatewayError(
          "demoOnly",
          "Uninstall is only available in the UI simulation",
        );
      },
      catalog: () => call("list_catalog"),
      installed: (deviceId) => call("list_installed", { deviceId }),
      install: (deviceId, packageId) =>
        call("install_package", { deviceId, packageId }),
    },
    operations: {
      cancel: (operationId) => call("cancel_operation", { operationId }),
      onEvent: (handler) =>
        subscribe<OperationEvent>("studio://operation", handler),
    },
    logs: {
      list: () => call("list_logs"),
      export: () => call("export_logs"),
      onEntry: (handler) => subscribe<LogEntry>("studio://log", handler),
    },
    demo: {
      attachDevice: () => call("demo_attach_device"),
      detachDevice: () => call("demo_detach_device"),
      setDeviceMode: (mode) => call("demo_set_device_mode", { mode }),
      setJailbroken: (jailbroken) =>
        call("demo_set_jailbroken", { jailbroken }),
      failNextStep: (stepId) => call("demo_fail_next_step", { stepId }),
    },
  };
}
