<script setup lang="ts">
/**
 * Stylised iPod touch (4th generation): power button on the top edge,
 * Home button under the screen. Used for the empty state and DFU guide.
 */
defineProps<{
  pressHome?: boolean;
  pressPower?: boolean;
  screen?: "off" | "home" | "apple" | "recovery";
  cable?: boolean;
  width?: number;
}>();
</script>

<template>
  <svg
    :width="width ?? 180"
    viewBox="0 0 180 340"
    fill="none"
    aria-hidden="true"
    class="select-none"
  >
    <!-- cable -->
    <g v-if="cable" class="text-muted">
      <rect
        x="82"
        y="318"
        width="16"
        height="10"
        rx="2"
        fill="currentColor"
        opacity="0.6"
      />
      <path
        d="M90 328v12"
        stroke="currentColor"
        stroke-width="4"
        stroke-linecap="round"
        opacity="0.6"
      />
    </g>
    <!-- power button (top edge) -->
    <g :class="pressPower ? 'text-signal' : 'text-muted'">
      <rect x="124" y="8" width="30" height="7" rx="3.5" fill="currentColor" />
      <g v-if="pressPower" class="pulse">
        <circle
          cx="139"
          cy="11"
          r="18"
          stroke="currentColor"
          stroke-width="2"
          opacity="0.5"
        />
        <circle
          cx="139"
          cy="11"
          r="27"
          stroke="currentColor"
          stroke-width="1.5"
          opacity="0.25"
        />
      </g>
    </g>
    <!-- body -->
    <rect
      x="18"
      y="14"
      width="144"
      height="306"
      rx="22"
      class="fill-ink/85 dark:fill-raised"
    />
    <rect x="20" y="16" width="140" height="302" rx="20" fill="#1a1c1a" />
    <!-- camera + speaker -->
    <circle cx="90" cy="30" r="2.5" fill="#333" />
    <!-- screen -->
    <rect x="30" y="42" width="120" height="216" rx="3" fill="#050505" />
    <g v-if="screen === 'home'">
      <rect x="30" y="42" width="120" height="216" rx="3" fill="#0c2a44" />
      <g v-for="row in 4" :key="row">
        <rect
          v-for="col in 4"
          :key="col"
          :x="38 + (col - 1) * 27"
          :y="52 + (row - 1) * 30"
          width="20"
          height="20"
          rx="4"
          :fill="['#d96f1f', '#3b8f5a', '#2a6f9e', '#9b59b6'][(row + col) % 4]"
          opacity="0.9"
        />
      </g>
      <rect
        x="34"
        y="230"
        width="112"
        height="24"
        rx="6"
        fill="#ffffff"
        opacity="0.12"
      />
    </g>
    <g v-else-if="screen === 'apple'">
      <path
        d="M97 120c-3 0-5 2-8 2s-6-2-9-2c-7 0-13 7-13 17 0 11 8 25 13 25 3 0 5-2 9-2s6 2 9 2c5 0 13-14 13-25 0-10-7-17-14-17Zm-6-4c3-1 6-4 6-8-3 0-6 2-7 5-1 1-1 2 1 3Z"
        fill="#e8e8e8"
      />
    </g>
    <g v-else-if="screen === 'recovery'">
      <rect
        x="66"
        y="112"
        width="48"
        height="26"
        rx="4"
        stroke="#e8e8e8"
        stroke-width="3"
      />
      <path
        d="M90 138v20M76 172h28"
        stroke="#e8e8e8"
        stroke-width="3"
        stroke-linecap="round"
      />
      <circle cx="90" cy="176" r="10" stroke="#e8e8e8" stroke-width="3" />
    </g>
    <!-- home button -->
    <g :class="pressHome ? 'text-signal' : 'text-muted'">
      <circle
        cx="90"
        cy="288"
        r="14"
        fill="#101210"
        stroke="currentColor"
        stroke-width="2"
      />
      <rect
        x="83"
        y="281"
        width="14"
        height="14"
        rx="3"
        stroke="currentColor"
        stroke-width="1.5"
        opacity="0.7"
      />
      <g v-if="pressHome" class="pulse">
        <circle
          cx="90"
          cy="288"
          r="22"
          stroke="currentColor"
          stroke-width="2"
          opacity="0.5"
        />
        <circle
          cx="90"
          cy="288"
          r="31"
          stroke="currentColor"
          stroke-width="1.5"
          opacity="0.25"
        />
      </g>
    </g>
  </svg>
</template>
