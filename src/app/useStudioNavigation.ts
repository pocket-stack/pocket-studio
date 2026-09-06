import { computed, ref } from "vue";

export type StudioView = "device" | "installed" | "logs" | "store";
export type DeviceSection = "summary" | "conditions" | "environment";
export type StudioLocation =
  | { view: "device"; section: DeviceSection }
  | { view: "store"; packageId: string | null }
  | { view: "installed" | "logs" };

function sameLocation(left: StudioLocation, right: StudioLocation): boolean {
  if (left.view !== right.view) return false;
  if (left.view === "device" && right.view === "device")
    return left.section === right.section;
  if (left.view === "store" && right.view === "store")
    return left.packageId === right.packageId;
  return true;
}

/** App-local navigation only; returning to a page never replays a device action. */
export function createStudioNavigation(
  canNavigate: () => boolean = () => true,
) {
  const entries = ref<StudioLocation[]>([
    { view: "device", section: "summary" },
  ]);
  const index = ref(0);
  const current = computed(() => entries.value[index.value]!);
  const enabled = computed(canNavigate);
  const canGoBack = computed(() => enabled.value && index.value > 0);
  const canGoForward = computed(
    () => enabled.value && index.value < entries.value.length - 1,
  );

  function navigate(location: StudioLocation): void {
    if (!enabled.value || sameLocation(current.value, location)) return;
    entries.value = [
      ...entries.value.slice(0, index.value + 1),
      { ...location },
    ];
    index.value = entries.value.length - 1;
  }

  return {
    current,
    enabled,
    canGoBack,
    canGoForward,
    navigate,
    back: () => {
      if (canGoBack.value) index.value -= 1;
    },
    forward: () => {
      if (canGoForward.value) index.value += 1;
    },
  };
}
