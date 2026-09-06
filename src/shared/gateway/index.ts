import { isTauri } from "@tauri-apps/api/core";
import { createTauriGateway } from "./tauriGateway";
import { createSimulatedGateway } from "./simulatedGateway";
import type { StudioGateway } from "./types";
export * from "./types";
let instance: StudioGateway | undefined;
/** Physical I/O stays in Rust; the browser retains the interaction demo. */
export function useGateway(): StudioGateway {
  instance ??= isTauri() ? createTauriGateway() : createSimulatedGateway();
  return instance;
}
