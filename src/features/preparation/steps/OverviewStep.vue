<script setup lang="ts">
import IconPhBatteryMedium from "~icons/ph/battery-medium";
import IconPhDeviceMobile from "~icons/ph/device-mobile";
import IconPhShieldCheck from "~icons/ph/shield-check";
import IconPhPlugs from "~icons/ph/plugs";
import IconPhClock from "~icons/ph/clock";
import { computed, ref, type Component } from "vue";
import { useI18n } from "vue-i18n";

import type { PrerequisiteId, PreparationPlan } from "../../../shared/gateway";
import { useGateway } from "../../../shared/gateway";
import { useElementSize } from "../../../shared/composables/useElementSize";
import { useDefaultSshPassword } from "../../../shared/preferences/sshPassword";
import StudioButton from "../../../shared/ui/StudioButton.vue";
import StudioCallout from "../../../shared/ui/StudioCallout.vue";
import StudioInput from "../../../shared/ui/StudioInput.vue";
import StudioPanel from "../../../shared/ui/StudioPanel.vue";

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
const { effective: defaultPassword } = useDefaultSshPassword();

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

// Short windows drop the detail lines so every prerequisite stays visible.
const checklist = ref<HTMLElement | null>(null);
const { height: checklistHeight } = useElementSize(checklist);
const compact = computed(
  () => checklistHeight.value > 0 && checklistHeight.value < 300,
);

const prerequisiteIcons: Record<PrerequisiteId, Component> = {
  batteryAbove50: IconPhBatteryMedium,
  workingButtons: IconPhDeviceMobile,
  backupCompleted: IconPhShieldCheck,
  stableCable: IconPhPlugs,
  computerAwake: IconPhClock,
};
</script>

<template>
  <div class="flex h-full min-h-0 flex-col gap-3">
    <header>
      <h1 class="text-xl font-semibold">
        {{ t("preparation.overview.whatHappens") }}
      </h1>
      <p
        v-if="plan.entryMode === 'normal'"
        class="mt-0.5 line-clamp-3 text-sm text-muted"
      >
        {{
          t(
            plan.workflow === "appSync"
              ? "preparation.appSync.intro"
              : "preparation.overview.intro",
          )
        }}
      </p>
    </header>
    <StudioCallout v-if="plan.entryMode === 'dfu'" tone="info">
      {{ t("preparation.overview.dfuEntry") }}
    </StudioCallout>
    <StudioPanel :padded="false" class="flex divide-x-0 px-2 py-2">
      <div v-for="fact in facts" :key="fact.label" class="min-w-0 flex-1 px-3">
        <dt class="truncate text-2xs text-muted">{{ fact.label }}</dt>
        <dd class="truncate text-sm font-medium" :title="fact.value">
          {{ fact.value }}
        </dd>
      </div>
    </StudioPanel>
    <div class="grid min-h-0 flex-1 grid-cols-[1.15fr_1fr] gap-3">
      <StudioPanel class="flex min-h-0 flex-col overflow-hidden">
        <h3 class="text-sm font-semibold">
          {{ t("preparation.overview.stepsHeading") }}
        </h3>
        <ol
          class="mt-2 grid grid-cols-2 gap-x-4 gap-y-0.5 text-xs"
          :class="plan.steps.length <= 7 && 'grid-cols-1'"
        >
          <li
            v-for="(step, index) in plan.steps"
            :key="step.id"
            class="flex items-center gap-2 leading-[20px]"
          >
            <span class="w-4 shrink-0 text-right font-mono text-muted">{{
              index + 1
            }}</span>
            <span class="truncate">{{
              t(`preparation.steps.${step.id}.title`)
            }}</span>
            <IconPhWarning
              v-if="step.pointOfNoReturn"
              width="12"
              height="12"
              class="shrink-0 text-danger"
              :title="t('operation.pointOfNoReturn')"
            />
            <IconPhShieldCheck
              v-else-if="!step.cancellable"
              width="12"
              height="12"
              class="shrink-0 text-muted"
              :title="t('operation.notCancellable')"
            />
          </li>
        </ol>
        <div
          class="mt-auto flex flex-wrap gap-x-4 gap-y-1 pt-3 text-2xs text-muted"
        >
          <span class="flex items-center gap-1"
            ><IconPhWarning width="11" height="11" class="text-danger" />{{
              t("operation.pointOfNoReturn")
            }}</span
          >
          <span class="flex items-center gap-1"
            ><IconPhShieldCheck width="11" height="11" />{{
              t("operation.notCancellable")
            }}</span
          >
        </div>
        <p v-if="plan.systemPackages?.length" class="mt-2 text-2xs text-muted">
          {{ t("preparation.appSync.packages") }}
          <span v-for="pkg in plan.systemPackages" :key="pkg.name"
            >{{ pkg.name }} {{ pkg.version }};
          </span>
        </p>
      </StudioPanel>
      <StudioPanel class="flex min-h-0 flex-col overflow-hidden">
        <div ref="checklist" class="flex min-h-0 flex-1 flex-col">
          <h3 class="text-sm font-semibold">
            {{ t("preparation.overview.prerequisites") }}
          </h3>
          <p class="mt-0.5 text-xs text-muted" :class="compact && 'truncate'">
            {{ t("preparation.overview.prerequisitesHint") }}
          </p>
          <label
            v-if="sshPassword !== undefined"
            class="mt-2 flex items-center gap-2 text-xs"
            :title="t('preparation.appSync.passwordHint')"
          >
            <span class="shrink-0 font-medium">{{
              t("preparation.appSync.password")
            }}</span>
            <StudioInput
              :model-value="sshPassword"
              type="password"
              size="sm"
              class="min-w-0 flex-1"
              autocomplete="off"
              maxlength="1024"
              :secret="defaultPassword"
              :label="t('preparation.appSync.password')"
              @update:model-value="emit('update:sshPassword', $event)"
            />
          </label>
          <span
            v-if="sshPassword !== undefined && !compact"
            class="mt-1 block truncate text-2xs text-muted"
            >{{ t("preparation.appSync.passwordHint") }}</span
          >
          <ul class="mt-2 flex flex-col" :class="compact ? 'gap-1' : 'gap-1.5'">
            <li v-for="id in plan.prerequisites" :key="id">
              <label
                class="flex cursor-pointer items-center gap-2.5 rounded-control px-2.5 transition-colors"
                :class="[
                  compact ? 'py-1' : 'py-1.5',
                  confirmed.includes(id)
                    ? 'bg-success/8'
                    : 'bg-ink/4 hover:bg-ink/6',
                ]"
                :title="t(`preparation.prerequisites.${id}.detail`)"
              >
                <input
                  type="checkbox"
                  :checked="confirmed.includes(id)"
                  @change="emit('toggle', id)"
                />
                <component
                  :is="prerequisiteIcons[id]"
                  width="15"
                  height="15"
                  class="shrink-0 text-muted"
                />
                <span class="min-w-0 flex-1">
                  <span class="block text-sm leading-[18px] font-medium">{{
                    t(`preparation.prerequisites.${id}.title`)
                  }}</span>
                  <span
                    v-if="!compact"
                    class="block truncate text-2xs text-muted"
                    >{{ t(`preparation.prerequisites.${id}.detail`) }}</span
                  >
                </span>
              </label>
            </li>
          </ul>
        </div>
      </StudioPanel>
    </div>
    <footer class="flex items-center justify-between gap-3">
      <span v-if="useGateway().capabilities.demo" class="text-2xs text-muted">{{
        t("preparation.demoNotice")
      }}</span>
      <div class="ml-auto flex gap-2">
        <StudioButton variant="ghost" @click="emit('cancel')">
          {{ t("common.cancel") }}
        </StudioButton>
        <StudioButton
          variant="primary"
          :disabled="!allConfirmed"
          @click="emit('next')"
        >
          {{ t("preparation.overview.continue") }}
          <IconPhArrowRight width="14" height="14" />
        </StudioButton>
      </div>
    </footer>
  </div>
</template>
