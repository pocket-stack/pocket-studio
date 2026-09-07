import {
  computed,
  onBeforeUnmount,
  onMounted,
  ref,
  type MaybeRefOrGetter,
  toValue,
} from "vue";

/**
 * Minimum elapsed-time gate for forced reading. Background time counts, and
 * focus or visibility changes re-sync from wall-clock time so a throttled
 * webview cannot under-count.
 */
export function useReadingTimer(minimumSeconds: MaybeRefOrGetter<number>) {
  const elapsed = ref(0);
  let timer: number | undefined;
  let startedAt = 0;

  const minimum = computed(() => toValue(minimumSeconds));
  const remaining = computed(() => Math.max(0, minimum.value - elapsed.value));
  const satisfied = computed(() => remaining.value === 0);

  function tick(): void {
    elapsed.value = Math.min(
      minimum.value,
      Math.floor((performance.now() - startedAt) / 1000),
    );
    if (satisfied.value && timer !== undefined) {
      window.clearInterval(timer);
      timer = undefined;
    }
  }

  onMounted(() => {
    if (minimum.value <= 0) return;
    startedAt = performance.now();
    timer = window.setInterval(tick, 100);
    window.addEventListener("focus", tick);
    document.addEventListener("visibilitychange", tick);
  });
  onBeforeUnmount(() => {
    if (timer !== undefined) window.clearInterval(timer);
    window.removeEventListener("focus", tick);
    document.removeEventListener("visibilitychange", tick);
  });

  return { elapsed, remaining, satisfied };
}
