<script setup lang="ts">
import { computed } from "vue";
import { useI18n } from "vue-i18n";
import type { ReadinessReport } from "../../shared/gateway";
import { useOperations } from "../../shared/composables/useOperations";
import AppIcon from "../../shared/ui/AppIcon.vue";
const props = defineProps<{
  report: ReadinessReport | null;
  checking: boolean;
  compact?: boolean;
}>();
const emit = defineEmits<{
  recheck: [];
  prepare: [];
  details: [];
  openStore: [];
}>();
const { t, d } = useI18n();
const { active } = useOperations();
const checks = computed(
  () =>
    props.report?.checks.filter(
      (check) =>
        !props.compact ||
        [
          "osVersionSupported",
          "pairingTrusted",
          "jailbroken",
          "sshAvailable",
        ].includes(check.id),
    ) ?? [],
);
const passCount = computed(
  () => checks.value.filter((check) => check.status === "pass").length,
);
</script>
<template>
  <section
    :data-detailed="!compact"
    class="group/readiness overflow-hidden rounded-lg border border-line bg-surface data-[detailed=true]:mb-5"
    :aria-busy="checking"
  >
    <header
      class="flex items-center justify-between gap-[15px] px-4 pt-4 pb-1.5"
    >
      <div>
        <h2 class="text-[15px] font-semibold">
          {{
            t(
              report?.status === "ready"
                ? "studio.readinessReady"
                : "studio.readinessTitle",
            )
          }}
        </h2>
      </div>
      <span
        v-if="report"
        :data-ready="report.status === 'ready'"
        class="rounded-[10px] border border-line px-2 py-px text-[12px] whitespace-nowrap text-muted data-[ready=true]:border-success/35 data-[ready=true]:text-success"
        >{{ passCount }} / {{ checks.length }} {{ t("studio.passed") }}</span
      >
    </header>
    <div v-if="!report" class="p-7 text-sm text-muted flex items-center gap-2">
      <AppIcon
        name="refresh"
        :class="{ 'motion-safe:animate-studio-spin': checking }"
        :size="16"
      />{{ t(checking ? "readiness.checking" : "studio.checkFailed") }}
    </div>
    <ul v-else class="px-4 py-0">
      <li
        v-for="check in checks"
        :key="check.id"
        class="flex items-center gap-2.5 border-b border-line px-0 py-[9px] last:border-0 group-data-[detailed=true]/readiness:py-[15px]"
      >
        <span
          :data-status="check.status"
          class="grid h-[18px] w-4 place-items-center rounded-none bg-transparent text-muted data-[status=pass]:text-success data-[status=fail]:text-warning data-[status=warn]:text-warning"
          ><AppIcon
            :name="
              check.status === 'pass'
                ? 'check'
                : check.status === 'fail'
                  ? 'warning'
                  : 'minus'
            "
            :size="14"
        /></span>
        <div class="flex-1">
          <h3 class="text-[13px]">
            {{ t(`readiness.checks.${check.id}.title`) }}
          </h3>
          <p v-if="!compact" class="mt-1 text-[11px] text-muted">
            {{ t(`readiness.checks.${check.id}.${check.status}`) }}
          </p>
        </div>
        <span class="text-[12px] text-muted">{{
          check.value || t(`studio.checkStates.${check.status}`)
        }}</span>
      </li>
    </ul>
    <footer
      class="flex items-center justify-between gap-[15px] px-4 pt-3 pb-4 max-[800px]:flex-wrap"
    >
      <div class="flex flex-wrap gap-2">
        <button
          v-if="report?.status === 'needsPreparation'"
          class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
          :disabled="checking || !!active.length"
          @click="emit('prepare')"
        >
          {{ t("studio.beginPreparation")
          }}<AppIcon name="arrowRight" :size="14" /></button
        ><button
          v-else-if="report?.status === 'ready'"
          class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 bg-signal text-on-signal enabled:hover:brightness-[1.06]"
          @click="emit('openStore')"
        >
          {{ t("device.next.action")
          }}<AppIcon name="arrowRight" :size="14" /></button
        ><button
          class="inline-flex items-center justify-center gap-2 rounded-md px-[15px] py-1.5 text-[13px] leading-[18px] font-medium transition disabled:cursor-not-allowed disabled:opacity-45 border border-[#b5b5b5] bg-raised text-ink enabled:hover:border-muted"
          :disabled="checking"
          @click="emit('recheck')"
        >
          <AppIcon
            name="refresh"
            :size="13"
            :class="{ 'motion-safe:animate-studio-spin': checking }"
          />{{ t("readiness.recheck") }}
        </button>
      </div>
      <button
        v-if="compact"
        class="inline-flex items-center gap-[5px] text-[11px] text-signal hover:underline hover:underline-offset-[3px]"
        @click="emit('details')"
      >
        {{ t("studio.allConditions")
        }}<AppIcon name="chevronRight" :size="12" /></button
      ><span v-else-if="report" class="text-xs text-muted">{{
        t("readiness.checkedAt", { time: d(report.checkedAt, "time") })
      }}</span>
    </footer>
  </section>
</template>
