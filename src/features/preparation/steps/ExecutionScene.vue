<script setup lang="ts">
import IconPhCloudArrowDown from "~icons/ph/cloud-arrow-down";
import IconPhCpu from "~icons/ph/cpu";
import IconPhHardDrives from "~icons/ph/hard-drives";
import IconPhPackage from "~icons/ph/package";
import IconPhArrowsClockwise from "~icons/ph/arrows-clockwise";
import IconPhShieldCheck from "~icons/ph/shield-check";
import IconPhTerminalWindow from "~icons/ph/terminal-window";
import IconPhLightning from "~icons/ph/lightning";
import IconPhKey from "~icons/ph/key";
import IconPhLaptop from "~icons/ph/laptop";
import { computed, type Component } from "vue";

import type { StepId } from "../../../shared/gateway";
import DeviceIllustration from "../../../shared/ui/DeviceIllustration.vue";
import { deviceScreen } from "../../../shared/ui/deviceGeometry";

/**
 * What the machine is doing, drawn instead of described: the device screen
 * shows the state the real device is in, data pulses along the cable in the
 * direction it actually travels, and a badge names the kind of work.
 */
type Screen = "off" | "boot" | "apple" | "home" | "terminal" | "verified";
type Flow = "none" | "toDevice" | "fromDevice";
interface Scene {
  screen: Screen;
  flow: Flow;
  icon: Component;
  computerBusy?: boolean;
}

const scenes: Partial<Record<StepId, Scene>> = {
  fetchResources: {
    screen: "off",
    flow: "none",
    icon: IconPhCloudArrowDown,
    computerBusy: true,
  },
  buildRamdisk: {
    screen: "off",
    flow: "none",
    icon: IconPhCpu,
    computerBusy: true,
  },
  enterDfu: { screen: "off", flow: "fromDevice", icon: IconPhLightning },
  exploitBootrom: { screen: "off", flow: "toDevice", icon: IconPhLightning },
  bootRamdisk: { screen: "boot", flow: "toDevice", icon: IconPhHardDrives },
  mountFilesystem: { screen: "boot", flow: "none", icon: IconPhHardDrives },
  installUntether: { screen: "boot", flow: "toDevice", icon: IconPhPackage },
  rebootDevice: { screen: "apple", flow: "none", icon: IconPhArrowsClockwise },
  verifyJailbreak: {
    screen: "verified",
    flow: "fromDevice",
    icon: IconPhShieldCheck,
  },
  connectAppSync: { screen: "home", flow: "fromDevice", icon: IconPhKey },
  installAppSync: {
    screen: "terminal",
    flow: "toDevice",
    icon: IconPhPackage,
  },
  activateAppSync: {
    screen: "terminal",
    flow: "none",
    icon: IconPhTerminalWindow,
  },
  verifyAppSync: {
    screen: "verified",
    flow: "fromDevice",
    icon: IconPhShieldCheck,
  },
};
const fallback: Scene = { screen: "off", flow: "none", icon: IconPhCpu };

const props = withDefaults(defineProps<{ stepId?: StepId; width?: number }>(), {
  stepId: undefined,
  width: 140,
});
const scene = computed(
  () => (props.stepId && scenes[props.stepId]) ?? fallback,
);
const builtin = computed(() =>
  scene.value.screen === "apple" || scene.value.screen === "home"
    ? scene.value.screen
    : scene.value.screen === "verified" || scene.value.screen === "terminal"
      ? "home"
      : "off",
);
const s = deviceScreen;
const bootLines = [72, 118, 96, 140, 60, 124, 88, 132];
const terminalLines = [90, 130, 70, 110, 120];
</script>

<template>
  <div class="flex flex-col items-center" aria-hidden="true">
    <DeviceIllustration :width="width" :screen="builtin" cable shadow>
      <template v-if="scene.screen === 'boot'" #screen>
        <g>
          <rect
            :x="s.x"
            :y="s.y"
            :width="s.width"
            :height="s.height"
            fill="#050507"
          />
          <rect
            v-for="(line, index) in bootLines"
            :key="index"
            :x="s.x + 12"
            :y="s.y + 16 + index * 13"
            :width="line"
            height="5"
            rx="1"
            fill="#d9dbe0"
            class="motion-safe:animate-boot-line"
            :style="{ animationDelay: `${index * 0.35}s` }"
          />
        </g>
      </template>
      <template v-else-if="scene.screen === 'terminal'" #screen>
        <g>
          <rect
            :x="s.x"
            :y="s.y"
            :width="s.width"
            :height="s.height"
            fill="#0b0d10"
            opacity="0.92"
          />
          <rect
            v-for="(line, index) in terminalLines"
            :key="index"
            :x="s.x + 12"
            :y="s.y + 22 + index * 16"
            :width="line"
            height="6"
            rx="1"
            fill="#7fd08a"
            class="motion-safe:animate-boot-line"
            :style="{ animationDelay: `${index * 0.5}s` }"
          />
          <rect
            :x="s.x + 12"
            :y="s.y + 22 + terminalLines.length * 16"
            width="8"
            height="9"
            fill="#7fd08a"
            class="motion-safe:animate-pulse"
          />
        </g>
      </template>
      <template v-else-if="scene.screen === 'verified'" #screen>
        <g>
          <rect
            :x="s.x"
            :y="s.y"
            :width="s.width"
            :height="s.height"
            fill="#0b0d10"
            opacity="0.72"
          />
          <g
            class="origin-center motion-safe:animate-pop"
            :style="{
              transformOrigin: `${s.x + s.width / 2}px ${s.y + s.height / 2}px`,
            }"
          >
            <circle
              :cx="s.x + s.width / 2"
              :cy="s.y + s.height / 2"
              r="34"
              fill="var(--color-success)"
            />
            <path
              :d="`M${s.x + s.width / 2 - 15} ${s.y + s.height / 2 + 1}l10 10 20-22`"
              fill="none"
              stroke="#fff"
              stroke-width="5"
              stroke-linecap="round"
              stroke-linejoin="round"
            />
          </g>
        </g>
      </template>
    </DeviceIllustration>

    <!-- cable with directional data pulses and the work badge -->
    <div class="relative h-14 w-14">
      <span
        class="absolute top-0 bottom-0 left-1/2 w-[3px] -translate-x-1/2 rounded-full bg-[#b8babf]"
      />
      <template v-if="scene.flow !== 'none'">
        <span
          v-for="index in 3"
          :key="index"
          class="absolute left-1/2 size-1.5 -translate-x-1/2 rounded-full bg-signal"
          :class="
            scene.flow === 'toDevice'
              ? 'motion-safe:animate-flow-up'
              : 'motion-safe:animate-flow-down'
          "
          :style="{ animationDelay: `${(index - 1) * 0.4}s` }"
        />
      </template>
      <span
        class="absolute top-1/2 left-1/2 grid size-7 -translate-x-1/2 -translate-y-1/2 place-items-center rounded-full bg-raised text-signal shadow-control"
      >
        <component :is="scene.icon" width="15" height="15" />
      </span>
    </div>
    <div
      class="flex items-center gap-1.5 rounded-control bg-ink/6 px-2 py-1 text-muted"
      :class="scene.computerBusy && 'text-signal'"
    >
      <IconPhLaptop width="18" height="18" />
      <IconSvgSpinners90Ring v-if="scene.computerBusy" width="12" height="12" />
    </div>
  </div>
</template>
