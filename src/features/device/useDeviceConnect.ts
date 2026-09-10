import { ref } from "vue";
import type { StoreTarget } from "../../shared/gateway";

/**
 * Manual device connection and 3DS launcher preparation share one dialog.
 * "connect" pairs a console that discovery cannot see; "launcher" copies or
 * upgrades the Pocket launcher for a console that is already known.
 */
export type ConnectMode = "connect" | "launcher";
const open = ref(false);
const mode = ref<ConnectMode>("connect");
const requirement = ref<StoreTarget["runtime_requirement"]>(null);
const hostAbi = ref<number | null>(null);
export function useDeviceConnect() {
  return {
    open,
    mode,
    requirement,
    hostAbi,
    show: (
      kind: ConnectMode,
      runtime: StoreTarget["runtime_requirement"] = null,
      abi: number | null = null,
    ) => {
      mode.value = kind;
      requirement.value = runtime;
      hostAbi.value = abi;
      open.value = true;
    },
    close: () => {
      open.value = false;
    },
  };
}
