import { createSimulatedGateway } from "./simulatedGateway";
import type { StudioGateway } from "./types";
export * from "./types";
let instance: StudioGateway | undefined;
/** This UI prototype always uses fixtures, including in the desktop webview. */
export function useGateway(): StudioGateway {
  instance ??= createSimulatedGateway();
  return instance;
}
