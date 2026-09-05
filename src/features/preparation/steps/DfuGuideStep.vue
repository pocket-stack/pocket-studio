<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { useI18n } from "vue-i18n";

import { useDeviceSession } from "../../../shared/composables/useDeviceSession";
import { useGateway, type DeviceMode } from "../../../shared/gateway";
import DeviceIllustration from "../../../shared/ui/DeviceIllustration.vue";
import AppIcon from "../../../shared/ui/AppIcon.vue";
import StatusPill from "../../../shared/ui/StatusPill.vue";

/**
 * Guided DFU entry for iPod touch (4th gen), following the Legacy iOS Kit
 * DFU helper timings: hold Power+Home for 8 s, release Power, hold Home 8 s.
 */
type Phase =
  "idle" | "countdown" | "holdBoth" | "holdHome" | "detecting" | "timeout";

const HOLD_BOTH_SECONDS = 8;
const HOLD_HOME_SECONDS = 8;
const COUNTDOWN_SECONDS = 3;
const DETECT_TIMEOUT_SECONDS = 20;
const RING_RADIUS = 54;
const RING_CIRCUMFERENCE = 2 * Math.PI * RING_RADIUS;

const emit = defineEmits<{ cancel: [] }>();
const { t } = useI18n();
const { device } = useDeviceSession();
const gateway = useGateway();

const phase = ref<Phase>("idle");
const secondsLeft = ref(0);
const showTroubleshooting = ref(false);
let timer: number | undefined;

const phaseTotal = computed(() => {
  switch (phase.value) {
    case "countdown":
      return COUNTDOWN_SECONDS;
    case "holdBoth":
      return HOLD_BOTH_SECONDS;
    case "holdHome":
      return HOLD_HOME_SECONDS;
    case "detecting":
      return DETECT_TIMEOUT_SECONDS;
    default:
      return 1;
  }
});

const ringOffset = computed(
  () => RING_CIRCUMFERENCE * (1 - secondsLeft.value / phaseTotal.value),
);
const pressPower = computed(() => phase.value === "holdBoth");
const pressHome = computed(
  () => phase.value === "holdBoth" || phase.value === "holdHome",
);
const deviceMode = computed<DeviceMode | undefined>(() => device.value?.mode);
const screen = computed(() => (phase.value === "idle" ? "home" : "off"));

function stopTimer(): void {
  if (timer) window.clearInterval(timer);
  timer = undefined;
}

function runPhase(next: Phase, seconds: number, onDone: () => void): void {
  stopTimer();
  phase.value = next;
  secondsLeft.value = seconds;
  timer = window.setInterval(() => {
    secondsLeft.value -= 1;
    if (secondsLeft.value <= 0) {
      stopTimer();
      onDone();
    }
  }, 1000);
}

function startDetection(): void {
  runPhase("detecting", DETECT_TIMEOUT_SECONDS, () => {
    phase.value = "timeout";
    showTroubleshooting.value = true;
  });
  // Demo-only: a real build observes the USB mode change from the native
  // layer. Here the driver is told that the button sequence just finished.
  window.setTimeout(() => void gateway.demo.setDeviceMode("dfu"), 1500);
}

function start(): void {
  showTroubleshooting.value = false;
  runPhase("countdown", COUNTDOWN_SECONDS, () =>
    runPhase("holdBoth", HOLD_BOTH_SECONDS, () =>
      runPhase("holdHome", HOLD_HOME_SECONDS, startDetection),
    ),
  );
}

function alreadyInDfu(): void {
  startDetection();
}

function reset(): void {
  stopTimer();
  phase.value = "idle";
  secondsLeft.value = 0;
}

onBeforeUnmount(stopTimer);
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-4">
    <div class="grid min-h-0 flex-1 gap-5 lg:grid-cols-[minmax(0,1fr)_280px]">
      <section
        class="card flex flex-col items-center justify-center gap-6 p-6 text-center"
      >
        <div class="relative">
          <DeviceIllustration
            :width="150"
            :press-home="pressHome"
            :press-power="pressPower"
            :screen="screen"
            cable
          />
          <div
            v-if="phase !== 'idle' && phase !== 'timeout'"
            class="absolute -right-24 top-1/2 flex -translate-y-1/2 flex-col items-center"
          >
            <svg
              width="128"
              height="128"
              viewBox="0 0 128 128"
              class="-rotate-90"
            >
              <circle
                cx="64"
                cy="64"
                :r="RING_RADIUS"
                stroke="var(--ps-line)"
                stroke-width="8"
                fill="none"
              />
              <circle
                cx="64"
                cy="64"
                :r="RING_RADIUS"
                :stroke="
                  phase === 'detecting' ? 'var(--ps-info)' : 'var(--ps-signal)'
                "
                stroke-width="8"
                fill="none"
                stroke-linecap="round"
                :stroke-dasharray="RING_CIRCUMFERENCE"
                :stroke-dashoffset="ringOffset"
                class="transition-[stroke-dashoffset] duration-1000 ease-linear"
              />
            </svg>
            <span
              class="absolute inset-0 flex items-center justify-center font-mono text-3xl font-semibold tabular-nums"
            >
              {{ phase === "detecting" ? "" : secondsLeft }}
              <span
                v-if="phase === 'detecting'"
                class="spin block h-6 w-6 rounded-full border-2 border-info border-t-transparent"
              />
            </span>
          </div>
        </div>

        <div class="max-w-md">
          <h3 class="text-lg font-semibold">
            {{ t(`preparation.dfu.phase.${phase}.title`) }}
          </h3>
          <p class="mt-2 text-sm text-muted">
            {{ t(`preparation.dfu.phase.${phase}.body`) }}
          </p>
          <div
            v-if="phase === 'holdBoth' || phase === 'holdHome'"
            class="mt-3 flex items-center justify-center gap-2 text-sm"
          >
            <span
              class="kbd"
              :class="
                pressPower
                  ? 'text-signal border-signal'
                  : 'opacity-40 line-through'
              "
              >{{ t("preparation.dfu.keys.power") }}</span
            >
            <span class="text-muted">+</span>
            <span class="kbd text-signal border-signal">{{
              t("preparation.dfu.keys.home")
            }}</span>
          </div>
        </div>

        <div class="flex flex-wrap items-center justify-center gap-2">
          <button
            v-if="phase === 'idle' || phase === 'timeout'"
            class="btn btn-primary"
            @click="start"
          >
            <AppIcon name="play" :size="16" />
            {{
              phase === "timeout"
                ? t("preparation.dfu.retry")
                : t("preparation.dfu.start")
            }}
          </button>
          <button v-else class="btn btn-secondary" @click="reset">
            <AppIcon name="refresh" :size="16" />
            {{ t("preparation.dfu.restart") }}
          </button>
          <button
            v-if="phase === 'idle'"
            class="btn btn-ghost"
            @click="alreadyInDfu"
          >
            {{ t("preparation.dfu.alreadyInDfu") }}
          </button>
        </div>
      </section>

      <aside class="flex min-h-0 flex-col gap-3">
        <div class="card p-4">
          <h4 class="text-sm font-semibold">
            {{ t("preparation.dfu.deviceState") }}
          </h4>
          <div class="mt-2 flex items-center gap-2">
            <StatusPill
              v-if="deviceMode"
              :tone="deviceMode === 'dfu' ? 'success' : 'neutral'"
              dot
            >
              {{ t(`device.mode.${deviceMode}`) }}
            </StatusPill>
            <StatusPill v-else tone="danger" dot>{{
              t("preparation.dfu.disconnected")
            }}</StatusPill>
          </div>
          <p class="mt-2 text-xs text-muted">
            {{ t("preparation.dfu.autoAdvance") }}
          </p>
        </div>

        <ol class="card space-y-3 p-4 text-sm">
          <li
            v-for="(key, index) in [
              'plug',
              'holdBoth',
              'releasePower',
              'blackScreen',
            ]"
            :key="key"
            class="flex gap-3"
          >
            <span class="font-mono text-xs text-muted">{{ index + 1 }}</span>
            <span>{{ t(`preparation.dfu.summary.${key}`) }}</span>
          </li>
        </ol>

        <div class="card p-4">
          <button
            class="flex w-full items-center justify-between text-sm font-semibold"
            @click="showTroubleshooting = !showTroubleshooting"
          >
            {{ t("preparation.dfu.troubleshooting.title") }}
            <AppIcon
              :name="showTroubleshooting ? 'cross' : 'info'"
              :size="16"
              class="text-muted"
            />
          </button>
          <ul
            v-if="showTroubleshooting"
            class="mt-3 space-y-2 text-xs text-muted"
          >
            <li
              v-for="key in [
                'appleLogo',
                'recoveryScreen',
                'nothing',
                'exitDfu',
              ]"
              :key="key"
            >
              <span class="font-medium text-ink">{{
                t(`preparation.dfu.troubleshooting.${key}.title`)
              }}</span>
              {{ t(`preparation.dfu.troubleshooting.${key}.body`) }}
            </li>
          </ul>
        </div>
      </aside>
    </div>

    <footer class="flex items-center justify-between">
      <button class="btn btn-danger" @click="emit('cancel')">
        <AppIcon name="stop" :size="16" />
        {{ t("preparation.execution.cancel") }}
      </button>
      <span class="text-xs text-muted">{{
        t("preparation.dfu.safeToCancel")
      }}</span>
    </footer>
  </div>
</template>
