<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
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
      class="py-5 text-sm text-muted"
    >
      {{ t("store.actions.planning") }}
    </p>
    <div v-if="plan" class="space-y-5 text-sm">
      <div>
        <p class="text-xs text-muted">
          {{ t(`store.actions.kind.${plan.action}`) }}
        </p>
        <h3 class="mt-1 text-xl font-semibold">{{ name }}</h3>
      </div>
      <dl
        class="grid grid-cols-[auto_1fr] gap-x-5 gap-y-3 rounded-lg border border-line bg-surface p-4 text-xs"
      >
        <dt class="text-muted">{{ t("store.actions.device") }}</dt>
        <dd class="text-right break-words">{{ plan.deviceName }}</dd>
        <dt class="text-muted">{{ t("store.actions.nativeId") }}</dt>
        <dd class="text-right break-all font-mono">{{ plan.bundleId }}</dd>
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
            {{ plan.version }} · r{{ plan.revision }}<br />{{
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
      <p class="text-xs leading-6 text-muted">
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
      <p
        v-if="
          plan.action !== 'uninstall' &&
          (plan.appsync === 'unknown' || plan.jailbreak === 'unknown')
        "
        class="rounded-md border border-warning/30 bg-warning/5 p-3 text-xs leading-6 text-warning"
      >
        {{ t("store.actions.unknownRequirements") }}
      </p>
      <p v-if="plan.action !== 'uninstall'" class="text-xs text-muted">
        {{
          t("store.actions.catalogValid", {
            time: d(plan.catalogExpiresAt, "date"),
          })
        }}
      </p>
      <label
        v-if="plan.action === 'uninstall'"
        class="flex items-start gap-3 rounded-md border border-danger/30 p-3 text-xs leading-6"
        ><input
          v-model="deleteData"
          type="checkbox"
          class="mt-1 accent-danger"
        />{{ t("store.actions.deleteConsent") }}</label
      >
      <p v-if="changed" class="text-xs text-warning">
        {{ t("store.actions.deviceChanged") }}
      </p>
    </div>
    <p
      v-if="store.planIssue.value"
      role="alert"
      class="mt-4 text-xs leading-6 text-danger"
    >
      {{ issue }}
    </p>
    <div class="mt-6 flex justify-end gap-3 text-sm">
      <button
        class="rounded-md border border-line px-3 py-2 disabled:opacity-50"
        :disabled="store.submitting.value"
        @click="store.closePlan()"
      >
        {{ t("common.cancel") }}
      </button>
      <button
        v-if="plan"
        class="rounded-md bg-signal px-4 py-2 text-on-signal disabled:opacity-50"
        :disabled="
          store.submitting.value ||
          changed ||
          (plan.action === 'uninstall' && !deleteData)
        "
        @click="store.confirmPlan(deleteData)"
      >
        {{
          t(
            store.submitting.value
              ? "store.actions.starting"
              : "store.actions.confirm",
          )
        }}
      </button>
    </div>
  </StudioDialog>
</template>
