import { ref } from "vue";
import type { StoreTarget } from "../../shared/gateway";

/**
 * Manual device connection and 3DS card preparation share one dialog.
 * "connect" pairs a console that discovery cannot see; "launcher" copies or
 * upgrades the Pocket launcher; "card" copies a standalone title for FBI or
 * the Homebrew Launcher, which is how a console without the launcher gets apps.
 */
export type ConnectMode = "connect" | "launcher" | "card";
export interface CardTarget {
  appId: string;
  name: string;
  /** Bundled formats the catalog publishes for this title. */
  formats: string[];
}
const open = ref(false);
const mode = ref<ConnectMode>("connect");
const requirement = ref<StoreTarget["runtime_requirement"]>(null);
const hostAbi = ref<number | null>(null);
const target = ref<CardTarget | null>(null);
export function useDeviceConnect() {
  return {
    open,
    mode,
    requirement,
    hostAbi,
    target,
    show: (
      kind: ConnectMode,
      options: {
        runtime?: StoreTarget["runtime_requirement"];
        abi?: number | null;
        target?: CardTarget;
      } = {},
    ) => {
      mode.value = kind;
      requirement.value = options.runtime ?? null;
      hostAbi.value = options.abi ?? null;
      target.value = options.target ?? null;
      open.value = true;
    },
    close: () => {
      open.value = false;
    },
  };
}
