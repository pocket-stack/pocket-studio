import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { createRenderer, nextTick, type Component } from "vue";
import { buildJailbreakPlan } from "../../../shared/gateway/fixtures";
import RiskStep from "./RiskStep.vue";
import DisclaimerStep from "./DisclaimerStep.vue";
import DfuGuideStep from "./DfuGuideStep.vue";

const gateway = vi.hoisted(() => ({
  capabilities: { demo: false },
  demo: { setDeviceMode: vi.fn() },
}));
vi.mock("../../../shared/gateway", () => ({ useGateway: () => gateway }));
vi.mock("vue-i18n", () => ({
  useI18n: () => ({ t: (key: string) => key, tm: () => [] }),
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
  insert: (node, parent, anchor) => {
    node.parent = parent;
    const index = anchor ? parent.children.indexOf(anchor) : -1;
    if (index < 0) parent.children.push(node);
    else parent.children.splice(index, 0, node);
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
    expect(button(root, label).props.disabled).toBe(false);
    click(button(root, label));
    expect(confirm).toHaveBeenCalledExactlyOnceWith(5);
  },
);

it("catches up confirmation time after background callbacks were throttled", async () => {
  const root = mount(RiskStep, { plan: buildJailbreakPlan("demo", 1) });
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
