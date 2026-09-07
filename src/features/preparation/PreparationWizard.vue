<script setup lang="ts">
import { nextTick, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import RiskStep from "./steps/RiskStep.vue";
import DisclaimerStep from "./steps/DisclaimerStep.vue";
import { useGateway } from "../../shared/gateway";
import { focusInitial } from "../../shared/ui/dialogFocus";
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
    focusInitial(dialog.value);
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
    class="m-auto h-[min(600px,calc(100dvh-60px))] max-h-none w-[640px] max-w-[calc(100vw-40px)] flex-col overflow-hidden rounded-panel outline-none bg-surface p-0 text-ink shadow-overlay backdrop:bg-[#0b1220]/35 backdrop:backdrop-blur-[2px] open:flex"
    :aria-label="
      t(
        preparation.stage.value === 'risks'
          ? 'preparation.risks.banner.title'
          : 'preparation.disclaimer.title',
      )
    "
    tabindex="-1"
    @cancel.prevent="preparation.close"
  >
    <div
      v-if="preparation.consentVisible.value && preparation.plan.value"
      class="flex min-h-0 flex-1 flex-col gap-3 p-5"
    >
      <header class="flex items-center gap-3">
        <span
          class="rounded-full bg-ink/6 px-2 py-px text-2xs font-semibold whitespace-nowrap text-muted"
          >{{
            t("studio.consentStep", {
              step: preparation.stage.value === "risks" ? 1 : 2,
            })
          }}</span
        >
        <h2 class="text-lg font-semibold">
          {{
            t(
              preparation.stage.value === "risks"
                ? "preparation.risks.banner.title"
                : "preparation.disclaimer.title",
            )
          }}
        </h2>
        <button
          class="ml-auto -mr-1.5 inline-flex items-center justify-center rounded-control p-1 text-muted hover:bg-ink/6 hover:text-ink"
          :aria-label="t('common.close')"
          @click="preparation.close"
        >
          <IconPhX width="16" height="16" />
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
      <p v-if="useGateway().capabilities.demo" class="text-2xs text-muted">
        {{ t("studio.consentDemo") }}
      </p>
    </div>
  </dialog>
</template>
