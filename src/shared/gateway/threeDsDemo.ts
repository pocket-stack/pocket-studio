import type {
  CatalogCandidate,
  CatalogEntry,
  DeviceSummary,
  DiscoverySnapshot,
  InstalledPackage,
  PackageCategory,
  PackageConsent,
  PackageJob,
  PackagePlan,
  PackageRequest,
  ReadinessReport,
  RuntimeDelivery,
  SetupPlan,
  SetupRequest,
  SetupResult,
  StoreApplication,
  StoreArtifact,
  StoreRelease,
  StoreTarget,
  StoreVerdict,
} from "./types";
import { GatewayError } from "./types";
import { demoDevice3ds, installSteps } from "./fixtures";

/**
 * In-memory stand-in for the native 3DS management layer. It mirrors the
 * Rust contract — installation instances, delivery candidates, device-bound
 * plans and one-shot consent — without opening a socket or writing a card.
 */

type ThreeDsFormat = "pocket" | "cia" | "3dsx";

interface DemoTitle {
  id: string;
  category: PackageCategory;
  /** Published artifact formats; `.pocket` runs inside the launcher. */
  formats: [ThreeDsFormat, ...ThreeDsFormat[]];
  sizeBytes: number;
  publishedAt: number;
  developer: string;
  /** The launcher is upgraded through device preparation, never the store. */
  launcher?: boolean;
  locales: StoreApplication["locales"];
}

const RUNTIME_ID = "pocketjs-3ds";
const RUNTIME_MIN_VERSION = "0.11.0";
const LAUNCHER_APP_ID = "pocket-runtime-ctr";
const TARGET_ID = "3ds-new";
const MODELS: [string, ...string[]] = ["KTR", "RED", "JAN"];

const titles: DemoTitle[] = [
  {
    id: LAUNCHER_APP_ID,
    category: "runtime",
    formats: ["cia", "3dsx"],
    sizeBytes: 5_900_000,
    publishedAt: Date.UTC(2026, 8, 3),
    developer: "PocketJS",
    launcher: true,
    locales: {
      en: {
        name: "Pocket Runtime",
        summary: "The Pocket runtime built for the New 3DS family.",
        description:
          "Brings the Pocket JavaScript runtime and system bridge to the New Nintendo 3DS. Installed as a CIA title or a 3DSX homebrew app on a console with custom firmware; it drives both screens and hosts .pocket applications.",
      },
      "zh-CN": {
        name: "Pocket Runtime",
        summary: "面向 New 3DS 系列的 Pocket 运行时。",
        description:
          "把 Pocket 的 JavaScript 运行时与系统桥接带到 New Nintendo 3DS。以 CIA 标题或 3DSX 自制软件的形式安装在已有自制系统的主机上，驱动双屏并承载 .pocket 应用。",
      },
    },
  },
  {
    id: "pocket-agent-ctr",
    category: "tool",
    formats: ["pocket"],
    sizeBytes: 1_400_000,
    publishedAt: Date.UTC(2026, 8, 5),
    developer: "PocketJS",
    locales: {
      en: {
        name: "Pocket Agent",
        summary: "Keeps the console connected to Pocket Studio over Wi-Fi.",
        description:
          "A launcher module that receives install commands from Pocket Studio over the local network, reports console state and keeps the runtime updated.",
      },
      "zh-CN": {
        name: "Pocket Agent",
        summary: "通过 Wi-Fi 保持主机与 Pocket Studio 的连接。",
        description:
          "运行在启动器中的模块，通过局域网接收 Pocket Studio 的安装指令、上报主机状态并保持运行时更新。",
      },
    },
  },
  {
    id: "ctr-pixel-arcade",
    category: "game",
    formats: ["pocket", "cia"],
    sizeBytes: 24_000_000,
    publishedAt: Date.UTC(2026, 7, 18),
    developer: "Pocket Labs",
    locales: {
      en: {
        name: "Pixel Arcade",
        summary: "Open-source retro games tuned for two screens.",
        description:
          "A curated set of open-source retro games with the action on the top screen and controls or maps on the touch screen. Save games sync through Pocket Runtime.",
      },
      "zh-CN": {
        name: "Pixel Arcade",
        summary: "为双屏调校的开源复古游戏合集。",
        description:
          "精选的开源复古游戏合集：上屏显示画面，触摸屏承担操作或地图。存档通过 Pocket 运行时同步。",
      },
    },
  },
  {
    id: "ctr-dual-notebook",
    category: "app",
    formats: ["pocket", "cia", "3dsx"],
    sizeBytes: 7_600_000,
    publishedAt: Date.UTC(2026, 7, 26),
    developer: "Pocket Labs",
    locales: {
      en: {
        name: "Dual Notebook",
        summary: "Stylus notes on the bottom, pages on the top.",
        description:
          "Handwritten and typed notes that use the stylus on the touch screen while the top screen shows the page you are editing. Notebooks sync with other Pocket devices.",
      },
      "zh-CN": {
        name: "Dual Notebook",
        summary: "下屏手写，上屏看页。",
        description:
          "用触控笔在触摸屏上手写或输入笔记，上屏同步显示正在编辑的页面。笔记本会与其他 Pocket 设备同步。",
      },
    },
  },
  {
    id: "ctr-pocket-sync",
    category: "tool",
    formats: ["3dsx"],
    sizeBytes: 900_000,
    publishedAt: Date.UTC(2026, 6, 30),
    developer: "Pocket Labs",
    locales: {
      en: {
        name: "Pocket Sync",
        summary:
          "Homebrew helper that mirrors your Pocket data to the SD card.",
        description:
          "A lightweight 3DSX homebrew app launched from the Homebrew Launcher. It mirrors Pocket data to the SD card and back without needing a CIA install.",
      },
      "zh-CN": {
        name: "Pocket Sync",
        summary: "把 Pocket 数据镜像到 SD 卡的自制小工具。",
        description:
          "从 Homebrew Launcher 启动的轻量 3DSX 自制软件，在 SD 卡与 Pocket 之间镜像数据，无需安装 CIA。",
      },
    },
  },
  {
    id: "ctr-theme-forge",
    category: "app",
    formats: ["cia"],
    sizeBytes: 11_200_000,
    publishedAt: Date.UTC(2026, 8, 1),
    developer: "Pocket Labs",
    locales: {
      en: {
        name: "Theme Forge",
        summary: "Build HOME menu themes from your Pocket library.",
        description:
          "Turns artwork and colours from your Pocket library into HOME menu themes for the 3DS. Installs as a CIA title; themes are written to the SD card only.",
      },
      "zh-CN": {
        name: "Theme Forge",
        summary: "用 Pocket 素材生成 HOME 菜单主题。",
        description:
          "把 Pocket 库中的插画与配色转换为 3DS 的 HOME 菜单主题。以 CIA 标题安装，主题只写入 SD 卡。",
      },
    },
  },
];

const runtimeRequirement: StoreTarget["runtime_requirement"] = {
  id: RUNTIME_ID,
  min_version: RUNTIME_MIN_VERSION,
  bootstrap_app_id: LAUNCHER_APP_ID,
};

function deliveries(
  format: ThreeDsFormat,
): [RuntimeDelivery, ...RuntimeDelivery[]] {
  return format === "pocket" ? ["shared", "bundled"] : ["bundled"];
}

function policy(format: ThreeDsFormat): CatalogEntry["installPolicy"] {
  return format === "3dsx" ? "threeDsx" : format;
}

function fakeDigest(seed: string): string {
  let hash = 0x811c9dc5;
  for (const char of seed)
    hash = ((hash ^ char.charCodeAt(0)) * 0x01000193) >>> 0;
  return hash.toString(16).padStart(8, "0").repeat(8);
}

function artifact(title: DemoTitle, format: ThreeDsFormat): StoreArtifact {
  const titleId = `000400000f${fakeDigest(title.id).slice(0, 6)}`;
  return {
    id: `${title.id}-${format}`,
    format,
    build_id: fakeDigest(`${title.id}/${format}`).slice(0, 40),
    native_identity:
      format === "cia"
        ? { kind: "3ds_title", title_id: titleId, title_version: 1 }
        : format === "3dsx"
          ? { kind: "homebrew_file", entrypoint: `3ds/${title.id}/boot.3dsx` }
          : null,
    blob: {
      sha256: fakeDigest(`${title.id}#${format}`),
      size_bytes: title.sizeBytes,
      content_type: "application/octet-stream",
    },
    provenance: {
      source_commit: fakeDigest(`${title.id}@source`).slice(0, 40),
      runtime_commit: null,
      manifest_sha256: fakeDigest(`${title.id}@manifest`),
    },
    targets: [
      {
        target_id: TARGET_ID,
        platform: "3ds",
        arch: "armv6k",
        models: MODELS,
        os: { min: "11.0.0", max: null, builds: [] },
        host_abi: title.launcher ? null : 1,
        runtime_deliveries: deliveries(format),
        installer_id:
          format === "pocket"
            ? "pocket-runtime"
            : format === "cia"
              ? "3ds-cia"
              : "3dsx-file",
        requires: { kind: "3ds", cfw: true },
        runtime_requirement: title.launcher ? null : runtimeRequirement,
        runtime_provides: title.launcher
          ? {
              id: RUNTIME_ID,
              version: "0.3.0",
              capabilities: [
                "guest-update",
                "app-library",
                "cia-management",
                "file-management",
              ],
            }
          : null,
      },
    ],
  };
}

function release(title: DemoTitle): StoreRelease {
  const [first, ...rest] = title.formats;
  return {
    id: `${title.id}-0.3.0`,
    app_id: title.id,
    version: "0.3.0",
    revision: 1,
    status: "published",
    notes: {
      en: "First New 3DS release.",
      "zh-CN": "首个 New 3DS 版本。",
    },
    published_at: title.publishedAt,
    artifacts: [artifact(title, first), ...rest.map((f) => artifact(title, f))],
  };
}

export interface ThreeDsDemoContext {
  /** The currently attached simulated device, if any. */
  device(): DeviceSummary | null;
  cfwInstalled(): boolean;
  /** Attach the demo 3DS as if a paired console answered on the network. */
  attach(): Promise<void>;
  snapshot(): Promise<DiscoverySnapshot>;
}

/** Readiness as the native layer reports it for a paired 3DS. */
export function threeDsReadiness(
  current: DeviceSummary,
  cfwInstalled: boolean,
): ReadinessReport {
  return {
    deviceId: current.id,
    status: cfwInstalled ? "ready" : "needsPreparation",
    checkedAt: Date.now(),
    checks: [
      { id: "platformSupported", status: "pass", value: "Nintendo 3DS" },
      {
        id: "modelSupported",
        status: "pass",
        value: current.modelIdentifier ?? undefined,
      },
      { id: "pairingTrusted", status: "pass" },
      {
        id: "cfwInstalled",
        status: cfwInstalled ? "pass" : "fail",
        value: cfwInstalled ? "Luma3DS 13.0.2" : undefined,
      },
      {
        id: "runtimeAvailable",
        status: cfwInstalled ? "pass" : "unknown",
        value: cfwInstalled
          ? `Pocket ${current.threeDs?.runtime.version ?? RUNTIME_MIN_VERSION}`
          : undefined,
      },
    ],
  };
}

export function createThreeDsDemo(context: ThreeDsDemoContext) {
  const records = new Map<string, InstalledPackage>();
  const plans = new Map<string, PackagePlan>();
  const jobs: PackageJob[] = [];
  const setupPlans = new Map<string, SetupPlan>();

  function verdict(title: DemoTitle): StoreVerdict {
    const current = context.device();
    if (!current) return "noDevice";
    if (current.platform !== "3ds") return "unsupportedModel";
    if (title.launcher || !context.cfwInstalled()) return "requiresPreparation";
    return "compatible";
  }

  function entry(title: DemoTitle): CatalogEntry {
    const current = release(title);
    const primary = current.artifacts[0];
    const state = verdict(title);
    const candidates: CatalogCandidate[] = current.artifacts.flatMap((item) =>
      item.targets[0].runtime_deliveries.map((delivery) => ({
        delivery,
        format: item.format,
        version: current.version,
        revision: current.revision,
        releaseId: current.id,
        artifactId: item.id,
        targetId: TARGET_ID,
        verdict: state,
        requiresExistingHost:
          item.format === "pocket" && delivery === "bundled",
        runtimeRequirement: item.targets[0].runtime_requirement,
        hostAbi: item.targets[0].host_abi,
      })),
    );
    return {
      id: title.id,
      version: current.version,
      developer: title.developer,
      category: title.category,
      sizeBytes: title.sizeBytes,
      installPolicy: policy(title.formats[0]),
      checksumSha256: primary.blob.sha256,
      signed: true,
      compatibility: {
        platform: "3ds",
        models: MODELS,
        minOsVersion: "11.0.0",
        maxOsVersion: null,
        requiresJailbreak: true,
      },
      dependencies: [],
      publishedAt: title.publishedAt,
      details: {
        app: {
          id: title.id,
          slug: title.id,
          listing: "listed",
          publisher: {
            id: title.developer === "PocketJS" ? "pocketjs" : "pocket-labs",
            name: title.developer,
            verified: title.developer === "PocketJS",
          },
          category: title.category,
          source: {
            repository: "https://github.com/Pocket-Stack/pocketjs",
            path: `apps/${title.id}`,
          },
          locales: title.locales,
          media: [],
        },
        releaseId: current.id,
        artifactId: primary.id,
        targetId: TARGET_ID,
        revision: current.revision,
        nativeIdentity: primary.native_identity,
        verdict: state,
        history: [current],
        candidates,
      },
    };
  }

  function requireTitle(appId: string): DemoTitle {
    const title = titles.find((item) => item.id === appId);
    if (!title) throw new GatewayError("unknownPackage", "package not found");
    return title;
  }

  function plan(request: PackageRequest): PackagePlan {
    const current = context.device();
    if (!current || current.id !== request.deviceId)
      throw new GatewayError("deviceNotFound", "device is not attached");
    if (!context.cfwInstalled())
      throw new GatewayError("deviceNotReady", "custom firmware is missing");
    const title = requireTitle(request.appId);
    if (title.launcher)
      throw new GatewayError(
        "invalidAction",
        "The launcher is managed through device preparation",
      );
    const previous = request.installationId
      ? records.get(request.installationId)
      : undefined;
    if (request.installationId && previous?.packageId !== title.id)
      throw new GatewayError("invalidAction", "Unknown installation instance");
    const managed = previous?.managed ?? null;
    const delivery = managed?.delivery ?? request.delivery;
    if (!delivery)
      throw new GatewayError("invalidAction", "Choose a delivery form");
    const format: ThreeDsFormat =
      request.action === "uninstall"
        ? (managed!.format as ThreeDsFormat)
        : managed
          ? title.formats.includes("pocket")
            ? "pocket"
            : (managed.format as ThreeDsFormat)
          : delivery === "shared"
            ? "pocket"
            : ((request.format ?? "cia") as ThreeDsFormat);
    if (request.action !== "uninstall" && !title.formats.includes(format))
      throw new GatewayError("unsupportedInstaller", "format not published");
    if (
      request.action === "install" &&
      delivery === "bundled" &&
      format === "pocket"
    )
      throw new GatewayError(
        "runtimeRequired",
        "A standalone host must exist before a .pocket update",
      );
    const selected = artifact(title, format);
    const installationId =
      previous?.installationId ?? `${title.id}:${delivery}:${format}`;
    if (request.action === "install" && records.has(installationId))
      throw new GatewayError("invalidAction", "This instance already exists");
    const result: PackagePlan = {
      id: crypto.randomUUID(),
      deviceId: current.id,
      deviceName: current.marketingName,
      appId: title.id,
      names: Object.fromEntries(
        Object.entries(title.locales).map(([locale, text]) => [
          locale,
          text.name,
        ]),
      ),
      action: request.action,
      installation: {
        platform: "3ds",
        installationId,
        delivery,
        format,
        expectedGeneration: managed?.generation ?? 0,
        previous: managed,
        nativeIdentity:
          selected.native_identity ?? managed?.nativeIdentity ?? null,
        updatesHost: format !== "pocket",
      },
      deleteData:
        request.action === "uninstall" && (request.deleteData ?? false),
      releaseId: release(title).id,
      artifact: request.action === "uninstall" ? null : selected,
      target: selected.targets[0],
      version: "0.3.0",
      revision: 1,
      publicationId: "demo",
      sequence: 1,
      catalogExpiresAt: Date.now() + 900_000,
      expiresAt: Date.now() + 900_000,
      steps: installSteps,
    };
    plans.set(result.id, result);
    return result;
  }

  function start(consent: PackageConsent) {
    const pending = plans.get(consent.planId);
    if (!pending)
      throw new GatewayError("planExpired", "Unknown or consumed plan");
    plans.delete(consent.planId);
    const managed = pending.installation;
    if (managed.platform !== "3ds")
      throw new GatewayError("invalidAction", "Not a 3DS plan");
    if (consent.deleteData !== pending.deleteData)
      throw new GatewayError("consentRequired", "Consent does not match");
    const current = context.device();
    if (!current || current.id !== pending.deviceId)
      throw new GatewayError("deviceChanged", "The device changed");
    if (pending.action === "uninstall") records.delete(managed.installationId);
    else {
      const title = requireTitle(pending.appId);
      const nativeFormat = managed.previous?.format ?? managed.format;
      records.set(managed.installationId, {
        installationId: managed.installationId,
        packageId: pending.appId,
        version: pending.version!,
        revision: pending.revision,
        releaseId: pending.releaseId,
        artifactId: pending.artifact!.id,
        installedAt: Date.now(),
        managed: {
          installationId: managed.installationId,
          appId: pending.appId,
          containerId:
            managed.delivery === "shared"
              ? "launcher"
              : `${nativeFormat}-${fakeDigest(pending.appId).slice(0, 16)}`,
          generation: managed.expectedGeneration + 1,
          format: nativeFormat,
          delivery: managed.delivery,
          installed: true,
          health: "untested",
          title: title.locales["zh-CN"]?.name ?? title.locales.en!.name,
          version: pending.version!,
          revision: pending.revision,
          buildId: pending.artifact!.build_id,
          guestSha256: fakeDigest(`${pending.appId}#pocket`),
          nativeVersion:
            managed.previous?.nativeVersion ??
            (managed.format !== "pocket" ? pending.version : null),
          nativeBuildId:
            managed.previous?.nativeBuildId ?? pending.artifact!.build_id,
          nativeIdentity: managed.nativeIdentity,
          runtimeId: RUNTIME_ID,
          runtimeVersion: current.threeDs?.runtime.version ?? null,
          hostAbi: current.threeDs?.hostAbi ?? null,
          unavailable: false,
        },
      });
    }
    const handle = {
      operationId: crypto.randomUUID(),
      kind:
        pending.action === "uninstall"
          ? ("uninstall" as const)
          : ("install" as const),
      steps: pending.steps,
      subject: pending.appId,
    };
    jobs.push({
      queueOrder: jobs.length + 1,
      handle,
      plan: pending,
      phase: "verified",
      step: "verifyInstall",
      completedSteps: pending.steps.map((step) => step.id),
      percent: 100,
      submitted: true,
      cleanupComplete: true,
      stagingPath: null,
      error: null,
      updatedAt: Date.now(),
    });
    return handle;
  }

  const setup = {
    async plan(request: SetupRequest): Promise<SetupPlan> {
      const format = request.format;
      const title = request.appId
        ? titles.find((item) => item.id === request.appId)
        : titles.find((item) => item.launcher);
      if (
        !title ||
        (request.appId && (!format || !title.formats.includes(format)))
      )
        throw new GatewayError("artifactUnavailable", "No card file");
      const file =
        format === "cia"
          ? title.launcher
            ? `cias/${title.id}.cia`
            : `cias/${title.id}-${fakeDigest(`${title.id}#cia`).slice(0, 16)}.cia`
          : format === "3dsx"
            ? `3ds/${title.id}/boot.3dsx`
            : null;
      const result: SetupPlan = {
        appId: title.id,
        id: crypto.randomUUID(),
        destination:
          request.destination.kind === "sd"
            ? request.destination.path
            : `ftpd ${request.destination.address}:${request.destination.port}`,
        existingPairing: false,
        artifact: format ? artifact(title, format) : null,
        version: format ? "0.3.0" : null,
        files: ["pocketjs/runtime/dev.key", ...(file ? [file] : [])],
        expiresAt: Date.now() + 900_000,
      };
      setupPlans.set(result.id, result);
      return result;
    },
    async execute(planId: string): Promise<SetupResult> {
      if (!setupPlans.delete(planId))
        throw new GatewayError("planExpired", "Unknown demo plan");
      return {
        pairingId: demoDevice3ds.id,
        filesVerified: true,
        restartRequired: true,
      };
    },
    async connect(pairingId: string): Promise<DiscoverySnapshot> {
      if (pairingId !== demoDevice3ds.id)
        throw new GatewayError("pairingUnavailable", "Unknown demo pairing");
      await context.attach();
      return context.snapshot();
    },
    // The demo console is always on the simulated network; nothing to probe.
    async hint(): Promise<void> {},
  };

  return {
    catalog: (): CatalogEntry[] => titles.map(entry),
    installed: (): InstalledPackage[] => [...records.values()],
    plan,
    start,
    jobs: (): PackageJob[] => jobs.slice(),
    setup,
  };
}
