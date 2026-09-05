import { computed, readonly, ref } from "vue";

import {
  useGateway,
  type LogEntry,
  type LogLevel,
  type LogSource,
} from "../gateway";
import { notify } from "./useNotifications";

const entries = ref<LogEntry[]>([]);
const levelFilter = ref<Set<LogLevel>>(
  new Set(["error", "warn", "info", "debug"]),
);
const sourceFilter = ref<LogSource | "all">("all");
const operationFilter = ref<string | null>(null);
const exporting = ref(false);
let initialized = false;

const MAX_ENTRIES = 2000;

async function initialize(): Promise<void> {
  if (initialized) return;
  initialized = true;
  const gateway = useGateway();
  gateway.logs.onEntry((entry) => {
    entries.value.push(entry);
    if (entries.value.length > MAX_ENTRIES)
      entries.value.splice(0, entries.value.length - MAX_ENTRIES);
  });
  const existing = await gateway.logs.list();
  const known = new Set(entries.value.map((entry) => entry.id));
  entries.value = [
    ...existing.filter((entry) => !known.has(entry.id)),
    ...entries.value,
  ];
}

function toggleLevel(level: LogLevel): void {
  const next = new Set(levelFilter.value);
  if (next.has(level)) next.delete(level);
  else next.add(level);
  levelFilter.value = next;
}

async function exportLogs(): Promise<void> {
  exporting.value = true;
  try {
    const path = await useGateway().logs.export();
    notify("success", "notifications.logExported", { path });
  } catch {
    notify("error", "notifications.logExportFailed");
  } finally {
    exporting.value = false;
  }
}

export function useOperationLog() {
  const filtered = computed(() =>
    entries.value.filter(
      (entry) =>
        levelFilter.value.has(entry.level) &&
        (sourceFilter.value === "all" || entry.source === sourceFilter.value) &&
        (operationFilter.value === null ||
          entry.operationId === operationFilter.value),
    ),
  );

  return {
    entries: readonly(entries),
    filtered,
    levelFilter: readonly(levelFilter),
    sourceFilter,
    operationFilter,
    exporting: readonly(exporting),
    initialize,
    toggleLevel,
    exportLogs,
  };
}
