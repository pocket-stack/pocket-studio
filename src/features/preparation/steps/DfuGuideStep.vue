<script setup lang="ts">
import { computed, onBeforeUnmount, ref } from "vue";
import { useI18n } from "vue-i18n";

import { useDeviceSession } from "../../../shared/composables/useDeviceSession";
import { useGateway } from "../../../shared/gateway";
import DeviceIllustration from "../../../shared/ui/DeviceIllustration.vue";
import AppIcon from "../../../shared/ui/AppIcon.vue";

/**
 * Guided DFU entry for iPod touch (4th gen), following the Legacy iOS Kit
 * DFU helper timings: hold Power+Home for 10 s (8 s from Recovery), then hold Home for 8 s.
 */
type Phase =
  "idle" | "countdown" | "holdBoth" | "holdHome" | "detecting" | "timeout";

const holdBothSeconds = ref(10);
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

let timer: number | undefined;
let detectionTimer: number | undefined;

const phaseTotal = computed(() => {
  switch (phase.value) {
    case "countdown":
      return COUNTDOWN_SECONDS;
    case "holdBoth":
      return holdBothSeconds.value;
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
const screen = computed(() => (phase.value === "idle" ? "home" : "off"));

function stopTimer(): void {
  if (detectionTimer) window.clearTimeout(detectionTimer);
  detectionTimer = undefined;
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
  });
  // Demo-only: a real build observes the USB mode change from the native
  // layer. Here the driver is told that the button sequence just finished.
  detectionTimer = window.setTimeout(
    () => void gateway.demo.setDeviceMode("dfu"),
    1500,
  );
}

function start(): void {
  holdBothSeconds.value = device.value?.mode === "recovery" ? 8 : 10;

  runPhase("countdown", COUNTDOWN_SECONDS, () =>
    runPhase("holdBoth", holdBothSeconds.value, () =>
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
    <p class="text-muted text-left text-[10px] leading-[1.333333]">
      {{ t("studio.dfuDemo") }}
    </p>
    <div class="grid min-h-0 flex-1 gap-5 max-[850px]:overflow-y-auto">
      <section
        class="flex flex-col items-center justify-center gap-6 text-center rounded-lg border-line border-0 bg-transparent p-2.5 [@media(max-height:740px)]:gap-[13px]"
      >
        <div class="relative mx-0 mt-5 mb-[5px]">
          <div
            :data-pressed="pressPower"
            class="group/dfu-button absolute w-[105px] text-left text-[11px] text-muted data-[pressed=true]:font-medium data-[pressed=true]:text-signal top-0 left-[230px] max-[1150px]:left-[175px]"
          >
            <span>{{ t("studio.dfuPower") }}</span
            ><small class="mt-[5px] block text-[10px]">{{
              t(pressPower ? "studio.hold" : "studio.release")
            }}</small
            ><i
              class="absolute top-[7px] h-px w-[50px] bg-line group-data-[pressed=true]/dfu-button:bg-signal right-[111px]"
            />
          </div>
          <div
            :data-pressed="pressHome"
            class="group/dfu-button absolute w-[105px] text-[11px] text-muted data-[pressed=true]:font-medium data-[pressed=true]:text-signal right-[220px] bottom-[50px] text-right max-[1150px]:right-[170px] max-[1150px]:bottom-10"
          >
            <span>{{ t("studio.dfuHome") }}</span
            ><small class="mt-[5px] block text-[10px]">{{
              t(pressHome ? "studio.hold" : "studio.release")
            }}</small
            ><i
              class="absolute top-[7px] h-px w-[50px] bg-line group-data-[pressed=true]/dfu-button:bg-signal left-[111px]"
            />
          </div>
          <DeviceIllustration
            class="w-[220px] max-[1150px]:w-[170px]"
            :width="160"
            :press-home="pressHome"
            :press-power="pressPower"
            :screen="screen"
            cable
          />
          <div
            v-if="phase !== 'idle' && phase !== 'timeout'"
            class="absolute top-[146px] left-16 size-[92px] text-white max-[1150px]:top-[105px] max-[1150px]:left-[39px]"
          >
            <svg
              width="92"
              height="92"
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
              class="absolute inset-0 flex items-center justify-center font-mono font-semibold tabular-nums text-[25px] leading-[1.2]"
            >
              {{ phase === "detecting" ? "" : secondsLeft }}
              <span
                v-if="phase === 'detecting'"
                class="block h-6 w-6 rounded-full border-2 border-info border-t-transparent motion-safe:animate-studio-spin"
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
              class="rounded-md border border-b-2 border-line bg-raised px-1.5 py-px font-mono text-[12px]"
              :class="
                pressPower
                  ? 'text-signal border-signal'
                  : 'opacity-40 line-through'
              "
              >{{ t("preparation.dfu.keys.power") }}</span
            >
            <span class="text-muted">+</span>
            <span
              class="text-signal rounded-md border border-b-2 border-line bg-raised px-1.5 py-px font-mono text-[12px]"
              >{{ t("preparation.dfu.keys.home") }}</span
            >
          </div>
        </div>

        <div class="flex flex-wrap items-center justify-center gap-2">
          <button
            v-if="phase === 'idle' || phase === 'timeout'"
            class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
            @click="start"
          >
            <AppIcon name="play" :size="16" />
            {{
              phase === "timeout"
                ? t("preparation.dfu.retry")
                : t("preparation.dfu.start")
            }}
          </button>
          <button
            v-else
            class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
            @click="reset"
          >
            <AppIcon name="refresh" :size="16" />
            {{ t("preparation.dfu.restart") }}
          </button>
          <button
            v-if="phase === 'idle'"
            class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-transparent text-muted enabled:hover:bg-ink/6 enabled:hover:text-ink"
            @click="alreadyInDfu"
          >
            {{ t("preparation.dfu.alreadyInDfu") }}
          </button>
        </div>
      </section>
    </div>

    <footer class="items-center justify-between hidden">
      <button
        class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-danger bg-transparent text-danger enabled:hover:bg-danger/10"
        @click="emit('cancel')"
      >
        <AppIcon name="stop" :size="16" />
        {{ t("preparation.execution.cancel") }}
      </button>
      <span class="text-xs text-muted">{{
        t("preparation.dfu.safeToCancel")
      }}</span>
    </footer>
  </div>
</template>
