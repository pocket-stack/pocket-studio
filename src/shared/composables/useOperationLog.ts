import { computed, readonly, ref } from "vue";

import {
  useGateway,
  type LogEntry,
  type LogLevel,
  type LogSource,
  type Unsubscribe,
} from "../gateway";
import { notify } from "./useNotifications";

const entries = ref<LogEntry[]>([]);
const levelFilter = ref<Set<LogLevel>>(
  new Set(["error", "warn", "info", "debug"]),
);
const sourceFilter = ref<LogSource | "all">("all");
const operationFilter = ref<string | null>(null);
const exporting = ref(false);
const exportText = ref<string | null>(null);
const storageFailed = ref(false);
const query = ref("");
const storageKey = useGateway().capabilities.demo
  ? "pocket-studio.operation-history.v1"
  : "pocket-studio.operation-history.native.v1";
const entryId = (id: string) =>
  useGateway().capabilities.demo ? `${runId}/${id}` : id;
const runId = crypto.randomUUID().slice(0, 8);
function persist(): void {
  try {
    localStorage.setItem(
      storageKey,
      JSON.stringify(
        entries.value.map((entry) => ({
          ...entry,
          operationId:
            entry.operationId && entry.id.startsWith(`${runId}/`)
              ? `${runId}/${entry.operationId}`
              : entry.operationId,
        })),
      ),
    );
    storageFailed.value = false;
  } catch {
    storageFailed.value = true;
  }
}
function loadHistory(): LogEntry[] {
  try {
    const stored: unknown = JSON.parse(
      localStorage.getItem(storageKey) ?? "[]",
    );
    if (!Array.isArray(stored)) return [];
    return stored
      .filter(
        (entry): entry is LogEntry =>
          entry &&
          typeof entry.id === "string" &&
          Number.isFinite(entry.timestamp) &&
          typeof entry.message === "string" &&
          typeof entry.code === "string" &&
          ["error", "warn", "info", "debug"].includes(entry.level) &&
          ["device", "preparation", "store", "system"].includes(entry.source),
      )
      .slice(-MAX_ENTRIES);
  } catch {
    storageFailed.value = true;
    return [];
  }
}
function serializeLogs(items: readonly LogEntry[]): string {
  return items
    .map(
      (entry) =>
        `${new Date(entry.timestamp).toISOString()} ${entry.level.toUpperCase().padEnd(5)} [${entry.source}] [${entry.operationId ?? "-"}] ${entry.message} ${JSON.stringify({ event: entry.id, code: entry.code, ...entry.params })}`,
    )
    .join("\n");
}
let initialized = false;
let stop: Unsubscribe | undefined;

const MAX_ENTRIES = 2000;

async function initialize(): Promise<void> {
  if (initialized) return;
  initialized = true;
  const gateway = useGateway();
  entries.value = loadHistory();
  try {
    stop ??= await gateway.logs.onEntry((entry) => {
      if (entries.value.some((existing) => existing.id === entryId(entry.id)))
        return;
      entries.value.push({ ...entry, id: entryId(entry.id) });
      if (entries.value.length > MAX_ENTRIES)
        entries.value.splice(0, entries.value.length - MAX_ENTRIES);
      persist();
    });
    const existing = await gateway.logs.list();
    const known = new Set(entries.value.map((entry) => entry.id));
    entries.value = [
      ...existing
        .filter((entry) => !known.has(entryId(entry.id)))
        .map((entry) => ({ ...entry, id: entryId(entry.id) })),
      ...entries.value,
    ]
      .sort((a, b) => a.timestamp - b.timestamp)
      .slice(-MAX_ENTRIES);
    persist();
  } catch {
    initialized = false;
    notify("error", "notifications.logLoadFailed");
  }
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
    exportText.value = serializeLogs(entries.value);
    // Native exports remain reviewable
    // and copyable in a dialog; browser previews additionally download the file.
    if (useGateway().flavor === "browser") {
      const url = URL.createObjectURL(
        new Blob([exportText.value], { type: "text/plain;charset=utf-8" }),
      );
      const link = document.createElement("a");
      link.href = url;
      link.download = `pocket-studio-${new Date().toISOString().replace(/[:.]/g, "-")}.log`;
      document.body.append(link);
      link.click();
      link.remove();
      window.setTimeout(() => URL.revokeObjectURL(url), 1000);
    }
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
        (query.value.trim() === "" ||
          `${entry.message} ${entry.code} ${entry.operationId ?? ""} ${JSON.stringify(entry.params ?? {})}`
            .toLowerCase()
            .includes(query.value.trim().toLowerCase())) &&
        levelFilter.value.has(entry.level) &&
        (sourceFilter.value === "all" || entry.source === sourceFilter.value) &&
        (operationFilter.value === null ||
          entry.operationId === operationFilter.value),
    ),
  );

  return {
    entries: readonly(entries),
    query,
    exportText,
    storageFailed: readonly(storageFailed),
    serializeLogs,
    recordUiEvent: (
      code: string,
      message: string,
      params: Record<string, string>,
    ) => {
      entries.value.push({
        id: `${runId}/ui-${crypto.randomUUID()}`,
        timestamp: Date.now(),
        level: "info",
        source: "preparation",
        code,
        message,
        params,
      });
      entries.value = entries.value.slice(-MAX_ENTRIES);
      persist();
    },
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
