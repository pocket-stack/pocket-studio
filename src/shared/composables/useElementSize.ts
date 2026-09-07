import { onBeforeUnmount, onMounted, ref, watch, type Ref } from "vue";

/** Live content box size of an element; zero until mounted or in non-DOM hosts. */
export function useElementSize(target: Ref<HTMLElement | null>) {
  const width = ref(0);
  const height = ref(0);
  let observer: ResizeObserver | undefined;

  function measure(): void {
    const element = target.value;
    if (!element) return;
    width.value = element.clientWidth;
    height.value = element.clientHeight;
  }

  onMounted(() => {
    measure();
    if (typeof ResizeObserver === "undefined") return;
    observer = new ResizeObserver(measure);
    if (target.value) observer.observe(target.value);
  });
  watch(target, (element, previous) => {
    if (previous) observer?.unobserve(previous);
    if (element) observer?.observe(element);
    measure();
  });
  onBeforeUnmount(() => observer?.disconnect());

  return { width, height };
}
