import { computed, readonly, ref } from "vue";

import {
  GatewayError,
  useGateway,
  type DeviceSummary,
  type ReadinessReport,
} from "../gateway";
import { notify } from "./useNotifications";

const device = ref<DeviceSummary | null>(null);
const readiness = ref<ReadinessReport | null>(null);
const checking = ref(false);
const lastError = ref<string | null>(null);
let initialized = false;

async function checkReadiness(): Promise<void> {
  const current = device.value;
  if (!current || checking.value) return;
  checking.value = true;
  lastError.value = null;
  try {
    const report = await useGateway().devices.checkReadiness(current.id);
    if (device.value?.id === current.id) readiness.value = report;
  } catch (error) {
    lastError.value = error instanceof GatewayError ? error.code : "unknown";
    notify("error", "notifications.readinessFailed");
  } finally {
    checking.value = false;
  }
}

function clear(): void {
  device.value = null;
  readiness.value = null;
}

async function initialize(): Promise<void> {
  if (initialized) return;
  initialized = true;
  const gateway = useGateway();

  gateway.devices.onEvent((event) => {
    switch (event.type) {
      case "attached":
        device.value = event.device;
        readiness.value = null;
        notify("info", "notifications.deviceAttached", {
          name: event.device.marketingName,
        });
        void checkReadiness();
        break;
      case "detached":
        if (device.value?.id === event.deviceId) {
          clear();
          notify("warning", "notifications.deviceDetached");
        }
        break;
      case "modeChanged":
        if (device.value?.id === event.deviceId) {
          device.value = { ...device.value, mode: event.mode };
        }
        break;
    }
  });

  const attached = await gateway.devices.list();
  if (attached[0]) {
    device.value = attached[0];
    void checkReadiness();
  }
}

export function useDeviceSession() {
  return {
    device: computed(() => device.value),
    readiness: computed(() => readiness.value),
    checking: readonly(checking),
    lastError: readonly(lastError),
    isReady: computed(() => readiness.value?.status === "ready"),
    needsPreparation: computed(
      () => readiness.value?.status === "needsPreparation",
    ),
    initialize,
    checkReadiness,
  };
}
