import { readonly, ref } from "vue";

export type NotificationTone = "info" | "success" | "warning" | "error";

export interface Notification {
  id: number;
  tone: NotificationTone;
  /** i18n key; params are interpolated by the renderer. */
  key: string;
  params?: Record<string, string>;
}

const items = ref<Notification[]>([]);
let sequence = 0;

export function notify(
  tone: NotificationTone,
  key: string,
  params?: Record<string, string>,
): void {
  sequence += 1;
  const id = sequence;
  items.value.push({ id, tone, key, params });
  window.setTimeout(() => dismiss(id), tone === "error" ? 8000 : 4500);
}

export function dismiss(id: number): void {
  items.value = items.value.filter((item) => item.id !== id);
}

export function useNotifications() {
  return { items: readonly(items), dismiss };
}
