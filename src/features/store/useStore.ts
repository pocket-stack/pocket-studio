import { computed, readonly, ref, watch } from "vue";

import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { notify } from "../../shared/composables/useNotifications";
import {
  trackOperation,
  useOperations,
  type OperationState,
} from "../../shared/composables/useOperations";
import {
  GatewayError,
  useGateway,
  type CatalogEntry,
  type InstalledPackage,
  type PackageCategory,
} from "../../shared/gateway";
import {
  evaluateCompatibility,
  type CompatibilityVerdict,
} from "./compatibility";

const catalog = ref<CatalogEntry[]>([]);
const installed = ref<InstalledPackage[]>([]);
const loading = ref(false);
const loadError = ref<string | null>(null);
const selectedId = ref<string | null>(null);
const categoryFilter = ref<PackageCategory | "all">("all");
const query = ref("");
/** package id -> most recent install operation id */
const installOperations = ref(new Map<string, string>());
let initialized = false;

const { device, readiness } = useDeviceSession();
const { get, cancel } = useOperations();

async function refreshInstalled(): Promise<void> {
  const current = device.value;
  if (!current) {
    installed.value = [];
    return;
  }
  try {
    installed.value = await useGateway().store.installed(current.id);
  } catch {
    installed.value = [];
  }
}

async function initialize(): Promise<void> {
  if (initialized) return;
  initialized = true;
  loading.value = true;
  loadError.value = null;
  try {
    catalog.value = await useGateway().store.catalog();
  } catch (error) {
    loadError.value = error instanceof GatewayError ? error.code : "unknown";
  } finally {
    loading.value = false;
  }
  await refreshInstalled();
  watch(
    () => device.value?.id,
    () => void refreshInstalled(),
  );
}

export interface PackageView {
  entry: CatalogEntry;
  verdict: CompatibilityVerdict;
  installed?: InstalledPackage;
  operation?: OperationState;
  missingDependencies: string[];
}

function view(entry: CatalogEntry): PackageView {
  const operationId = installOperations.value.get(entry.id);
  const installedIds = new Set(installed.value.map((item) => item.packageId));
  return {
    entry,
    verdict: evaluateCompatibility(entry, device.value, readiness.value),
    installed: installed.value.find((item) => item.packageId === entry.id),
    operation: operationId ? get(operationId) : undefined,
    missingDependencies: entry.dependencies.filter(
      (dependency) => !installedIds.has(dependency),
    ),
  };
}

async function install(packageId: string): Promise<void> {
  const current = device.value;
  if (!current) return;
  const active = installOperations.value.get(packageId);
  if (active && get(active)?.status === "running") return;
  try {
    const handle = await useGateway().store.install(current.id, packageId);
    trackOperation(handle);
    const next = new Map(installOperations.value);
    next.set(packageId, handle.operationId);
    installOperations.value = next;
    watchOperation(handle.operationId);
  } catch (error) {
    const code = error instanceof GatewayError ? error.code : "unknown";
    notify(
      "error",
      code === "deviceNotReady"
        ? "notifications.installNeedsPreparation"
        : "notifications.installStartFailed",
    );
  }
}

function watchOperation(operationId: string): void {
  const stop = watch(
    () => get(operationId)?.status,
    (status) => {
      if (!status || status === "running") return;
      stop();
      void refreshInstalled();
      if (status === "finished")
        notify("success", "notifications.installFinished");
      if (status === "failed") notify("error", "notifications.installFailed");
    },
  );
}

async function cancelInstall(packageId: string): Promise<void> {
  const operationId = installOperations.value.get(packageId);
  if (!operationId) return;
  try {
    await cancel(operationId);
  } catch (error) {
    const code = error instanceof GatewayError ? error.code : "unknown";
    notify(
      "warning",
      code === "notCancellable"
        ? "notifications.cancelNotAllowed"
        : "notifications.cancelFailed",
    );
  }
}

export function useStore() {
  const packages = computed(() => {
    const needle = query.value.trim().toLowerCase();
    return catalog.value
      .filter(
        (entry) =>
          categoryFilter.value === "all" ||
          entry.category === categoryFilter.value,
      )
      .filter(
        (entry) =>
          needle === "" ||
          entry.id.includes(needle) ||
          entry.developer.toLowerCase().includes(needle),
      )
      .map(view);
  });

  const selected = computed(() => {
    const entry = catalog.value.find((item) => item.id === selectedId.value);
    return entry ? view(entry) : null;
  });

  return {
    catalog: readonly(catalog),
    installed: readonly(installed),
    loading: readonly(loading),
    loadError: readonly(loadError),
    packages,
    selected,
    selectedId,
    categoryFilter,
    query,
    initialize,
    install,
    cancelInstall,
    select: (id: string | null) => {
      selectedId.value = id;
    },
  };
}
