<script setup lang="ts">
import CountdownRing from "../../../assets/indicators/countdown-ring.svg?component";
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";

import { useGateway } from "../../../shared/gateway";
import DeviceIllustration from "../../../shared/ui/DeviceIllustration.vue";

/** Start fully powered off, hold Power + Home for 10 s, then Home for 8 s. */
type Phase = "idle" | "holdBoth" | "holdHome" | "detecting" | "timeout";
const HOLD_BOTH_SECONDS = 10;
const HOLD_HOME_SECONDS = 8;
const DETECT_TIMEOUT_SECONDS = 20;

const emit = defineEmits<{ cancel: [] }>();
const { t } = useI18n();
const gateway = useGateway();
const phase = ref<Phase>("idle");
const secondsLeft = ref(0);
let startedAt: number | undefined;
let timer: number | undefined;
let detectionTimer: number | undefined;

const phaseTotal = computed(() => {
  switch (phase.value) {
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
  () => 100 * (1 - secondsLeft.value / phaseTotal.value),
);
const pressPower = computed(() => phase.value === "holdBoth");
const pressHome = computed(
  () => phase.value === "holdBoth" || phase.value === "holdHome",
);

function stopTimer(): void {
  if (detectionTimer !== undefined) window.clearTimeout(detectionTimer);
  if (timer !== undefined) window.clearInterval(timer);
  detectionTimer = undefined;
  timer = undefined;
  startedAt = undefined;
}

function tick(): void {
  if (startedAt === undefined) return;
  const elapsed = (performance.now() - startedAt) / 1000;
  const homeEndsAt = HOLD_BOTH_SECONDS + HOLD_HOME_SECONDS;
  if (elapsed < HOLD_BOTH_SECONDS) {
    phase.value = "holdBoth";
    secondsLeft.value = Math.ceil(HOLD_BOTH_SECONDS - elapsed);
  } else if (elapsed < homeEndsAt) {
    phase.value = "holdHome";
    secondsLeft.value = Math.ceil(homeEndsAt - elapsed);
  } else if (elapsed < homeEndsAt + DETECT_TIMEOUT_SECONDS) {
    if (phase.value !== "detecting" && gateway.capabilities.demo) {
      // Browser preview only. A desktop run advances solely from native USB events.
      detectionTimer = window.setTimeout(
        () => void gateway.demo.setDeviceMode("dfu"),
        1500,
      );
    }
    phase.value = "detecting";
    secondsLeft.value = Math.ceil(
      homeEndsAt + DETECT_TIMEOUT_SECONDS - elapsed,
    );
  } else {
    stopTimer();
    phase.value = "timeout";
    secondsLeft.value = 0;
  }
}

function start(): void {
  stopTimer();
  startedAt = performance.now();
  tick();
  timer = window.setInterval(tick, 100);
}

function reset(): void {
  stopTimer();
  phase.value = "idle";
  secondsLeft.value = 0;
}

onMounted(() => {
  window.addEventListener("focus", tick);
  document.addEventListener("visibilitychange", tick);
});
onBeforeUnmount(() => {
  stopTimer();
  window.removeEventListener("focus", tick);
  document.removeEventListener("visibilitychange", tick);
});
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-4">
    <p class="text-muted text-left text-[10px] leading-[1.333333]">
      {{
        t(
          gateway.capabilities.demo
            ? "studio.dfuDemo"
            : "preparation.dfu.autoAdvance",
        )
      }}
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
            screen="off"
            cable
          />
          <div
            v-if="phase !== 'idle' && phase !== 'timeout'"
            class="absolute top-[146px] left-16 size-[92px] text-white max-[1150px]:top-[105px] max-[1150px]:left-[39px]"
          >
            <CountdownRing
              aria-hidden="true"
              class="block size-full"
              :style="{
                '--countdown-color':
                  phase === 'detecting' ? 'var(--ps-info)' : 'var(--ps-signal)',
                '--countdown-offset': ringOffset,
              }"
            />
            <span
              class="absolute inset-0 flex items-center justify-center font-mono font-semibold tabular-nums text-[25px] leading-[1.2]"
            >
              {{ phase === "detecting" ? "" : secondsLeft }}
              <IconSvgSpinnersRingResize
                v-if="phase === 'detecting'"
                width="24"
                height="24"
                class="text-info"
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
            <IconPhPlayFill width="16" height="16" />
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
            <IconPhArrowsClockwise width="16" height="16" />
            {{ t("preparation.dfu.restart") }}
          </button>
        </div>
      </section>
    </div>

    <footer class="items-center justify-between hidden">
      <button
        class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-danger bg-transparent text-danger enabled:hover:bg-danger/10"
        @click="emit('cancel')"
      >
        <IconPhStop width="16" height="16" />
        {{ t("preparation.execution.cancel") }}
      </button>
      <span class="text-xs text-muted">{{
        t("preparation.dfu.safeToCancel")
      }}</span>
    </footer>
  </div>
</template>
