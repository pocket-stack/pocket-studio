import { useI18n } from "vue-i18n";
import type { LogEntry } from "../gateway";

/** Display readable labels while preserving the original diagnostic export. */
export function useLogMessage() {
  const { t, te } = useI18n();
  return (entry: LogEntry): string => {
    if (!te(entry.code)) return entry.message;
    const params = { ...entry.params };
    if (params.issue && te(`connection.issues.${params.issue}`))
      params.issue = t(`connection.issues.${params.issue}`);
    if (params.status && te(`readiness.status.${params.status}`))
      params.status = t(`readiness.status.${params.status}`);
    if (params.mode && te(`device.mode.${params.mode}`))
      params.mode = t(`device.mode.${params.mode}`);
    if (params.action === "enterDfu")
      params.action = t("preparation.steps.enterDfu.title");
    if (params.step) {
      const key = te(`preparation.steps.${params.step}.title`)
        ? `preparation.steps.${params.step}.title`
        : `store.steps.${params.step}.title`;
      if (te(key)) params.step = t(key);
    }
    return t(entry.code, params);
  };
}
