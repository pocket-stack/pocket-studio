import { computed, readonly, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

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
  type CatalogSnapshot,
  type InstalledPackage,
  type InstalledSnapshot,
  type PackageCategory,
} from "../../shared/gateway";
import {
  evaluateCompatibility,
  type CompatibilityVerdict,
} from "./compatibility";
import { packageText } from "./packageContent";

const catalog = ref<CatalogEntry[]>([]);
const snapshot = ref<CatalogSnapshot | null>(null);
const installed = ref<InstalledPackage[]>([]);
const installedSnapshot = ref<InstalledSnapshot | null>(null);
const installedLoading = ref(false);
const loading = ref(false);
const loadError = ref<string | null>(null);
const installedIssue = ref<string | null>(null);
let catalogRequest = 0;
let installedRequest = 0;
let installedDeviceId: string | null = null;
const selectedId = ref<string | null>(null);
const categoryFilter = ref<PackageCategory | "all">("all");
const compatibleOnly = ref(false);
const query = ref("");
/** package id -> most recent install operation id */
const installOperations = ref(new Map<string, string>());
let initialized = false;
let watchingSession = false;

const { device, readiness } = useDeviceSession();
const { get, cancel, active } = useOperations();
const queuedIds = ref<string[]>([]);
const startingId = ref<string | null>(null);
watch(
  () => active.value.length,
  () => void processQueue(),
);

async function refreshInstalled(): Promise<void> {
  const request = ++installedRequest;
  const current = device.value;
  if (installedDeviceId !== (current?.id ?? null)) {
    installed.value = [];
    installedSnapshot.value = null;
    installedDeviceId = current?.id ?? null;
  }
  installedIssue.value = null;
  if (!current || !useGateway().capabilities.installed) {
    installedLoading.value = false;
    installed.value = [];
    return;
  }
  try {
    installedLoading.value = true;
    const result = await useGateway().store.installed(current.id);
    if (request === installedRequest && result.deviceId === current.id) {
      installed.value = result.entries;
      installedSnapshot.value = result;
      installedIssue.value = result.issue;
    }
  } catch (error) {
    if (request === installedRequest) {
      installedIssue.value =
        error instanceof GatewayError ? error.code : "unknown";
      if (installedSnapshot.value)
        installedSnapshot.value = {
          ...installedSnapshot.value,
          state: "stale",
          issue: installedIssue.value,
        };
    }
  } finally {
    if (request === installedRequest) installedLoading.value = false;
  }
}

async function refreshCatalog(refresh = true): Promise<void> {
  const request = ++catalogRequest;
  if (refresh) loading.value = true;
  loadError.value = null;
  try {
    const result = await useGateway().store.catalog(device.value?.id, refresh);
    if (request !== catalogRequest) return;
    snapshot.value = result;
    catalog.value = result.entries;
  } catch (error) {
    if (request === catalogRequest)
      loadError.value = error instanceof GatewayError ? error.code : "unknown";
  } finally {
    if (request === catalogRequest) loading.value = false;
  }
}

async function initialize(): Promise<void> {
  if (initialized) return;
  initialized = true;
  await refreshCatalog();
  await refreshInstalled();
  if (!watchingSession) {
    watchingSession = true;
    watch(
      () =>
        JSON.stringify([
          device.value?.id,
          device.value?.modelIdentifier,
          device.value?.osVersion,
          device.value?.buildNumber,
          device.value?.mode,
          readiness.value?.status,
        ]),
      () => {
        if (!device.value) {
          queuedIds.value = [];
          compatibleOnly.value = false;
        }
        if (useGateway().flavor === "tauri") void refreshCatalog(false);
        void refreshInstalled();
      },
    );
  }
}

export interface PackageView {
  entry: CatalogEntry;
  verdict: CompatibilityVerdict;
  installed?: InstalledPackage;
  operation?: OperationState;
  missingDependencies: string[];
  queuePosition?: number;
}

function view(entry: CatalogEntry): PackageView {
  const operationId = installOperations.value.get(entry.id);
  const installedIds = new Set(installed.value.map((item) => item.packageId));
  return {
    entry,
    queuePosition: queuedIds.value.includes(entry.id)
      ? queuedIds.value.indexOf(entry.id) + 1
      : startingId.value === entry.id
        ? 1
        : undefined,
    verdict: evaluateCompatibility(entry, device.value, readiness.value),
    installed: installed.value.find((item) => item.packageId === entry.id),
    operation: operationId ? get(operationId) : undefined,
    missingDependencies: entry.dependencies.filter(
      (dependency) => !installedIds.has(dependency),
    ),
  };
}

async function install(packageId: string): Promise<void> {
  if (
    !device.value ||
    startingId.value === packageId ||
    queuedIds.value.includes(packageId)
  )
    return;
  const previous = installOperations.value.get(packageId);
  if (previous && get(previous)?.status === "running") return;
  queuedIds.value.push(packageId);
  await processQueue();
}
async function processQueue(): Promise<void> {
  if (active.value.length || startingId.value || !queuedIds.value.length)
    return;
  const current = device.value;
  if (!current) {
    queuedIds.value = [];
    return;
  }
  const packageId = queuedIds.value.shift()!;
  startingId.value = packageId;
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
  } finally {
    startingId.value = null;
    if (!active.value.length && queuedIds.value.length) void processQueue();
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
  if (queuedIds.value.includes(packageId)) {
    queuedIds.value = queuedIds.value.filter((id) => id !== packageId);
    return;
  }
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
  const { t, locale } = useI18n();
  const allPackages = computed(() => catalog.value.map(view));
  const packages = computed(() => {
    const needle = query.value.trim().toLowerCase();
    return catalog.value
      .filter((entry) => entry.details?.app.listing !== "unlisted")
      .filter(
        (entry) =>
          !compatibleOnly.value ||
          evaluateCompatibility(entry, device.value, readiness.value) ===
            "compatible",
      )
      .filter(
        (entry) =>
          categoryFilter.value === "all" ||
          entry.category === categoryFilter.value,
      )
      .filter(
        (entry) =>
          needle === "" ||
          entry.id.includes(needle) ||
          packageText(entry, "name", locale.value, t)
            .toLowerCase()
            .includes(needle) ||
          packageText(entry, "summary", locale.value, t)
            .toLowerCase()
            .includes(needle) ||
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
    snapshot: readonly(snapshot),
    installed: readonly(installed),
    installedSnapshot: readonly(installedSnapshot),
    installedLoading: readonly(installedLoading),
    refreshInstalled,
    queuedIds: readonly(queuedIds),
    loading: readonly(loading),
    loadError: readonly(loadError),
    installedIssue: readonly(installedIssue),
    packages,
    allPackages,
    reload: async () => {
      initialized = false;
      await initialize();
    },
    selected,
    selectedId,
    categoryFilter,
    compatibleOnly,
    query,
    initialize,
    install,
    cancelInstall,
    uninstall: async (packageId: string) => {
      if (!device.value) return;
      try {
        await useGateway().store.uninstall(device.value.id, packageId);
        await refreshInstalled();
      } catch (error) {
        notify(
          "error",
          error instanceof GatewayError && error.code === "packageInUse"
            ? "studio.packageInUse"
            : "studio.uninstallFailed",
        );
      }
    },
    select: (id: string | null) => {
      selectedId.value = id;
    },
  };
}
