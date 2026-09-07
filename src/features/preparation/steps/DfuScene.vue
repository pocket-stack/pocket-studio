<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";

import DeviceIllustration from "../../../shared/ui/DeviceIllustration.vue";
import {
  anchorPercent,
  deviceScreen,
  deviceViewBox,
} from "../../../shared/ui/deviceGeometry";

export type DfuPhase =
  "idle" | "holdBoth" | "holdHome" | "detecting" | "timeout";

/**
 * Animated DFU guidance around the device drawing. Fingers press and lift on
 * the real button positions, countdown rings sit on the buttons themselves,
 * and the cable pulses while the app waits for the device to reappear.
 */
const props = withDefaults(
  defineProps<{
    phase: DfuPhase;
    secondsLeft: number;
    phaseTotal: number;
    width?: number;
  }>(),
  { width: 170 },
);
const { t } = useI18n();

const pressPower = computed(() => props.phase === "holdBoth");
const pressHome = computed(
  () => props.phase === "holdBoth" || props.phase === "holdHome",
);
const counting = computed(
  () => props.phase === "holdBoth" || props.phase === "holdHome",
);
const fraction = computed(() =>
  props.phaseTotal > 0 ? 1 - props.secondsLeft / props.phaseTotal : 0,
);
const height = computed(
  () => (props.width * deviceViewBox.height) / deviceViewBox.width,
);
const power = anchorPercent("power");
const home = anchorPercent("home");
const dock = anchorPercent("dock");
const screenBox = {
  left: `${((deviceScreen.x + deviceScreen.width / 2) / deviceViewBox.width) * 100}%`,
  top: `${((deviceScreen.y + deviceScreen.height / 2) / deviceViewBox.height) * 100}%`,
};
const RING = 2 * Math.PI * 22;
</script>

<template>
  <div
    class="relative select-none"
    :style="{ width: `${width}px`, height: `${height}px` }"
    aria-hidden="true"
  >
    <DeviceIllustration
      :width="width"
      screen="off"
      :press-power="pressPower"
      :press-home="pressHome"
      cable
      shadow
    >
      <template v-if="phase === 'idle'" #screen>
        <!-- iOS 6 "slide to power off" loop: slide, then the screen goes dark -->
        <g>
          <rect
            :x="deviceScreen.x"
            :y="deviceScreen.y"
            :width="deviceScreen.width"
            :height="deviceScreen.height"
            fill="#0b1220"
          />
          <rect
            :x="deviceScreen.x + 12"
            :y="deviceScreen.y + 26"
            :width="deviceScreen.width - 24"
            height="34"
            rx="8"
            fill="#c0392b"
          />
          <rect
            :x="deviceScreen.x + 14"
            :y="deviceScreen.y + 28"
            width="44"
            height="30"
            rx="7"
            fill="#f4f4f5"
            class="motion-safe:animate-dfu-slide"
          />
          <rect
            :x="deviceScreen.x"
            :y="deviceScreen.y"
            :width="deviceScreen.width"
            :height="deviceScreen.height"
            fill="#050507"
            class="motion-safe:animate-dfu-dim"
          />
        </g>
      </template>
    </DeviceIllustration>

    <!-- countdown on the screen -->
    <div
      v-if="counting"
      class="absolute -translate-x-1/2 -translate-y-1/2 text-center font-mono text-[34px] leading-none font-semibold text-white tabular-nums"
      :style="screenBox"
    >
      {{ secondsLeft }}
    </div>
    <div
      v-else-if="phase === 'detecting'"
      class="absolute -translate-x-1/2 -translate-y-1/2 text-center text-[10px] leading-[14px] text-white/60"
      :style="screenBox"
    >
      {{ t("preparation.dfu.blackScreenHint") }}
    </div>

    <!-- power button: ring, finger, label -->
    <div class="absolute size-0" :style="{ left: power.left, top: power.top }">
      <svg
        v-if="counting"
        class="absolute -top-8 -left-8 size-16 -rotate-90"
        viewBox="0 0 64 64"
      >
        <circle
          cx="32"
          cy="32"
          r="22"
          fill="none"
          stroke="currentColor"
          stroke-width="3"
          class="text-ink/12"
        />
        <circle
          v-if="pressPower"
          cx="32"
          cy="32"
          r="22"
          fill="none"
          stroke="var(--color-signal)"
          stroke-width="3"
          stroke-linecap="round"
          :stroke-dasharray="RING"
          :stroke-dashoffset="RING * (1 - fraction)"
          class="transition-[stroke-dashoffset] duration-1000 ease-linear"
        />
      </svg>
      <div
        class="absolute left-0 flex -translate-x-1/2 flex-col items-center transition-[transform,opacity] duration-300 ease-out"
        :class="pressPower ? 'opacity-100' : 'opacity-40'"
        :style="{
          transform: `translate(-50%, ${pressPower ? '-100%' : 'calc(-100% - 22px)'})`,
        }"
      >
        <span
          class="block h-10 w-5 rounded-full bg-[#f2cdb0] shadow-[inset_-2px_0_0_#e0ab8a]"
          ><span class="mt-1 ml-1.5 block h-3 w-2 rounded-full bg-[#fbe6d8]"
        /></span>
      </div>
      <span
        class="absolute top-0 left-6 -translate-y-1/2 rounded-full px-2 py-px text-2xs font-semibold whitespace-nowrap transition-colors"
        :class="pressPower ? 'bg-signal text-on-signal' : 'bg-ink/8 text-muted'"
        >{{ t("preparation.dfu.keys.power") }} ·
        {{ t(pressPower ? "studio.hold" : "studio.release") }}</span
      >
    </div>

    <!-- home button: ring, finger, label -->
    <div class="absolute size-0" :style="{ left: home.left, top: home.top }">
      <svg
        v-if="counting"
        class="absolute -top-8 -left-8 size-16 -rotate-90"
        viewBox="0 0 64 64"
      >
        <circle
          cx="32"
          cy="32"
          r="22"
          fill="none"
          stroke="currentColor"
          stroke-width="3"
          class="text-white/15"
        />
        <circle
          cx="32"
          cy="32"
          r="22"
          fill="none"
          stroke="var(--color-signal)"
          stroke-width="3"
          stroke-linecap="round"
          :stroke-dasharray="RING"
          :stroke-dashoffset="RING * (1 - fraction)"
          class="transition-[stroke-dashoffset] duration-1000 ease-linear"
        />
      </svg>
      <div
        class="absolute left-0 flex flex-col items-center transition-[transform,opacity] duration-300 ease-out"
        :class="pressHome ? 'opacity-100' : 'opacity-40'"
        :style="{
          transform: `translate(-50%, ${pressHome ? '-6px' : '18px'})`,
        }"
      >
        <span
          class="block h-10 w-5 rounded-full bg-[#f2cdb0] shadow-[inset_-2px_0_0_#e0ab8a]"
          ><span class="mt-1 ml-1.5 block h-3 w-2 rounded-full bg-[#fbe6d8]"
        /></span>
      </div>
      <span
        class="absolute top-0 -left-11 -translate-x-full -translate-y-1/2 rounded-full px-2 py-px text-2xs font-semibold whitespace-nowrap transition-colors"
        :class="pressHome ? 'bg-signal text-on-signal' : 'bg-ink/8 text-muted'"
        >{{ t("preparation.dfu.keys.home") }} ·
        {{ t(pressHome ? "studio.hold" : "studio.release") }}</span
      >
    </div>

    <!-- cable activity while detecting -->
    <div
      v-if="phase === 'detecting'"
      class="absolute h-[42px] w-2 -translate-x-1/2 overflow-hidden"
      :style="{ left: dock.left, top: `calc(${dock.top} + 20px)` }"
    >
      <span
        class="block size-2 rounded-full bg-signal motion-safe:animate-dfu-pulse-down"
      />
    </div>
  </div>
</template>
