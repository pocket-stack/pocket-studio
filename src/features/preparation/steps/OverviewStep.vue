<script setup lang="ts">
import IconStudioBattery from "~icons/studio/battery";
import IconStudioDevice from "~icons/studio/device";
import IconStudioShield from "~icons/studio/shield";
import IconStudioCable from "~icons/studio/cable";
import IconStudioClock from "~icons/studio/clock";
import { computed, type Component } from "vue";
import { useI18n } from "vue-i18n";

import type { PrerequisiteId, PreparationPlan } from "../../../shared/gateway";
import StatusPill from "../../../shared/ui/StatusPill.vue";

const props = defineProps<{
  plan: PreparationPlan;
  confirmed: readonly PrerequisiteId[];
  allConfirmed: boolean;
  sshPassword?: string;
}>();
const emit = defineEmits<{
  toggle: [id: PrerequisiteId];
  "update:sshPassword": [value: string];
  next: [];
  cancel: [];
}>();
const { t } = useI18n();

const totalMinutes = computed(() =>
  Math.ceil(
    props.plan.steps.reduce((sum, step) => sum + step.estimatedSeconds, 0) / 60,
  ),
);

const facts = computed(() => [
  {
    label: t("preparation.overview.facts.method"),
    value: t(`preparation.overview.method.${props.plan.method}`),
  },
  ...(props.plan.exploit
    ? [
        {
          label: t("preparation.overview.facts.exploit"),
          value: props.plan.exploit,
        },
      ]
    : []),
  {
    label: t(
      props.plan.entryMode === "dfu"
        ? "preparation.overview.facts.requiredTarget"
        : "preparation.overview.facts.target",
    ),
    value: `iOS ${props.plan.targetOsVersion}`,
  },
  ...(props.plan.tether
    ? [
        {
          label: t("preparation.overview.facts.tether"),
          value: t(`preparation.overview.tether.${props.plan.tether}`),
        },
      ]
    : []),
  {
    label: t("preparation.overview.facts.dataLoss"),
    value: t(`preparation.overview.dataLoss.${props.plan.dataLoss}`),
  },
  {
    label: t("preparation.overview.facts.duration"),
    value: t("preparation.overview.minutes", { minutes: totalMinutes.value }),
  },
]);

const prerequisiteIcons: Record<PrerequisiteId, Component> = {
  batteryAbove50: IconStudioBattery,
  workingButtons: IconStudioDevice,
  backupCompleted: IconStudioShield,
  stableCable: IconStudioCable,
  computerAwake: IconStudioClock,
};
</script>

<template>
  <div class="flex min-h-0 flex-1 flex-col gap-5">
    <p
      v-if="plan.entryMode === 'dfu'"
      role="status"
      class="rounded-lg border border-info/35 bg-info/5 p-4 text-sm leading-7"
    >
      {{ t("preparation.overview.dfuEntry") }}
    </p>
    <div class="grid gap-5 lg:grid-cols-[1.2fr_1fr]">
      <section class="p-5 rounded-lg border border-line bg-surface">
        <h3 class="text-base font-semibold">
          {{ t("preparation.overview.whatHappens") }}
        </h3>
        <p v-if="plan.entryMode === 'normal'" class="mt-1 text-sm text-muted">
          {{
            t(
              plan.workflow === "appSync"
                ? "preparation.appSync.intro"
                : "preparation.overview.intro",
            )
          }}
        </p>
        <dl class="mt-4 grid grid-cols-2 gap-x-6 gap-y-2 text-sm">
          <div
            v-for="fact in facts"
            :key="fact.label"
            class="min-w-0 border-b border-line/60 py-2"
          >
            <dt class="text-xs text-muted">{{ fact.label }}</dt>
            <dd class="mt-1 leading-6 font-medium">{{ fact.value }}</dd>
          </div>
        </dl>
        <p class="mt-4 text-sm leading-6 text-muted">
          {{ t("preparation.appSync.packages") }}
        </p>
        <ul
          v-if="plan.systemPackages?.length"
          class="mt-2 space-y-1 text-xs text-muted"
        >
          <li v-for="pkg in plan.systemPackages" :key="pkg.name">
            {{ pkg.name }} · {{ pkg.version }}
          </li>
        </ul>
        <h4 class="mt-5 text-sm font-semibold">
          {{ t("preparation.overview.stepsHeading") }}
        </h4>
        <ol class="mt-2 space-y-1 text-sm">
          <li
            v-for="(step, index) in plan.steps"
            :key="step.id"
            class="flex items-center gap-2"
          >
            <span class="w-5 font-mono text-xs text-muted">{{
              index + 1
            }}</span>
            <span>{{ t(`preparation.steps.${step.id}.title`) }}</span>
            <StatusPill v-if="step.pointOfNoReturn" tone="danger">{{
              t("operation.pointOfNoReturn")
            }}</StatusPill>
            <StatusPill v-else-if="!step.cancellable" tone="neutral">{{
              t("operation.notCancellable")
            }}</StatusPill>
          </li>
        </ol>
      </section>

      <section class="p-5 rounded-lg border border-line bg-surface">
        <h3 class="text-base font-semibold">
          {{ t("preparation.overview.prerequisites") }}
        </h3>
        <p class="mt-1 text-sm text-muted">
          {{ t("preparation.overview.prerequisitesHint") }}
        </p>
        <label
          v-if="sshPassword !== undefined"
          class="mt-4 block rounded-lg border border-line p-3 text-sm"
        >
          {{ t("preparation.appSync.password") }}
          <input
            type="password"
            autocomplete="off"
            :value="sshPassword"
            maxlength="1024"
            class="mt-2 w-full rounded border border-line bg-canvas px-3 py-2"
            @input="
              emit(
                'update:sshPassword',
                ($event.target as HTMLInputElement).value,
              )
            "
          />
          <span class="mt-2 block text-xs leading-6 text-muted">{{
            t("preparation.appSync.passwordHint")
          }}</span>
        </label>
        <ul class="mt-4 space-y-2">
          <li v-for="id in plan.prerequisites" :key="id">
            <label
              class="flex cursor-pointer items-start gap-3 rounded-lg border p-3 transition"
              :class="
                confirmed.includes(id)
                  ? 'border-success/50 bg-success/6'
                  : 'border-line hover:border-muted'
              "
            >
              <input
                type="checkbox"
                class="mt-1 accent-signal"
                :checked="confirmed.includes(id)"
                @change="emit('toggle', id)"
              />
              <component
                :is="prerequisiteIcons[id]"
                class="mt-0.5 text-muted"
              />
              <span class="flex-1">
                <span class="block text-sm font-medium">{{
                  t(`preparation.prerequisites.${id}.title`)
                }}</span>
                <span class="block text-xs text-muted">{{
                  t(`preparation.prerequisites.${id}.detail`)
                }}</span>
              </span>
            </label>
          </li>
        </ul>
      </section>
    </div>

    <footer class="mt-auto flex items-center justify-between">
      <button
        class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-transparent text-muted enabled:hover:bg-ink/6 enabled:hover:text-ink"
        @click="emit('cancel')"
      >
        {{ t("common.cancel") }}
      </button>
      <button
        class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
        :disabled="!allConfirmed"
        @click="emit('next')"
      >
        {{ t("preparation.overview.continue") }}
        <IconStudioArrowRight width="16" height="16" />
      </button>
    </footer>
  </div>
</template>
