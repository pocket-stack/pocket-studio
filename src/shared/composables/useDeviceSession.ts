import { computed, readonly, ref } from "vue";
import {
  GatewayError,
  useGateway,
  type DeviceEvent,
  type DeviceSummary,
  type DiscoveryIssue,
  type DiscoverySnapshot,
  type ReadinessReport,
  type StudioGateway,
  type Unsubscribe,
} from "../gateway";
import { notify } from "./useNotifications";

/** Ignore stale IPC results after a detach, selection change, or newer scan. */
export function createDeviceSession(gateway: StudioGateway) {
  const devices = ref<DeviceSummary[]>([]);
  const selectedId = ref<string | null>(null);
  const readiness = ref<ReadinessReport | null>(null);
  const issues = ref<DiscoveryIssue[]>([]);
  const checking = ref(false);
  const scanning = ref(false);
  const lastError = ref<string | null>(null);
  const reports = new Map<string, ReadinessReport>();
  const device = computed(
    () => devices.value.find((item) => item.id === selectedId.value) ?? null,
  );
  let revision = -1;
  let lifecycle = 0;
  let checkToken = 0;
  let initialized = false;
  let initializePromise: Promise<void> | undefined;
  let refreshPromise: Promise<void> | undefined;
  let stop: Unsubscribe | undefined;

  function select(id: string): void {
    if (!devices.value.some((item) => item.id === id)) return;
    if (selectedId.value !== id) {
      checkToken++;
      checking.value = false;
    }
    selectedId.value = id;
    readiness.value = reports.get(id) ?? null;
    if (gateway.capabilities.demo && !readiness.value) void checkReadiness();
  }

  function reconcileSelection(): void {
    if (device.value) return;
    checkToken++;
    checking.value = false;
    const preferred =
      devices.value.find((item) => item.modelIdentifier === "iPod4,1") ??
      devices.value[0];
    selectedId.value = preferred?.id ?? null;
    readiness.value = preferred ? (reports.get(preferred.id) ?? null) : null;
  }

  function applySnapshot(snapshot: DiscoverySnapshot): void {
    if (!gateway.capabilities.demo && snapshot.revision <= revision) return;
    revision = snapshot.revision;
    lifecycle++;
    devices.value = snapshot.devices;
    issues.value = snapshot.issues;
    reports.clear();
    snapshot.reports.forEach((report) => reports.set(report.deviceId, report));
    reconcileSelection();
    readiness.value = selectedId.value
      ? (reports.get(selectedId.value) ?? null)
      : null;
    lastError.value = null;
  }

  function onEvent(event: DeviceEvent): void {
    if (event.type === "snapshot") {
      applySnapshot(event.snapshot);
      return;
    }
    lifecycle++;
    if (event.type === "attached") {
      devices.value = [
        ...devices.value.filter((item) => item.id !== event.device.id),
        event.device,
      ];
      reports.delete(event.device.id);
      reconcileSelection();
      if (selectedId.value === event.device.id) {
        readiness.value = null;
        void checkReadiness();
      }
      notify("info", "notifications.deviceAttached", {
        name: event.device.marketingName,
      });
    } else if (event.type === "detached") {
      devices.value = devices.value.filter(
        (item) => item.id !== event.deviceId,
      );
      reports.delete(event.deviceId);
      reconcileSelection();
      notify("warning", "notifications.deviceDetached");
    } else {
      devices.value = devices.value.map((item) =>
        item.id === event.deviceId ? { ...item, mode: event.mode } : item,
      );
    }
  }

  async function fetchDevices(): Promise<void> {
    const started = lifecycle;
    const snapshot = await gateway.devices.list();
    if (!gateway.capabilities.demo || lifecycle === started)
      applySnapshot(snapshot);
  }

  function initialize(): Promise<void> {
    if (initialized) return Promise.resolve();
    if (initializePromise) return initializePromise;
    initializePromise = (async () => {
      scanning.value = true;
      try {
        stop ??= await gateway.devices.onEvent(onEvent);
        await fetchDevices();
        initialized = true;
      } catch (error) {
        lastError.value =
          error instanceof GatewayError ? error.code : "unknown";
      } finally {
        scanning.value = false;
        initializePromise = undefined;
      }
    })();
    return initializePromise;
  }

  function refresh(): Promise<void> {
    if (!initialized) return initialize();
    if (refreshPromise) return refreshPromise;
    refreshPromise = (async () => {
      scanning.value = true;
      lastError.value = null;
      try {
        await fetchDevices();
      } catch (error) {
        lastError.value =
          error instanceof GatewayError ? error.code : "unknown";
        readiness.value = null;
      } finally {
        scanning.value = false;
        refreshPromise = undefined;
      }
    })();
    return refreshPromise;
  }

  async function checkReadiness(): Promise<void> {
    const current = device.value;
    if (!current) return;
    const token = ++checkToken;
    checking.value = true;
    lastError.value = null;
    try {
      if (!gateway.capabilities.demo) {
        await refresh();
        return;
      }
      const report = await gateway.devices.checkReadiness(current.id);
      if (token === checkToken && device.value?.id === current.id) {
        readiness.value = report;
        reports.set(current.id, report);
      }
    } catch (error) {
      if (token === checkToken) {
        readiness.value = null;
        lastError.value =
          error instanceof GatewayError ? error.code : "unknown";
        notify("error", "notifications.readinessFailed");
      }
    } finally {
      if (token === checkToken) checking.value = false;
    }
  }

  return {
    device,
    devices: readonly(devices),
    readiness: computed(() => readiness.value),
    checking: computed(() => checking.value || scanning.value),
    scanning: readonly(scanning),
    issues: readonly(issues),
    lastError: readonly(lastError),
    isReady: computed(() => readiness.value?.status === "ready"),
    needsPreparation: computed(
      () => readiness.value?.status === "needsPreparation",
    ),
    initialize,
    refresh,
    select,
    checkReadiness,
    dispose() {
      stop?.();
      stop = undefined;
      initialized = false;
      checkToken++;
    },
  };
}

let session: ReturnType<typeof createDeviceSession> | undefined;
export function useDeviceSession() {
  return (session ??= createDeviceSession(useGateway()));
}
