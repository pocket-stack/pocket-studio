<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import StudioDialog from "../../shared/ui/StudioDialog.vue";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioSegmented from "../../shared/ui/StudioSegmented.vue";
import { installationForms } from "./compatibility";
import { packageText } from "./packageContent";
import { useStore } from "./useStore";
import { useThreeDsSetup } from "../device/useThreeDsSetup";

/**
 * A 3DS title can run inside the Pocket launcher or as its own native app.
 * This is the one decision the user has to make before a 3DS install; the
 * plan itself is resolved and started without a further confirmation.
 */
const store = useStore();
const { t, locale } = useI18n();
const choice = ref<"shared" | "bundled">("shared");
const format = ref("cia");
const forms = computed(() => installationForms(store.deliveryEntry.value));
const hasShared = computed(() =>
  forms.value.some((form) => form.delivery === "shared"),
);
const bundledFormats = computed(() =>
  forms.value
    .filter((form) => form.delivery === "bundled")
    .map((form) => form.format),
);
watch(
  () => store.deliveryAppId.value,
  () => {
    choice.value = hasShared.value ? "shared" : "bundled";
    format.value = bundledFormats.value[0] ?? "cia";
  },
);
const name = computed(() =>
  store.deliveryEntry.value
    ? packageText(store.deliveryEntry.value, "name", locale.value, t)
    : t("threeDs.chooseDelivery"),
);
const candidate = computed(() =>
  forms.value.find(
    (form) =>
      form.delivery === choice.value &&
      (choice.value === "shared" || form.format === format.value),
  ),
);
const alreadyInstalled = computed(() =>
  store.installed.value.some(
    (record) =>
      record.packageId === store.deliveryAppId.value &&
      record.managed?.delivery === choice.value &&
      (choice.value === "shared" || record.managed.format === format.value),
  ),
);
function prepare(): void {
  const required = candidate.value?.runtimeRequirement ?? null;
  const abi = candidate.value?.hostAbi ?? null;
  store.closeDelivery();
  useThreeDsSetup().show(required, abi);
}
</script>
<template>
  <StudioDialog
    :open="!!store.deliveryAppId.value"
    :title="name"
    compact
    @close="store.closeDelivery"
  >
    <p class="text-xs leading-5 text-muted">{{ t("threeDs.deliveryHelp") }}</p>
    <div
      class="mt-3 grid grid-cols-2 gap-2"
      role="radiogroup"
      :aria-label="t('threeDs.chooseDelivery')"
    >
      <button
        v-for="value in ['shared', 'bundled'] as const"
        :key="value"
        type="button"
        role="radio"
        :aria-checked="choice === value"
        :disabled="value === 'shared' ? !hasShared : !bundledFormats.length"
        class="flex flex-col gap-1 rounded-control bg-ink/4 px-3 py-2.5 text-left text-sm transition-[background-color,box-shadow] duration-150 hover:bg-ink/8 disabled:opacity-40 aria-checked:bg-raised aria-checked:shadow-control"
        @click="choice = value"
      >
        <span class="font-medium">{{ t(`threeDs.delivery.${value}`) }}</span>
        <span class="text-2xs leading-4 text-muted">{{
          t(`threeDs.deliveryHint.${value}`)
        }}</span>
      </button>
    </div>
    <div
      v-if="choice === 'bundled' && bundledFormats.length > 1"
      class="mt-3 flex items-center gap-3 text-xs"
    >
      <span class="text-muted">{{ t("threeDs.nativeFormat") }}</span>
      <StudioSegmented
        v-model="format"
        size="sm"
        :label="t('threeDs.nativeFormat')"
        :options="
          bundledFormats.map((value) => ({
            value,
            label: value.toUpperCase(),
          }))
        "
      />
    </div>
    <p v-if="candidate" class="mt-3 text-xs text-muted">
      {{ candidate.version }} · r{{ candidate.revision }} ·
      {{ candidate.format.toUpperCase() }}
    </p>
    <StudioCallout v-if="alreadyInstalled" tone="neutral" class="mt-3">{{
      t("threeDs.alreadyInstalled")
    }}</StudioCallout>
    <StudioCallout
      v-else-if="!candidate || candidate.verdict !== 'compatible'"
      tone="warning"
      class="mt-3"
    >
      {{
        candidate
          ? t(`store.verdict.${candidate.verdict}`)
          : t("threeDs.noCandidate")
      }}
      <p v-if="candidate?.runtimeRequirement" class="mt-1">
        {{
          t("threeDs.hostRequirement", {
            version: candidate.runtimeRequirement.min_version,
            abi: candidate.hostAbi,
          })
        }}
      </p>
      <ol
        v-if="candidate?.verdict === 'requiresPreparation'"
        class="mt-2 list-inside list-decimal space-y-1 text-xs"
      >
        <li>
          {{
            t("threeDs.prepareRuntime", {
              app: candidate.runtimeRequirement?.bootstrap_app_id ?? "Pocket",
            })
          }}
        </li>
        <li>{{ t("threeDs.verifyPrepared") }}</li>
        <li>{{ t("threeDs.planAgain") }}</li>
      </ol>
      <StudioButton
        v-if="candidate?.verdict === 'requiresPreparation'"
        variant="link"
        @click="prepare"
        >{{ t("threeDs.setupTitle") }}</StudioButton
      >
    </StudioCallout>
    <div class="mt-4 flex justify-end gap-2">
      <StudioButton @click="store.closeDelivery">{{
        t("common.cancel")
      }}</StudioButton>
      <StudioButton
        variant="primary"
        :disabled="alreadyInstalled || candidate?.verdict !== 'compatible'"
        @click="store.chooseDelivery(choice, candidate!.format)"
        >{{ t("store.detail.install") }}</StudioButton
      >
    </div>
  </StudioDialog>
</template>
