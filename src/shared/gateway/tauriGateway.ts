import { invoke, convertFileSrc } from "@tauri-apps/api/core";
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

async function subscribe<T>(
  eventName: string,
  handler: (payload: T) => void,
): Promise<Unsubscribe> {
  try {
    return await listen<T>(eventName, (event) => handler(event.payload));
  } catch {
    throw new GatewayError(
      "eventSubscriptionFailed",
      "Native event subscription failed",
    );
  }
}

async function unavailable(): Promise<never> {
  throw new GatewayError(
    "operationUnavailable",
    "This operation is unavailable",
  );
}

export function createTauriGateway(): StudioGateway {
  return {
    flavor: "tauri",
    capabilities: {
      demo: false,
      preparation: true,
      catalog: true,
      packages: true,
      installed: true,
    },
    devices: {
      checkAppSync: (deviceId, sshPassword) =>
        call("check_appsync", { deviceId, sshPassword }),
      list: () => call("list_devices"),
      checkReadiness: (deviceId) => call("check_readiness", { deviceId }),
      onEvent: (handler) => subscribe<DeviceEvent>("studio://device", handler),
    },
    preparation: {
      plan: (deviceId) => call("plan_preparation", { deviceId }),
      start: (consent, sshPassword) =>
        call("start_preparation", { consent, sshPassword }),
    },
    store: {
      uninstall: unavailable,
      catalog: (deviceId, refresh = true) =>
        call("list_catalog", { deviceId, refresh }),
      media: async (sha256) =>
        convertFileSrc(await call<string>("store_media", { sha256 })),
      installed: (deviceId) => call("list_installed", { deviceId }),
      install: unavailable,
      plan: (request) => call("plan_package", { request }),
      start: (consent) => call("start_package", { consent }),
      jobs: () => call("list_package_operations"),
      verify: (operationId, deviceId) =>
        call("verify_package_operation", { operationId, deviceId }),
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
      attachDevice: unavailable,
      detachDevice: unavailable,
      setDeviceMode: unavailable,
      setJailbroken: unavailable,
      failNextStep: unavailable,
    },
  };
}
