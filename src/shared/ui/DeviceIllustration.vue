<script setup lang="ts">
import { computed, useId } from "vue";

import { deviceScreen, deviceViewBox } from "./deviceGeometry";

/**
 * iPod touch (4th generation), front view: steel rim, black glass, front
 * camera, round Home button, sleep button on the top edge, volume rocker on
 * the left edge, 30-pin dock at the bottom. The screen is a slot so scenes can
 * draw anything inside it in screen coordinates (0..181 x 0..272).
 */
const props = withDefaults(
  defineProps<{
    width?: number;
    screen?: "off" | "home" | "apple" | "recovery";
    pressHome?: boolean;
    pressPower?: boolean;
    cable?: boolean;
    shadow?: boolean;
  }>(),
  { width: 180, screen: "off" },
);

const id = useId();
const ids = computed(() => ({
  rim: `${id}-rim`,
  glass: `${id}-glass`,
  glare: `${id}-glare`,
  home: `${id}-home`,
  clip: `${id}-clip`,
  wallpaper: `${id}-wallpaper`,
}));
const height = computed(
  () => (props.width * deviceViewBox.height) / deviceViewBox.width,
);
const s = deviceScreen;

const homeIcons = [
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
  "#7fa14a",
  "#c0664f",
  "#3f8fbf",
  "#9a7bcf",
];
</script>

<template>
  <svg
    :viewBox="`0 0 ${deviceViewBox.width} ${deviceViewBox.height}`"
    :width="width"
    :height="height"
    class="block shrink-0 select-none"
    :class="shadow && 'drop-shadow-[0_10px_18px_rgb(0_0_0/0.18)]'"
    aria-hidden="true"
    focusable="false"
  >
    <defs>
      <linearGradient :id="ids.rim" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#f2f3f5" />
        <stop offset="0.35" stop-color="#c9cbd0" />
        <stop offset="0.7" stop-color="#8e9096" />
        <stop offset="1" stop-color="#b9bbc0" />
      </linearGradient>
      <linearGradient :id="ids.glass" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#232427" />
        <stop offset="1" stop-color="#0e0e10" />
      </linearGradient>
      <linearGradient :id="ids.glare" x1="0" y1="0" x2="1" y2="1">
        <stop offset="0" stop-color="#ffffff" stop-opacity="0.16" />
        <stop offset="0.45" stop-color="#ffffff" stop-opacity="0.03" />
        <stop offset="1" stop-color="#ffffff" stop-opacity="0" />
      </linearGradient>
      <radialGradient :id="ids.home" cx="0.5" cy="0.4" r="0.6">
        <stop offset="0" stop-color="#1b1c1f" />
        <stop offset="0.85" stop-color="#0a0a0b" />
        <stop offset="1" stop-color="#2a2b2f" />
      </radialGradient>
      <linearGradient :id="ids.wallpaper" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#1c2b45" />
        <stop offset="1" stop-color="#0d1524" />
      </linearGradient>
      <clipPath :id="ids.clip">
        <rect :x="s.x" :y="s.y" :width="s.width" :height="s.height" rx="3" />
      </clipPath>
    </defs>

    <!-- cable + 30-pin plug -->
    <g v-if="cable">
      <path
        d="M120 452v28"
        stroke="#b8babf"
        stroke-width="5"
        stroke-linecap="round"
      />
      <rect x="96" y="426" width="48" height="26" rx="5" fill="#d7d9dd" />
      <rect
        x="96"
        y="426"
        width="48"
        height="26"
        rx="5"
        :fill="`url(#${ids.rim})`"
        opacity="0.5"
      />
      <rect x="104" y="433" width="32" height="3" rx="1.5" fill="#8d9096" />
    </g>

    <!-- sleep/wake button (top edge) -->
    <g :data-pressed="pressPower">
      <rect
        x="150"
        y="6"
        width="40"
        height="9"
        rx="3"
        :fill="pressPower ? 'var(--color-signal)' : '#c4c6cb'"
        stroke="#7c7e84"
        stroke-width="0.8"
      />
    </g>

    <!-- volume rocker (left edge) -->
    <rect
      x="6"
      y="110"
      width="8"
      height="24"
      rx="2.5"
      fill="#c4c6cb"
      stroke="#7c7e84"
      stroke-width="0.8"
    />
    <rect
      x="6"
      y="140"
      width="8"
      height="24"
      rx="2.5"
      fill="#c4c6cb"
      stroke="#7c7e84"
      stroke-width="0.8"
    />

    <!-- steel rim -->
    <rect
      x="12"
      y="14"
      width="216"
      height="412"
      rx="30"
      :fill="`url(#${ids.rim})`"
    />
    <!-- black glass face -->
    <rect
      x="15.5"
      y="17.5"
      width="209"
      height="405"
      rx="27"
      :fill="`url(#${ids.glass})`"
    />
    <!-- 30-pin dock opening on the bottom edge -->
    <rect
      v-if="!cable"
      x="84"
      y="423.5"
      width="72"
      height="2.5"
      rx="1.25"
      fill="#5d6067"
    />

    <!-- front camera -->
    <circle cx="120" cy="44" r="4.2" fill="#0a0b10" />
    <circle cx="120" cy="44" r="2.4" fill="#182338" />
    <circle cx="119" cy="43" r="0.9" fill="#8fa8d8" opacity="0.9" />

    <!-- screen -->
    <rect
      :x="s.x"
      :y="s.y"
      :width="s.width"
      :height="s.height"
      rx="3"
      fill="#050507"
    />
    <g :clip-path="`url(#${ids.clip})`">
      <slot name="screen">
        <g v-if="screen === 'home'">
          <rect
            :x="s.x"
            :y="s.y"
            :width="s.width"
            :height="s.height"
            :fill="`url(#${ids.wallpaper})`"
          />
          <rect
            :x="s.x"
            :y="s.y"
            :width="s.width"
            height="11"
            fill="#000"
            opacity="0.55"
          />
          <rect
            :x="s.x + 6"
            :y="s.y + 4"
            width="10"
            height="3"
            rx="0.8"
            fill="#dfe6f2"
          />
          <rect
            :x="s.x + s.width - 18"
            :y="s.y + 3.5"
            width="12"
            height="4"
            rx="1"
            fill="#7fd08a"
          />
          <g v-for="(color, index) in homeIcons" :key="index">
            <rect
              :x="s.x + 12 + (index % 4) * 41"
              :y="s.y + 24 + Math.floor(index / 4) * 44"
              width="30"
              height="30"
              rx="7"
              :fill="color"
            />
            <rect
              :x="s.x + 12 + (index % 4) * 41"
              :y="s.y + 24 + Math.floor(index / 4) * 44"
              width="30"
              height="14"
              rx="7"
              fill="#fff"
              opacity="0.16"
            />
          </g>
          <rect
            :x="s.x"
            :y="s.y + s.height - 46"
            :width="s.width"
            height="46"
            fill="#ffffff"
            opacity="0.1"
          />
          <rect
            v-for="dock in 4"
            :key="dock"
            :x="s.x + 12 + (dock - 1) * 41"
            :y="s.y + s.height - 38"
            width="30"
            height="30"
            rx="7"
            :fill="homeIcons[(dock + 3) % homeIcons.length]"
          />
        </g>
        <g v-else-if="screen === 'apple'">
          <path
            :transform="`translate(${s.x + s.width / 2 - 28} ${s.y + s.height / 2 - 34}) scale(1.15)`"
            fill="#e9e9ec"
            d="M39.2 24.1c0-5.4 4.4-8 4.6-8.1-2.5-3.7-6.4-4.2-7.8-4.2-3.3-.3-6.5 2-8.2 2-1.7 0-4.3-1.9-7.1-1.9-3.6.1-7 2.1-8.8 5.4-3.8 6.6-1 16.3 2.7 21.7 1.8 2.6 3.9 5.6 6.7 5.5 2.7-.1 3.7-1.7 7-1.7s4.2 1.7 7 1.7c2.9-.1 4.8-2.7 6.5-5.3 2.1-3 2.9-6 3-6.1-.1 0-5.6-2.2-5.6-9zM33.9 8.3c1.5-1.8 2.5-4.3 2.2-6.8-2.1.1-4.7 1.4-6.2 3.2-1.4 1.6-2.6 4.1-2.3 6.6 2.4.2 4.8-1.2 6.3-3z"
          />
        </g>
        <g
          v-else-if="screen === 'recovery'"
          fill="none"
          stroke="#e9e9ec"
          stroke-width="3"
          stroke-linecap="round"
        >
          <path :d="`M${s.x + s.width / 2} ${s.y + s.height / 2 + 40}v34`" />
          <rect
            :x="s.x + s.width / 2 - 14"
            :y="s.y + s.height / 2 + 12"
            width="28"
            height="30"
            rx="4"
          />
          <path
            :d="`M${s.x + s.width / 2} ${s.y + s.height / 2 - 16}v-26m-10 10 10-10 10 10`"
          />
          <circle
            :cx="s.x + s.width / 2"
            :cy="s.y + s.height / 2 - 62"
            r="24"
          />
        </g>
      </slot>
    </g>

    <!-- glass glare over the face, above the screen -->
    <rect
      x="15.5"
      y="17.5"
      width="209"
      height="405"
      rx="27"
      :fill="`url(#${ids.glare})`"
    />

    <!-- home button -->
    <g :data-pressed="pressHome">
      <circle cx="120" cy="388" r="20" :fill="`url(#${ids.home})`" />
      <circle
        cx="120"
        cy="388"
        r="20"
        fill="none"
        stroke="#3b3c41"
        stroke-width="1"
      />
      <circle
        cx="120"
        cy="388"
        r="19"
        fill="none"
        stroke="#ffffff"
        stroke-opacity="0.08"
        stroke-width="1"
      />
      <rect
        x="112.5"
        y="380.5"
        width="15"
        height="15"
        rx="3.2"
        fill="none"
        :stroke="pressHome ? 'var(--color-signal)' : '#8b8d93'"
        stroke-width="1.6"
      />
      <circle
        v-if="pressHome"
        cx="120"
        cy="388"
        r="20"
        fill="none"
        stroke="var(--color-signal)"
        stroke-width="2.5"
      />
    </g>
  </svg>
</template>
