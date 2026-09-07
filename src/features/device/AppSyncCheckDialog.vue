<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { GatewayError, useGateway } from "../../shared/gateway";
import { useDefaultSshPassword } from "../../shared/preferences/sshPassword";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioDialog from "../../shared/ui/StudioDialog.vue";
import StudioInput from "../../shared/ui/StudioInput.vue";

const props = defineProps<{ open: boolean; deviceId: string }>();
const emit = defineEmits<{ close: []; checked: [] }>();
const { t } = useI18n();
const { effective: defaultPassword } = useDefaultSshPassword();
const password = ref("");
const pending = ref(false);
const error = ref<string | null>(null);
let generation = 0;
watch(
  () => [props.open, props.deviceId],
  () => {
    generation += 1;
    password.value = "";
    pending.value = false;
    error.value = null;
  },
  { immediate: true },
);
async function check(): Promise<void> {
  const secret = password.value || defaultPassword.value;
  if (!secret || pending.value) return;
  const request = generation;
  password.value = "";
  pending.value = true;
  error.value = null;
  try {
    await useGateway().devices.checkAppSync(props.deviceId, secret);
    if (request !== generation) return;
    emit("checked");
    emit("close");
  } catch (failure) {
    if (request === generation)
      error.value =
        failure instanceof GatewayError
          ? failure.code
          : "appSyncCheckUnavailable";
  } finally {
    if (request === generation) pending.value = false;
  }
}
</script>
<template>
  <StudioDialog
    :open="open"
    :title="t('readiness.appSyncCheck.title')"
    compact
    @close="emit('close')"
  >
    <form class="flex flex-col gap-3" @submit.prevent="check">
      <p class="text-sm text-muted">{{ t("readiness.appSyncCheck.body") }}</p>
      <label class="block text-sm">
        <span class="mb-1 block font-medium">{{
          t("preparation.appSync.password")
        }}</span>
        <StudioInput
          v-model="password"
          type="password"
          class="w-full"
          autofocus
          autocomplete="off"
          :secret="defaultPassword"
          maxlength="1024"
          :disabled="pending"
          :label="t('preparation.appSync.password')"
        />
      </label>
      <p class="text-xs text-muted">
        {{ t("preparation.appSync.passwordHint") }}
      </p>
      <StudioCallout v-if="error" tone="danger">
        {{
          t(
            `readiness.appSyncCheck.errors.${error}`,
            t("readiness.appSyncCheck.errors.appSyncCheckUnavailable"),
          )
        }}
      </StudioCallout>
      <div class="mt-1 flex justify-end gap-2">
        <StudioButton @click="emit('close')">{{
          t("common.close")
        }}</StudioButton>
        <StudioButton
          type="submit"
          variant="primary"
          :loading="pending"
          :disabled="!password && !defaultPassword"
        >
          {{ t("readiness.appSyncCheck.action") }}
        </StudioButton>
      </div>
    </form>
  </StudioDialog>
</template>
