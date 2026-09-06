import { useI18n } from "vue-i18n";
import type { LogEntry } from "../gateway";

/** Display readable labels while preserving the original diagnostic export. */
export function useLogMessage() {
  const { t, te } = useI18n();
  return (entry: LogEntry): string => {
    if (!te(entry.code)) return entry.message;
    const params = { ...entry.params };
    if (params.stage && te(`preparation.usbStages.${params.stage}`))
      params.stage = t(`preparation.usbStages.${params.stage}`);
    if (params.reason && te(`preparation.usbReasons.${params.reason}`))
      params.reason = t(`preparation.usbReasons.${params.reason}`);
    if (params.issue && te(`connection.issues.${params.issue}`))
      params.issue = t(`connection.issues.${params.issue}`);
    if (params.status && te(`readiness.status.${params.status}`))
      params.status = t(`readiness.status.${params.status}`);
    if (params.mode && te(`device.mode.${params.mode}`))
      params.mode = t(`device.mode.${params.mode}`);
    if (params.action === "enterDfu")
      params.action = t("preparation.steps.enterDfu.title");
    if (params.step) {
      params.step = params.step.charAt(0).toLowerCase() + params.step.slice(1);
      const key = te(`preparation.steps.${params.step}.title`)
        ? `preparation.steps.${params.step}.title`
        : `store.steps.${params.step}.title`;
      if (te(key)) params.step = t(key);
    }
    return t(entry.code, params);
  };
}
