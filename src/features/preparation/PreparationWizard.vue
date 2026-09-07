<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import RiskStep from "./steps/RiskStep.vue";
import DisclaimerStep from "./steps/DisclaimerStep.vue";
import { useGateway } from "../../shared/gateway";
import { usePreparation } from "./usePreparation";
const { t } = useI18n();
const preparation = usePreparation();
const dialog = ref<HTMLDialogElement | null>(null);
let previousFocus: HTMLElement | null = null;
watch(preparation.consentVisible, async (visible) => {
  if (visible) {
    previousFocus = document.activeElement as HTMLElement;
    await nextTick();
    dialog.value?.showModal();
  } else {
    dialog.value?.close();
    await nextTick();
    previousFocus?.focus();
  }
});
</script>
<template>
  <dialog
    ref="dialog"
    class="m-auto h-[min(740px,calc(100dvh-70px))] max-h-none w-[640px] max-w-[calc(100vw-36px)] rounded-lg border border-[#b5b5b5] bg-canvas p-0 text-ink shadow-[0_10px_45px_#00000030] backdrop:bg-[#0000002b]"
    :aria-label="
      t(
        preparation.stage.value === 'risks'
          ? 'preparation.risks.banner.title'
          : 'preparation.disclaimer.title',
      )
    "
    @cancel.prevent="preparation.close"
  >
    <div
      v-if="preparation.consentVisible.value && preparation.plan.value"
      class="flex h-full flex-col gap-4 p-6"
    >
      <header class="flex items-center gap-3">
        <span class="text-[12px] whitespace-nowrap text-muted">{{
          t("studio.consentStep", {
            step: preparation.stage.value === "risks" ? 1 : 2,
          })
        }}</span>
        <h2 class="text-[18px] font-semibold">
          {{
            t(
              preparation.stage.value === "risks"
                ? "preparation.risks.banner.title"
                : "preparation.disclaimer.title",
            )
          }}
        </h2>
        <button
          class="ml-auto inline-flex items-center justify-center rounded p-[5px] text-muted hover:bg-track hover:text-ink"
          :aria-label="t('common.close')"
          @click="preparation.close"
        >
          <IconStudioCross width="17" height="17" />
        </button>
      </header>
      <RiskStep
        v-if="preparation.stage.value === 'risks'"
        :plan="preparation.plan.value"
        @next="preparation.acknowledgeRisks"
        @back="preparation.close"
      /><DisclaimerStep
        v-else
        :plan="preparation.plan.value"
        :start-error="preparation.startError.value"
        @accept="preparation.acceptDisclaimer"
        @back="preparation.backToRisks"
      />
      <p
        v-if="useGateway().capabilities.demo"
        class="text-[10px] leading-[1.7] text-muted"
      >
        {{ t("studio.consentDemo") }}
      </p>
    </div>
  </dialog>
</template>
