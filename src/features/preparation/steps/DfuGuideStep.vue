<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { useI18n } from "vue-i18n";

import { useGateway } from "../../../shared/gateway";
import StudioButton from "../../../shared/ui/StudioButton.vue";
import DfuScene, { type DfuPhase } from "./DfuScene.vue";

/** Start fully powered off, hold Power + Home for 10 s, then Home for 8 s. */
const HOLD_BOTH_SECONDS = 10;
const HOLD_HOME_SECONDS = 8;
const DETECT_TIMEOUT_SECONDS = 20;

const emit = defineEmits<{ cancel: [] }>();
const { t } = useI18n();
const gateway = useGateway();
const phase = ref<DfuPhase>("idle");
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
const pressPower = computed(() => phase.value === "holdBoth");
const pressHome = computed(
  () => phase.value === "holdBoth" || phase.value === "holdHome",
);
const troubleshooting = ["appleLogo", "recoveryScreen", "nothing"] as const;

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
  <div class="flex h-full min-h-0 flex-col">
    <header class="flex items-center justify-between gap-3">
      <h1 class="text-xl font-semibold">
        {{ t("preparation.steps.enterDfu.title") }}
      </h1>
      <span
        class="rounded-full px-2.5 py-px text-xs font-semibold"
        :class="
          phase === 'detecting'
            ? 'bg-info/12 text-info'
            : 'bg-warning/14 text-warning motion-safe:animate-pulse'
        "
        >{{
          t(
            phase === "detecting"
              ? "preparation.execution.mode.auto"
              : "preparation.execution.mode.manual",
          )
        }}</span
      >
    </header>
    <div class="flex min-h-0 flex-1 items-center gap-8 py-2 pl-24">
      <div class="shrink-0">
        <DfuScene
          :phase="phase"
          :seconds-left="secondsLeft"
          :phase-total="phaseTotal"
          :width="150"
        />
      </div>
      <div class="flex min-w-0 max-w-[340px] flex-1 flex-col gap-3">
        <div>
          <h3 class="text-lg font-semibold">
            {{ t(`preparation.dfu.phase.${phase}.title`) }}
          </h3>
          <p class="mt-1 text-sm text-muted">
            {{ t(`preparation.dfu.phase.${phase}.body`) }}
          </p>
        </div>
        <div
          v-if="phase === 'holdBoth' || phase === 'holdHome'"
          class="flex items-center gap-2 text-sm"
        >
          <span
            class="rounded-control px-2 py-0.5 font-mono text-xs transition-colors"
            :class="
              pressPower
                ? 'bg-signal/12 text-signal'
                : 'bg-ink/6 text-muted line-through'
            "
            >{{ t("preparation.dfu.keys.power") }}</span
          >
          <span class="text-muted">+</span>
          <span
            class="rounded-control px-2 py-0.5 font-mono text-xs"
            :class="
              pressHome ? 'bg-signal/12 text-signal' : 'bg-ink/6 text-muted'
            "
            >{{ t("preparation.dfu.keys.home") }}</span
          >
        </div>
        <ul
          v-if="phase === 'timeout'"
          class="flex flex-col gap-1 text-xs text-muted"
        >
          <li v-for="item in troubleshooting" :key="item">
            <b class="font-medium text-ink">{{
              t(`preparation.dfu.troubleshooting.${item}.title`)
            }}</b>
            {{ t(`preparation.dfu.troubleshooting.${item}.body`) }}
          </li>
        </ul>
        <div class="flex flex-wrap items-center gap-2">
          <StudioButton
            v-if="phase === 'idle' || phase === 'timeout'"
            variant="primary"
            @click="start"
          >
            <IconPhPlayFill width="14" height="14" />
            {{
              phase === "timeout"
                ? t("preparation.dfu.retry")
                : t("preparation.dfu.start")
            }}
          </StudioButton>
          <StudioButton v-else @click="reset">
            <IconPhArrowsClockwise width="14" height="14" />
            {{ t("preparation.dfu.restart") }}
          </StudioButton>
          <StudioButton variant="ghost" @click="emit('cancel')">
            {{ t("preparation.execution.cancel") }}
          </StudioButton>
        </div>
        <p class="text-2xs text-muted">
          {{
            t(
              gateway.capabilities.demo
                ? "studio.dfuDemo"
                : "preparation.dfu.autoAdvance",
            )
          }}
        </p>
      </div>
    </div>
  </div>
</template>
