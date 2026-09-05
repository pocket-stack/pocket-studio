import { createSimulatedGateway } from "./simulatedGateway";
import { createTauriGateway } from "./tauriGateway";
import type { StudioGateway } from "./types";

export * from "./types";

let instance: StudioGateway | undefined;

function runningInsideTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

/** Single gateway for the whole webview; components never call `invoke` directly. */
export function useGateway(): StudioGateway {
  instance ??= runningInsideTauri()
    ? createTauriGateway()
    : createSimulatedGateway();
  return instance;
}
