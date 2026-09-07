<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioDialog from "../../shared/ui/StudioDialog.vue";
import { useStore } from "./useStore";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { formatBytes } from "./compatibility";
const store = useStore();
const { device } = useDeviceSession();
const { t, d, locale, te } = useI18n();
const deleteData = ref(false);
const plan = computed(() => store.operationPlan.value);
watch(
  () => plan.value?.id,
  () => {
    deleteData.value = false;
  },
);
const name = computed(
  () =>
    plan.value?.names[locale.value] ??
    plan.value?.names.en ??
    plan.value?.appId ??
    "",
);
const issue = computed(() => {
  const key = `store.actions.errors.${store.planIssue.value}`;
  return te(key) ? t(key) : t("store.actions.errors.unknown");
});
const changed = computed(
  () => !!plan.value && plan.value.deviceId !== device.value?.id,
);
</script>
<template>
  <StudioDialog
    :open="store.planOpen.value"
    :title="t('store.actions.review')"
    compact
    @close="store.closePlan()"
  >
    <p
      v-if="store.planning.value"
      role="status"
      class="flex items-center gap-2 py-4 text-sm text-muted"
    >
      <IconSvgSpinners90Ring width="14" height="14" />{{
        t("store.actions.planning")
      }}
    </p>
    <div v-if="plan" class="flex flex-col gap-3 text-sm">
      <div>
        <p class="text-xs text-muted">
          {{ t(`store.actions.kind.${plan.action}`) }}
        </p>
        <h3 class="text-lg font-semibold">{{ name }}</h3>
      </div>
      <dl
        class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1.5 rounded-control bg-ink/4 px-3 py-2.5 text-xs"
      >
        <dt class="text-muted">{{ t("store.actions.device") }}</dt>
        <dd class="text-right break-words">{{ plan.deviceName }}</dd>
        <dt class="text-muted">{{ t("store.actions.nativeId") }}</dt>
        <dd class="text-right font-mono break-all">{{ plan.bundleId }}</dd>
        <template v-if="plan.previous"
          ><dt class="text-muted">{{ t("store.actions.current") }}</dt>
          <dd class="text-right">
            {{
              plan.previous.productVersion ??
              t("store.installed.unknownVersion")
            }}<span v-if="plan.previous.buildNumber">
              ·
              {{
                t("store.installed.build", { value: plan.previous.buildNumber })
              }}</span
            >
          </dd></template
        >
        <template v-if="plan.artifact"
          ><dt class="text-muted">{{ t("store.actions.selected") }}</dt>
          <dd class="text-right">
            {{ plan.version }} · r{{ plan.revision }} ·
            {{
              t("store.installed.build", {
                value: plan.artifact.native_identity.build_number,
              })
            }}
          </dd>
          <dt class="text-muted">{{ t("store.installed.packageSize") }}</dt>
          <dd class="text-right">
            {{ formatBytes(plan.artifact.blob.size_bytes) }}
          </dd></template
        >
      </dl>
      <p class="text-xs text-muted">
        {{
          t(
            plan.action === "uninstall"
              ? "store.actions.removesData"
              : plan.action === "install"
                ? "store.actions.installs"
                : "store.actions.preservesData",
          )
        }}
      </p>
      <StudioCallout
        v-if="
          plan.action !== 'uninstall' &&
          (plan.appsync === 'unknown' || plan.jailbreak === 'unknown')
        "
        tone="warning"
      >
        {{ t("store.actions.unknownRequirements") }}
      </StudioCallout>
      <p v-if="plan.action !== 'uninstall'" class="text-2xs text-muted">
        {{
          t("store.actions.catalogValid", {
            time: d(plan.catalogExpiresAt, "date"),
          })
        }}
      </p>
      <label
        v-if="plan.action === 'uninstall'"
        class="flex items-start gap-2.5 rounded-control bg-danger/8 px-3 py-2 text-xs leading-[18px]"
        ><input
          v-model="deleteData"
          type="checkbox"
          class="mt-0.5 accent-danger"
        />{{ t("store.actions.deleteConsent") }}</label
      >
      <StudioCallout v-if="changed" tone="warning">
        {{ t("store.actions.deviceChanged") }}
      </StudioCallout>
    </div>
    <StudioCallout v-if="store.planIssue.value" tone="danger" class="mt-3">
      {{ issue }}
    </StudioCallout>
    <div class="mt-4 flex justify-end gap-2">
      <StudioButton
        :disabled="store.submitting.value"
        @click="store.closePlan()"
      >
        {{ t("common.cancel") }}
      </StudioButton>
      <StudioButton
        v-if="plan"
        variant="primary"
        :loading="store.submitting.value"
        :disabled="changed || (plan.action === 'uninstall' && !deleteData)"
        @click="store.confirmPlan(deleteData)"
      >
        {{ t("store.actions.confirm") }}
      </StudioButton>
    </div>
  </StudioDialog>
</template>
