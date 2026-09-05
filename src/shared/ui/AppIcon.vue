<script setup lang="ts">
const paths: Record<string, string> = {
  device:
    "M7 2h10a2 2 0 0 1 2 2v16a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2Zm0 2v13h10V4H7Zm5 17a1.2 1.2 0 1 0 0-2.4A1.2 1.2 0 0 0 12 21Z",
  store:
    "M4 4h16l1 5a3 3 0 0 1-3 3 3 3 0 0 1-3-3 3 3 0 0 1-6 0 3 3 0 0 1-3 3 3 3 0 0 1-3-3l1-5Zm1 9.8A5 5 0 0 0 7 14a5 5 0 0 0 2.5-.7A5 5 0 0 0 12 14a5 5 0 0 0 2.5-.7A5 5 0 0 0 17 14a5 5 0 0 0 2-.2V20H5v-6.2ZM9 15v5h6v-5H9Z",
  logs: "M5 3h14v18H5V3Zm2 2v14h10V5H7Zm2 3h6v2H9V8Zm0 4h6v2H9v-2Zm0 4h4v2H9v-2Z",
  settings:
    "M12 8a4 4 0 1 1 0 8 4 4 0 0 1 0-8Zm0 2a2 2 0 1 0 0 4 2 2 0 0 0 0-4Zm-1.5-8h3l.5 2.3 1.6.7 2-1.4 2.1 2.1-1.4 2 .7 1.6 2.3.5v3l-2.3.5-.7 1.6 1.4 2-2.1 2.1-2-1.4-1.6.7-.5 2.3h-3l-.5-2.3-1.6-.7-2 1.4-2.1-2.1 1.4-2-.7-1.6L2 13.5v-3l2.3-.5.7-1.6-1.4-2 2.1-2.1 2 1.4 1.6-.7.5-2.3Z",
  check: "M9.5 16.2 5.3 12l-1.4 1.4 5.6 5.6 12-12-1.4-1.4z",
  cross:
    "M18.3 5.7 12 12l6.3 6.3-1.4 1.4L10.6 13.4 12 12 5.7 5.7l1.4-1.4L12 10.6l6.3-6.3z",
  warning:
    "M12 2 1 21h22L12 2Zm0 4 7.5 13h-15L12 6Zm-1 4v5h2v-5h-2Zm0 6v2h2v-2h-2Z",
  info: "M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20Zm0 2a8 8 0 1 0 0 16 8 8 0 0 0 0-16Zm-1 6h2v7h-2v-7Zm0-3h2v2h-2V7Z",
  usb: "M11 2h2v6.3l3-1.8V5h3v3h-2.3L13 11.2V14l3 1.8V14h3v3h-2.3L13 19.8V22h-2v-2.2L7.3 17H5v-3h3v1.8l3-1.8v-2.8L7.3 8H5V5h3v1.5l3 1.8V2Z",
  arrowLeft: "M20 11H7.8l5.6-5.6L12 4l-8 8 8 8 1.4-1.4L7.8 13H20v-2Z",
  arrowRight: "M4 11h12.2l-5.6-5.6L12 4l8 8-8 8-1.4-1.4 5.6-5.6H4v-2Z",
  refresh:
    "M12 4a8 8 0 0 1 7.4 5h-2.2A6 6 0 0 0 6 12H4a8 8 0 0 1 8-8Zm0 16a8 8 0 0 1-7.4-5h2.2A6 6 0 0 0 18 12h2a8 8 0 0 1-8 8Zm8-16v5h-5l5-5ZM4 20v-5h5l-5 5Z",
  download:
    "M11 3h2v9.2l3.3-3.3 1.4 1.4L12 16l-5.7-5.7 1.4-1.4L11 12.2V3ZM4 18h16v2H4v-2Z",
  shield:
    "M12 2 4 5v6c0 5 3.4 9.5 8 11 4.6-1.5 8-6 8-11V5l-8-3Zm0 2.2 6 2.2V11c0 3.9-2.5 7.5-6 8.8-3.5-1.3-6-4.9-6-8.8V6.4l6-2.2Z",
  bolt: "M13 2 4 14h6l-1 8 9-12h-6l1-8Z",
  play: "M8 5v14l11-7L8 5Z",
  stop: "M6 6h12v12H6z",
  clock:
    "M12 2a10 10 0 1 1 0 20 10 10 0 0 1 0-20Zm0 2a8 8 0 1 0 0 16 8 8 0 0 0 0-16Zm-1 3h2v5.6l3.5 2-1 1.7L11 13V7Z",
  lab: "M9 2h6v2h-1v5.5l5.6 9.4A2 2 0 0 1 17.9 22H6.1a2 2 0 0 1-1.7-3.1L10 9.5V4H9V2Zm3 2v6.1L7.9 17h8.2L12 10.1V4Z",
  copy: "M8 3h11v13h-2V5H8V3ZM5 7h11v14H5V7Zm2 2v10h7V9H7Z",
  cable: "M4 11h3V8h2v8H7v-3H4v-2Zm13 0h3v2h-3v3h-2V8h2v3Zm-8 0h6v2H9v-2Z",
  battery: "M3 7h15v3h2v4h-2v3H3V7Zm2 2v6h11V9H5Zm1 1h5v4H6v-4Z",
  external:
    "M14 3h7v7h-2V6.4l-8.3 8.3-1.4-1.4L17.6 5H14V3ZM5 5h6v2H7v10h10v-4h2v6H5V5Z",
};

defineProps<{ name: keyof typeof paths | string; size?: number }>();
</script>

<template>
  <svg
    :width="size ?? 18"
    :height="size ?? 18"
    viewBox="0 0 24 24"
    fill="currentColor"
    aria-hidden="true"
    class="shrink-0"
  >
    <path :d="paths[name] ?? ''" />
  </svg>
</template>
