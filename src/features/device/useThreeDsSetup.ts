import { ref } from "vue";
import type { StoreTarget } from "../../shared/gateway";
const open = ref(false);
const requirement = ref<StoreTarget["runtime_requirement"]>(null);
const hostAbi = ref<number | null>(null);
export function useThreeDsSetup() {
  return {
    open,
    requirement,
    hostAbi,
    show: (
      runtime: StoreTarget["runtime_requirement"] = null,
      abi: number | null = null,
    ) => {
      requirement.value = runtime;
      hostAbi.value = abi;
      open.value = true;
    },
    close: () => {
      open.value = false;
    },
  };
}
