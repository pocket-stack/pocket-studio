import { computed, readonly, ref, watch } from "vue";
import { useI18n } from "vue-i18n";

import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { notify } from "../../shared/composables/useNotifications";
import {
  trackOperation,
  hydratePackageJobs,
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
  type PackageAction,
  type PackageJob,
  type PackageRequest,
  type RuntimeDelivery,
  type PackageCategory,
  type Platform,
} from "../../shared/gateway";
import {
  evaluateCompatibility,
  installationForms,
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
/** Storefront platform, like iTunes' iPhone/iPad switch; follows the device. */
const platformFilter = ref<Platform>("ios");
const compatibleOnly = ref(false);
const query = ref("");
/** package id -> most recent install operation id */
const installOperations = ref(new Map<string, string>());
const packageJobs = ref<PackageJob[]>([]);
/** Packages whose native plan is being resolved and submitted. */
const pendingIds = ref<string[]>([]);
/** 3DS package waiting for the user to pick an installation form. */
const deliveryAppId = ref<string | null>(null);
let jobsRequest = 0;
let nativeSubscribed = false;
let jobsQueued = false;

async function refreshJobs(): Promise<void> {
  const request = ++jobsRequest;
  try {
    const jobs = await useGateway().store.jobs();
    if (request !== jobsRequest) return;
    packageJobs.value = jobs;
    hydratePackageJobs(jobs);
  } catch {
    notify("warning", "store.actions.jobsUnavailable");
  }
}
function scheduleJobs(): void {
  if (jobsQueued) return;
  jobsQueued = true;
  queueMicrotask(() => {
    jobsQueued = false;
    void refreshJobs();
  });
}
async function verifyJob(job: {
  handle: { operationId: string };
}): Promise<void> {
  if (!device.value) return;
  try {
    await useGateway().store.verify(job.handle.operationId, device.value.id);
    // Reverification owns the same journal record; hydrate it as active.
    const existing = get(job.handle.operationId);
    if (existing) existing.status = "running";
    await refreshJobs();
  } catch {
    notify("warning", "store.actions.verificationUnavailable");
  }
}

let initialized = false;
let watchingSession = false;

const { device, readiness } = useDeviceSession();
const { get, cancel, active } = useOperations();
watch(
  () => device.value?.platform,
  (platform) => {
    if (platform) platformFilter.value = platform;
  },
);
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
  if (!nativeSubscribed) {
    await useGateway().operations.onEvent((event) => {
      if (event.type !== "progress") scheduleJobs();
      if (["finished", "failed", "cancelled"].includes(event.type))
        void refreshInstalled();
    });
    nativeSubscribed = true;
  }
  await refreshCatalog();
  await refreshInstalled();
  await refreshJobs();
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
        void refreshCatalog(false);
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
  /** A native plan for this package is being resolved and submitted. */
  pending: boolean;
}

function view(entry: CatalogEntry): PackageView {
  const nativeJob = packageJobs.value.find(
    (job) =>
      job.plan.appId === entry.id && job.plan.deviceId === device.value?.id,
  );
  // 3DS instances are planned natively even in the browser demo.
  const planned =
    useGateway().flavor === "tauri" || device.value?.platform === "3ds";
  const operationId = planned
    ? nativeJob?.handle.operationId
    : installOperations.value.get(entry.id);
  const installedIds = new Set(installed.value.map((item) => item.packageId));
  return {
    entry,
    queuePosition: planned
      ? nativeJob?.phase === "queued"
        ? packageJobs.value
            .filter((job) => job.phase === "queued")
            .sort((a, b) => a.queueOrder - b.queueOrder)
            .findIndex(
              (job) => job.handle.operationId === nativeJob.handle.operationId,
            ) + 1
        : undefined
      : queuedIds.value.includes(entry.id)
        ? queuedIds.value.indexOf(entry.id) + 1
        : startingId.value === entry.id
          ? 1
          : undefined,
    verdict: evaluateCompatibility(entry, device.value, readiness.value),
    installed: installed.value.find(
      (item) =>
        item.packageId === entry.id &&
        (useGateway().flavor !== "tauri" ||
          !item.native ||
          item.native.bundleId === catalogBundleId(entry)),
    ),
    operation: operationId ? get(operationId) : undefined,
    missingDependencies: entry.dependencies.filter(
      (dependency) => !installedIds.has(dependency),
    ),
    pending: pendingIds.value.includes(entry.id),
  };
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
  if (useGateway().flavor === "tauri") {
    const job = packageJobs.value.find(
      (job) =>
        job.plan.appId === packageId &&
        job.plan.deviceId === device.value?.id &&
        ["queued", "running", "verifying"].includes(job.phase),
    );
    if (job) await cancelJob(job);
    return;
  }
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

async function cancelJob(job: {
  handle: { operationId: string };
}): Promise<void> {
  try {
    await cancel(job.handle.operationId);
    await refreshJobs();
  } catch {
    notify("warning", "notifications.cancelNotAllowed");
  }
}

export function useStore() {
  const { t, te, locale } = useI18n();
  const allPackages = computed(() => catalog.value.map(view));
  const packages = computed(() => {
    const needle = query.value.trim().toLowerCase();
    return catalog.value
      .filter((entry) => entry.details?.app.listing !== "unlisted")
      .filter((entry) => entry.compatibility.platform === platformFilter.value)
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

  /**
   * Native application operations resolve a device-bound plan and submit it
   * in one go; the plan's uncertainties surface as notices instead of a
   * confirmation dialog. Consent mirrors the plan: removing an iOS app takes
   * its data with it, a 3DS instance keeps its data unless asked otherwise.
   */
  async function runAction(
    appId: string,
    action: PackageAction,
    options: Omit<PackageRequest, "deviceId" | "appId" | "action"> = {},
  ): Promise<void> {
    const current = device.value;
    if (!current || pendingIds.value.includes(appId)) return;
    pendingIds.value = [...pendingIds.value, appId];
    try {
      const plan = await useGateway().store.plan({
        deviceId: current.id,
        appId,
        action,
        ...options,
      });
      if (plan.deviceId !== device.value?.id)
        throw new GatewayError("deviceChanged", "device changed");
      const installation = plan.installation;
      if (
        action !== "uninstall" &&
        installation.platform === "ios" &&
        (installation.appsync === "unknown" ||
          installation.jailbreak === "unknown")
      )
        notify("warning", "store.actions.requirementsUnconfirmed");
      await useOperations().ready();
      const handle = await useGateway().store.start({
        planId: plan.id,
        deleteData: plan.deleteData,
      });
      const operation = trackOperation(handle);
      operation.packagePlan = plan;
      await refreshJobs();
      await refreshInstalled();
    } catch (error) {
      const code = error instanceof GatewayError ? error.code : "unknown";
      const key = `store.actions.errors.${code}`;
      notify("error", te(key) ? key : "store.actions.errors.unknown");
    } finally {
      pendingIds.value = pendingIds.value.filter((id) => id !== appId);
    }
  }

  // Same version with an unknown or equal revision is a reinstall: the native
  // layer never treats an unknown installed revision as older than the catalog.
  function nextAction(
    previous: InstalledPackage,
    entry: CatalogEntry | undefined,
  ): PackageAction {
    const revision = previous.managed?.revision ?? previous.revision;
    return previous.version === entry?.version &&
      (revision == null || revision === entry?.details?.revision)
      ? "reinstall"
      : "update";
  }

  /**
   * A 3DS title may live in the launcher and as a standalone app at once.
   * With an instance id the operation targets that instance; otherwise a
   * single published form installs directly and several forms ask the user.
   */
  async function install(
    packageId: string,
    installationId?: string,
  ): Promise<void> {
    const entry = catalog.value.find((item) => item.id === packageId);
    if (device.value?.platform === "3ds") {
      const forms = installationForms(entry);
      const previous = installationId
        ? installed.value.find(
            (record) =>
              record.installationId === installationId &&
              record.packageId === packageId,
          )
        : forms.length === 1
          ? installed.value.find(
              (record) =>
                record.packageId === packageId &&
                record.managed?.delivery === forms[0]!.delivery &&
                record.managed.format === forms[0]!.format,
            )
          : undefined;
      if (installationId && !previous) return;
      if (previous)
        return runAction(packageId, nextAction(previous, entry), {
          installationId: previous.installationId,
        });
      if (forms.length === 1)
        return runAction(packageId, "install", {
          delivery: forms[0]!.delivery,
          format: forms[0]!.format,
        });
      deliveryAppId.value = packageId;
      return;
    }
    if (useGateway().flavor === "tauri") {
      const previous = installed.value.find(
        (record) =>
          record.packageId === packageId &&
          (!record.native || record.native.bundleId === catalogBundleId(entry)),
      );
      const action: PackageAction = !previous
        ? "install"
        : !previous.revision ||
            previous.artifactId === entry?.details?.artifactId
          ? "reinstall"
          : "update";
      return runAction(packageId, action, {
        installationId: previous?.installationId ?? null,
      });
    }
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

  async function uninstall(
    packageId: string,
    installationId: string | null = null,
    deleteData = false,
  ): Promise<void> {
    if (!device.value) return;
    if (useGateway().flavor === "tauri" || device.value.platform === "3ds")
      return runAction(packageId, "uninstall", { installationId, deleteData });
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
  }

  return {
    packageJobs: readonly(packageJobs),
    pendingIds: readonly(pendingIds),
    deliveryAppId: readonly(deliveryAppId),
    deliveryEntry: computed(() =>
      catalog.value.find((entry) => entry.id === deliveryAppId.value),
    ),
    closeDelivery: () => {
      deliveryAppId.value = null;
    },
    chooseDelivery: async (delivery: RuntimeDelivery, format: string) => {
      const appId = deliveryAppId.value;
      if (!appId) return;
      deliveryAppId.value = null;
      await runAction(appId, "install", { delivery, format });
    },
    verifyJob,
    cancelJob,
    refreshJobs,
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
    platformFilter,
    compatibleOnly,
    query,
    initialize,
    install,
    cancelInstall,
    uninstall,
    select: (id: string | null) => {
      selectedId.value = id;
    },
  };
}

function catalogBundleId(entry: CatalogEntry | undefined): string | null {
  const identity = entry?.details?.nativeIdentity;
  return identity?.kind === "ios_bundle" ? identity.bundle_id : null;
}
