import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { createRenderer, nextTick, type Component } from "vue";
import {
  buildJailbreakPlan,
  demoDevice,
  demoCatalog,
} from "../../../shared/gateway/fixtures";
import type { ReadinessReport } from "../../../shared/gateway";
import AppSyncCheckDialog from "../../device/AppSyncCheckDialog.vue";
import ReadinessPanel from "../../device/ReadinessPanel.vue";
import PackageDetail from "../../store/PackageDetail.vue";
import OverviewStep from "./OverviewStep.vue";
import RiskStep from "./RiskStep.vue";
import DisclaimerStep from "./DisclaimerStep.vue";
import DfuGuideStep from "./DfuGuideStep.vue";
import ResultStep from "./ResultStep.vue";

const gateway = vi.hoisted(() => ({
  flavor: "tauri",
  capabilities: { demo: false, preparation: true, packages: true },
  demo: { setDeviceMode: vi.fn() },
  operations: { onEvent: () => () => {} },
  devices: { checkAppSync: vi.fn().mockResolvedValue({}) },
  preparation: { start: vi.fn() },
}));
vi.mock("../../../shared/gateway", () => ({
  useGateway: () => gateway,
  GatewayError: class extends Error {
    code = "appSyncCheckUnavailable";
  },
}));
vi.mock("vue-i18n", () => ({
  useI18n: () => ({
    t: (key: string) => key,
    tm: () => [],
    d: () => "",
    locale: { value: "zh-CN" },
    te: (key: string) => key.includes("signatureRejected"),
  }),
}));

// Exercise actual component render output and lifecycle in a memory renderer.
// No browser or native device commands are involved in these timer tests.
interface Node {
  tag: string;
  text: string;
  props: Record<string, unknown>;
  parent: Node | null;
  children: Node[];
}
const node = (tag: string, text = ""): Node => ({
  tag,
  text,
  props: {},
  parent: null,
  children: [],
});

function insertNode(node: Node, parent: Node, anchor: Node | null = null) {
  node.parent = parent;
  const index = anchor ? parent.children.indexOf(anchor) : -1;
  if (index < 0) parent.children.push(node);
  else parent.children.splice(index, 0, node);
}

it("offers preparation for an identified DFU device despite unknown OS and pairing", () => {
  const device = {
    ...demoDevice,
    mode: "dfu" as const,
    osVersion: null,
    batteryPercent: null,
  };
  const report: ReadinessReport = {
    deviceId: device.id,
    status: "needsAttention",
    checks: [],
    checkedAt: Date.now(),
  };
  const root = mount(ReadinessPanel, { device, report, checking: false });
  expect(button(root, "preparation.reviewPlan")).toBeDefined();
});

it("does not offer DFU preparation for unidentified A4 hardware", () => {
  const device = {
    ...demoDevice,
    mode: "dfu" as const,
    modelIdentifier: null,
    boardConfig: null,
    osVersion: null,
  };
  const report: ReadinessReport = {
    deviceId: device.id,
    status: "needsAttention",
    checks: [],
    checkedAt: Date.now(),
  };
  const root = mount(ReadinessPanel, { device, report, checking: false });
  expect(button(root, "preparation.reviewPlan")).toBeUndefined();
});

it("shows the DFU limitations and omits the button prerequisite in its overview", () => {
  const root = mount(OverviewStep, {
    plan: buildJailbreakPlan("demo", 1, "dfu"),
    confirmed: [],
    allConfirmed: false,
  });
  expect(text(root)).toContain("preparation.overview.dfuEntry");
  expect(text(root)).not.toContain("preparation.overview.intro");
  expect(text(root)).toContain("preparation.overview.facts.requiredTarget");
  expect(text(root)).not.toContain(
    "preparation.prerequisites.workingButtons.title",
  );
  expect(text(root)).not.toContain("preparation.steps.enterDfu.title");
});
const renderer = createRenderer<Node, Node>({
  createElement: (tag) => node(tag),
  createText: (text) => node("text", text),
  createComment: () => node("comment"),
  setText: (node, text) => {
    node.text = text;
  },
  setElementText: (node, text) => {
    node.text = text;
    node.children = [];
  },
  parentNode: (node) => node.parent,
  nextSibling: (node) =>
    node.parent?.children[node.parent.children.indexOf(node) + 1] ?? null,
  patchProp: (node, key, _previous, next) => {
    node.props[key] = next;
  },
  insert: insertNode,
  insertStaticContent: (content, parent, anchor) => {
    // SVG components can contain compiler-hoisted markup with no event handlers.
    const staticNode = node("static", content);
    insertNode(staticNode, parent, anchor);
    return [staticNode, staticNode];
  },
  remove: (node) => {
    if (node.parent)
      node.parent.children.splice(node.parent.children.indexOf(node), 1);
  },
});
const text = (node: Node): string =>
  node.text + node.children.map(text).join("");
function find(root: Node, predicate: (node: Node) => boolean): Node[] {
  return [
    ...(predicate(root) ? [root] : []),
    ...root.children.flatMap((child) => find(child, predicate)),
  ];
}
let unmount: () => void;
let windowEvents: EventTarget;
function mount(component: Component, props: Record<string, unknown> = {}) {
  const root = node("root");
  const app = renderer.createApp(component, props);
  app.mount(root);
  unmount = () => app.unmount();
  return root;
}
function button(root: Node, label: string): Node {
  return find(
    root,
    (node) => node.tag === "button" && text(node).includes(label),
  )[0]!;
}
function click(node: Node): void {
  (node.props.onClick as () => void)();
}
// Forced reading is paged: every page must be turned to before confirming.
async function turnPages(root: Node): Promise<void> {
  for (let guard = 0; guard < 20; guard += 1) {
    const next = button(root, "reading.next");
    if (!next || next.props.disabled) return;
    click(next);
    await nextTick();
  }
}

beforeEach(() => {
  vi.useFakeTimers({
    toFake: [
      "setInterval",
      "clearInterval",
      "setTimeout",
      "clearTimeout",
      "performance",
    ],
  });
  windowEvents = new EventTarget();
  vi.stubGlobal(
    "window",
    Object.assign(windowEvents, {
      setInterval,
      clearInterval,
      setTimeout,
      clearTimeout,
    }),
  );
  vi.stubGlobal(
    "document",
    Object.assign(new EventTarget(), {
      visibilityState: "hidden",
      hasFocus: () => false,
    }),
  );
  gateway.capabilities.demo = false;
  gateway.demo.setDeviceMode.mockReset();
});
afterEach(() => {
  unmount?.();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  vi.useRealTimers();
});

it.each([
  ["risks", RiskStep, "preparation.risks.continue", "onNext"],
  [
    "disclaimer",
    DisclaimerStep,
    "preparation.disclaimer.acceptAndStart",
    "onAccept",
  ],
] as const)(
  "enables %s with a single confirmation after five background seconds",
  async (_name, component, label, event) => {
    const confirm = vi.fn();
    const root = mount(component, {
      plan: buildJailbreakPlan("demo", 1),
      startError: null,
      [event]: confirm,
    });
    expect(find(root, (node) => node.tag === "input")).toHaveLength(0);
    expect(button(root, label).props.disabled).toBe(true);
    await vi.advanceTimersByTimeAsync(4999);
    expect(button(root, label).props.disabled).toBe(true);
    await vi.advanceTimersByTimeAsync(1);
    if (_name === "risks") {
      // Time alone is not enough: unread pages keep the confirmation locked.
      expect(button(root, label).props.disabled).toBe(true);
    }
    await turnPages(root);
    expect(button(root, label).props.disabled).toBe(false);
    click(button(root, label));
    expect(confirm).toHaveBeenCalledExactlyOnceWith(5);
  },
);

it("catches up confirmation time after background callbacks were throttled", async () => {
  const root = mount(RiskStep, { plan: buildJailbreakPlan("demo", 1) });
  await turnPages(root);
  vi.spyOn(performance, "now").mockReturnValue(8000);
  windowEvents.dispatchEvent(new Event("focus"));
  await nextTick();
  expect(button(root, "preparation.risks.continue").props.disabled).toBe(false);
});

it("guides power-off, ten seconds with both buttons and eight with Home without faking native DFU", async () => {
  const root = mount(DfuGuideStep);
  expect(text(root)).toContain("preparation.dfu.phase.idle.title");
  expect(text(root)).not.toContain("preparation.dfu.alreadyInDfu");
  click(button(root, "preparation.dfu.start"));
  await nextTick();
  expect(text(root)).toContain("preparation.dfu.phase.holdBoth.title");
  await vi.advanceTimersByTimeAsync(9999);
  expect(text(root)).toContain("preparation.dfu.phase.holdBoth.title");
  await vi.advanceTimersByTimeAsync(1);
  expect(text(root)).toContain("preparation.dfu.phase.holdHome.title");
  await vi.advanceTimersByTimeAsync(8000);
  expect(text(root)).toContain("preparation.dfu.phase.detecting.title");
  await vi.advanceTimersByTimeAsync(2000);
  expect(gateway.demo.setDeviceMode).not.toHaveBeenCalled();
});

it("catches up the DFU phase after background callbacks were throttled", async () => {
  const root = mount(DfuGuideStep);
  click(button(root, "preparation.dfu.start"));
  vi.spyOn(performance, "now").mockReturnValue(12_000);
  windowEvents.dispatchEvent(new Event("focus"));
  await nextTick();
  expect(text(root)).toContain("preparation.dfu.phase.holdHome.title");
  expect(gateway.demo.setDeviceMode).not.toHaveBeenCalled();
});

it("offers a read-only check instead of rerunning installation after verification timeout", () => {
  const onRecheck = vi.fn();
  const onRetry = vi.fn();
  const root = mount(ResultStep, {
    outcome: "failed",
    attempt: 1,
    operation: {
      id: "verify-timeout",
      kind: "preparation",
      status: "failed",
      startedAt: 1,
      steps: buildJailbreakPlan("demo", 1).steps.map((step) => ({
        ...step,
        status: step.id === "verifyJailbreak" ? "failed" : "done",
        percent: step.id === "verifyJailbreak" ? 0 : 100,
      })),
      error: { code: "verificationUnavailable", recoverable: false },
    },
    onRecheck,
    onRetry,
  });
  expect(text(root)).toContain(
    "preparation.errors.verificationUnavailable.title",
  );
  expect(text(root)).toContain("preparation.result.recheckHint");
  expect(text(root)).not.toContain("preparation.result.notRecoverable");
  expect(button(root, "preparation.result.retry")).toBeUndefined();
  click(button(root, "preparation.result.recheck"));
  expect(onRecheck).toHaveBeenCalledOnce();
  expect(onRetry).not.toHaveBeenCalled();
});

it("keeps a failed system write separate from a read-only verification check", () => {
  const root = mount(ResultStep, {
    outcome: "failed",
    attempt: 1,
    operation: {
      id: "write-failure",
      kind: "preparation",
      status: "failed",
      startedAt: 1,
      steps: [{ id: "installUntether", status: "failed", percent: 0 }],
      error: { code: "writeFailed", recoverable: false },
    },
  });
  expect(button(root, "preparation.result.recheck")).toBeUndefined();
  expect(text(root)).toContain("preparation.result.notRecoverable");
});

it.each([
  ["risks", RiskStep, "preparation.risks.continue", "onNext"],
  [
    "disclaimer",
    DisclaimerStep,
    "preparation.disclaimer.acceptAndStart",
    "onAccept",
  ],
] as const)(
  "enables standalone AppSync %s without a countdown and still requires a click",
  async (_name, component, label, event) => {
    const confirm = vi.fn();
    const plan = {
      ...buildJailbreakPlan("demo", 1),
      workflow: "appSync",
      minimumReadingSeconds: { risks: 0, disclaimer: 0 },
    };
    const root = mount(component, { plan, startError: null, [event]: confirm });
    await nextTick();
    await turnPages(root);
    expect(button(root, label).props.disabled).toBe(false);
    expect(confirm).not.toHaveBeenCalled();
    expect(text(root)).not.toContain("reading.remaining");
    click(button(root, label));
    expect(confirm).toHaveBeenCalledExactlyOnceWith(0);
  },
);

it("offers a read-only check instead of installation when AppSync status is unknown", () => {
  const root = mount(ReadinessPanel, {
    device: demoDevice,
    checking: false,
    report: {
      deviceId: demoDevice.id,
      status: "needsAttention",
      requiredWorkflow: "appSync",
      checkedAt: Date.now(),
      checks: [
        { id: "jailbroken", status: "pass" },
        { id: "appSyncInstalled", status: "unknown" },
        { id: "sshAvailable", status: "pass" },
      ],
    },
  });
  expect(button(root, "readiness.appSyncCheck.action")).toBeDefined();
  expect(button(root, "preparation.appSync.action")).toBeUndefined();
  expect(button(root, "preparation.reviewPlan")).toBeUndefined();
});

it("shows the device signature diagnosis and links to preparation from a failed store install", () => {
  const prepare = vi.fn();
  const root = mount(PackageDetail, {
    onPrepare: prepare,
    item: {
      entry: demoCatalog[0],
      verdict: "compatible",
      installed: null,
      missingDependencies: [],
      operation: {
        id: "failed-install",
        kind: "install",
        status: "failed",
        startedAt: 1,
        steps: [],
        error: {
          code: "installRejected",
          recoverable: false,
          diagnostic: { stage: "install", reason: "signatureRejected" },
        },
      },
    },
  });
  expect(text(root)).toContain("store.errors.signatureRejected.title");
  expect(text(root)).toContain("store.actions.errors.signatureRejected");
  expect(text(root)).not.toContain("store.errors.installRejected.body");
  click(button(root, "preparation.appSync.action"));
  expect(prepare).toHaveBeenCalledOnce();
});

it("prominently asks for a manual reboot after activation failure without offering reinstall", () => {
  const recheck = vi.fn();
  const retry = vi.fn();
  const root = mount(ResultStep, {
    outcome: "failed",
    attempt: 1,
    workflow: "appSync",
    onRecheck: recheck,
    onRetry: retry,
    operation: {
      id: "activation-pending",
      kind: "preparation",
      status: "failed",
      startedAt: 1,
      steps: [{ id: "activateAppSync", status: "failed", percent: 0 }],
      error: { code: "appSyncRestartRequired", recoverable: false },
    },
  });
  expect(text(root)).toContain(
    "preparation.errors.appSyncRestartRequired.title",
  );
  expect(find(root, (node) => node.props.role === "alert").map(text)).toEqual([
    "preparation.appSync.restartInstructions",
  ]);
  expect(button(root, "preparation.result.retry")).toBeUndefined();
  expect(recheck).not.toHaveBeenCalled();
  click(button(root, "preparation.appSync.recheckAfterRestart"));
  expect(recheck).toHaveBeenCalledOnce();
  expect(retry).not.toHaveBeenCalled();
});

it("keeps a previous installation visibly separate from current verification", () => {
  const root = mount(ReadinessPanel, {
    device: demoDevice,
    checking: false,
    report: {
      deviceId: demoDevice.id,
      status: "needsAttention",
      requiredWorkflow: "appSync",
      checkedAt: 2,
      checks: [
        {
          id: "appSyncInstalled",
          status: "warn",
          previousObservation: { installed: true, observedAt: 1 },
        },
        { id: "sshAvailable", status: "pass" },
      ],
    },
  });
  expect(text(root)).toContain("readiness.appSyncCheck.lastInstalled");
  expect(text(root)).toContain("readiness.appSyncCheck.previousInstalled");
  expect(text(root)).not.toContain("readiness.checks.appSyncInstalled.fail");
  expect(button(root, "preparation.appSync.action")).toBeUndefined();
});

it("only authenticates and reads AppSync after an explicit verification submit", async () => {
  gateway.devices.checkAppSync.mockClear();
  const checked = vi.fn();
  const root = mount(AppSyncCheckDialog, {
    open: true,
    deviceId: "selected-device",
    onChecked: checked,
  });
  expect(gateway.devices.checkAppSync).not.toHaveBeenCalled();
  const input = find(root, (n) => n.tag === "input")[0]!;
  (input.props.onInput as (event: unknown) => void)({
    target: { value: "test-password" },
  });
  const form = find(root, (n) => n.tag === "form")[0]!;
  await (form.props.onSubmit as (event: unknown) => Promise<void>)({
    preventDefault() {},
  });
  await nextTick();
  expect(gateway.devices.checkAppSync).toHaveBeenCalledExactlyOnceWith(
    "selected-device",
    "test-password",
  );
  expect(checked).toHaveBeenCalledOnce();
  expect(input.props.value).toBe("");
  expect(gateway.preparation.start).not.toHaveBeenCalled();
});

it("falls back to the default root password when the AppSync field is left empty", async () => {
  gateway.devices.checkAppSync.mockClear();
  const root = mount(AppSyncCheckDialog, { open: true, deviceId: "device" });
  const input = find(root, (n) => n.tag === "input")[0]!;
  expect(input.props.value).toBe("");
  expect(input.props.placeholder).toBe("alpine");
  const form = find(root, (n) => n.tag === "form")[0]!;
  await (form.props.onSubmit as (event: unknown) => Promise<void>)({
    preventDefault() {},
  });
  await nextTick();
  expect(gateway.devices.checkAppSync).toHaveBeenCalledExactlyOnceWith(
    "device",
    "alpine",
  );
});
