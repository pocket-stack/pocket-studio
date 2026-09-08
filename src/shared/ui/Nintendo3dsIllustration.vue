<script setup lang="ts">
import { computed, useId } from "vue";

/**
 * New Nintendo 3DS LL, opened, front view: metallic blue shell, glossy black
 * lid face with the wide 3D screen, hinge, dark lower face with the touch
 * screen, Circle Pad, D-pad, C-Stick, ABXY, START/SELECT and HOME.
 */
const props = withDefaults(
  defineProps<{
    width?: number;
    screen?: "off" | "home";
    shadow?: boolean;
  }>(),
  { width: 180, screen: "off" },
);

const viewBox = { width: 320, height: 380 } as const;
const id = useId();
const ids = computed(() => ({
  shell: `${id}-shell`,
  face: `${id}-face`,
  lower: `${id}-lower`,
  hinge: `${id}-hinge`,
  glare: `${id}-glare`,
  pad: `${id}-pad`,
  wallpaper: `${id}-wallpaper`,
}));
const height = computed(() => (props.width * viewBox.height) / viewBox.width);
const top = { x: 66, y: 34, width: 188, height: 106 } as const;
const bottom = { x: 75, y: 214, width: 170, height: 128 } as const;
const tiles = [
  "#4a8fd6",
  "#d9a441",
  "#5aa66b",
  "#c95c4f",
  "#8a6fc9",
  "#4fa7b8",
  "#d97a3d",
  "#6b7fd6",
  "#b04e8c",
  "#4c9f8f",
  "#c7a23a",
  "#5a6fb0",
];
</script>

<template>
  <svg
    :viewBox="`0 0 ${viewBox.width} ${viewBox.height}`"
    :width="width"
    :height="height"
    class="block shrink-0 select-none"
    :class="shadow && 'drop-shadow-[0_10px_18px_rgb(0_0_0/0.18)]'"
    aria-hidden="true"
    focusable="false"
  >
    <defs>
      <linearGradient :id="ids.shell" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#5d86d0" />
        <stop offset="0.5" stop-color="#3a5fae" />
        <stop offset="1" stop-color="#27407c" />
      </linearGradient>
      <linearGradient :id="ids.face" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#26272b" />
        <stop offset="1" stop-color="#0f1013" />
      </linearGradient>
      <linearGradient :id="ids.lower" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#34363c" />
        <stop offset="1" stop-color="#232429" />
      </linearGradient>
      <linearGradient :id="ids.hinge" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#6f95d8" />
        <stop offset="0.5" stop-color="#2c4a8c" />
        <stop offset="1" stop-color="#1c2f5e" />
      </linearGradient>
      <linearGradient :id="ids.glare" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#ffffff" stop-opacity="0.18" />
        <stop offset="0.5" stop-color="#ffffff" stop-opacity="0.03" />
        <stop offset="1" stop-color="#ffffff" stop-opacity="0" />
      </linearGradient>
      <radialGradient :id="ids.pad" cx="0.4" cy="0.35" r="0.7">
        <stop offset="0" stop-color="#5a5d66" />
        <stop offset="1" stop-color="#2a2c32" />
      </radialGradient>
      <linearGradient :id="ids.wallpaper" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#1e3a6b" />
        <stop offset="1" stop-color="#0c1730" />
      </linearGradient>
    </defs>

    <!-- lid -->
    <rect
      x="12"
      y="8"
      width="296"
      height="170"
      rx="16"
      :fill="`url(#${ids.shell})`"
    />
    <rect
      x="22"
      y="16"
      width="276"
      height="156"
      rx="11"
      :fill="`url(#${ids.face})`"
    />
    <!-- inner camera and IR sensor -->
    <circle cx="160" cy="25" r="3" fill="#0a0b10" />
    <circle cx="160" cy="25" r="1.6" fill="#182338" />
    <circle cx="176" cy="25" r="1.8" fill="#3a2c2c" />
    <!-- speaker slits -->
    <g fill="#3a3c44">
      <rect
        v-for="row in 5"
        :key="`l${row}`"
        x="40"
        :y="60 + row * 10"
        width="9"
        height="2"
        rx="1"
      />
      <rect
        v-for="row in 5"
        :key="`r${row}`"
        x="271"
        :y="60 + row * 10"
        width="9"
        height="2"
        rx="1"
      />
    </g>
    <!-- top (3D) screen -->
    <rect
      :x="top.x"
      :y="top.y"
      :width="top.width"
      :height="top.height"
      rx="2"
      fill="#050507"
    />
    <g v-if="screen === 'home'">
      <rect
        :x="top.x"
        :y="top.y"
        :width="top.width"
        :height="top.height"
        :fill="`url(#${ids.wallpaper})`"
      />
      <circle
        :cx="top.x + 52"
        :cy="top.y + 60"
        r="34"
        fill="#ffffff"
        opacity="0.06"
      />
      <circle
        :cx="top.x + 140"
        :cy="top.y + 40"
        r="52"
        fill="#ffffff"
        opacity="0.05"
      />
      <rect
        :x="top.x + 62"
        :y="top.y + 30"
        width="64"
        height="40"
        rx="6"
        fill="#4a8fd6"
      />
      <rect
        :x="top.x + 62"
        :y="top.y + 30"
        width="64"
        height="18"
        rx="6"
        fill="#ffffff"
        opacity="0.18"
      />
      <rect
        :x="top.x + 70"
        :y="top.y + 78"
        width="48"
        height="4"
        rx="2"
        fill="#dfe6f2"
        opacity="0.85"
      />
      <rect
        :x="top.x + 80"
        :y="top.y + 86"
        width="28"
        height="3"
        rx="1.5"
        fill="#dfe6f2"
        opacity="0.5"
      />
    </g>
    <rect
      x="22"
      y="16"
      width="276"
      height="156"
      rx="11"
      :fill="`url(#${ids.glare})`"
    />

    <!-- hinge -->
    <rect
      x="30"
      y="174"
      width="260"
      height="20"
      rx="10"
      :fill="`url(#${ids.hinge})`"
    />
    <rect x="118" y="174" width="84" height="20" fill="#1c2f5e" opacity="0.5" />

    <!-- lower half -->
    <rect
      x="12"
      y="190"
      width="296"
      height="182"
      rx="16"
      :fill="`url(#${ids.shell})`"
    />
    <rect
      x="20"
      y="196"
      width="280"
      height="170"
      rx="12"
      :fill="`url(#${ids.lower})`"
    />
    <!-- touch screen -->
    <rect
      :x="bottom.x - 4"
      :y="bottom.y - 4"
      :width="bottom.width + 8"
      :height="bottom.height + 8"
      rx="4"
      fill="#101114"
    />
    <rect
      :x="bottom.x"
      :y="bottom.y"
      :width="bottom.width"
      :height="bottom.height"
      rx="1.5"
      fill="#050507"
    />
    <g v-if="screen === 'home'">
      <rect
        :x="bottom.x"
        :y="bottom.y"
        :width="bottom.width"
        :height="bottom.height"
        fill="#e9edf3"
      />
      <rect
        :x="bottom.x"
        :y="bottom.y"
        :width="bottom.width"
        height="18"
        fill="#cfd6e1"
      />
      <rect
        :x="bottom.x + 8"
        :y="bottom.y + 6"
        width="26"
        height="6"
        rx="3"
        fill="#8b96a8"
      />
      <rect
        :x="bottom.x + bottom.width - 30"
        :y="bottom.y + 6"
        width="22"
        height="6"
        rx="3"
        fill="#6fb07f"
      />
      <g v-for="(color, index) in tiles" :key="index">
        <rect
          :x="bottom.x + 10 + (index % 6) * 26"
          :y="bottom.y + 30 + Math.floor(index / 6) * 28"
          width="20"
          height="20"
          rx="3"
          :fill="color"
        />
        <rect
          :x="bottom.x + 10 + (index % 6) * 26"
          :y="bottom.y + 30 + Math.floor(index / 6) * 28"
          width="20"
          height="9"
          rx="3"
          fill="#fff"
          opacity="0.2"
        />
      </g>
      <rect
        :x="bottom.x"
        :y="bottom.y + bottom.height - 24"
        :width="bottom.width"
        height="24"
        fill="#cfd6e1"
      />
      <rect
        v-for="dock in 3"
        :key="dock"
        :x="bottom.x + 36 + (dock - 1) * 36"
        :y="bottom.y + bottom.height - 19"
        width="26"
        height="14"
        rx="3"
        fill="#ffffff"
      />
    </g>

    <!-- Circle Pad -->
    <circle cx="42" cy="236" r="19" fill="#17181c" />
    <circle cx="42" cy="236" r="15" :fill="`url(#${ids.pad})`" />
    <circle cx="42" cy="236" r="11" fill="#3b3e46" />
    <!-- D-pad -->
    <g fill="#3a3d45" stroke="#1a1b1f" stroke-width="1">
      <rect x="35" y="278" width="14" height="40" rx="3" />
      <rect x="22" y="291" width="40" height="14" rx="3" />
    </g>
    <!-- C-Stick -->
    <circle cx="278" cy="206" r="7" fill="#17181c" />
    <circle cx="278" cy="206" r="4.5" fill="#4a4d56" />
    <!-- ABXY -->
    <g stroke="#1a1b1f" stroke-width="1">
      <circle cx="278" cy="224" r="7" fill="#3a3d45" />
      <circle cx="262" cy="240" r="7" fill="#3a3d45" />
      <circle cx="294" cy="240" r="7" fill="#3a3d45" />
      <circle cx="278" cy="256" r="7" fill="#3a3d45" />
    </g>
    <g fill="#9ba0ab" font-size="6" font-weight="700" text-anchor="middle">
      <text x="278" y="226">X</text>
      <text x="262" y="242">Y</text>
      <text x="294" y="242">A</text>
      <text x="278" y="258">B</text>
    </g>
    <!-- START / SELECT -->
    <g fill="#3a3d45" stroke="#1a1b1f" stroke-width="0.8">
      <rect x="268" y="290" width="20" height="6" rx="3" />
      <rect x="268" y="304" width="20" height="6" rx="3" />
    </g>
    <!-- HOME -->
    <rect
      x="147"
      y="351"
      width="26"
      height="9"
      rx="4.5"
      fill="#3a3d45"
      stroke="#1a1b1f"
      stroke-width="0.8"
    />
    <path d="M157 358v-2.5l3-2.5 3 2.5V358h-2v-1.6h-2V358z" fill="#c9ccd3" />
    <!-- microphone and power LED -->
    <circle cx="120" cy="356" r="1.2" fill="#15161a" />
    <circle cx="288" cy="356" r="1.6" fill="#4fb7ff" opacity="0.9" />
  </svg>
</template>
