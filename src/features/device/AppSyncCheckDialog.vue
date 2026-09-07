<script setup lang="ts">
import { ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import { GatewayError, useGateway } from "../../shared/gateway";
import StudioDialog from "../../shared/ui/StudioDialog.vue";

const props = defineProps<{ open: boolean; deviceId: string }>();
const emit = defineEmits<{ close: []; checked: [] }>();
const { t } = useI18n();
const password = ref("");
const pending = ref(false);
const error = ref<string | null>(null);
let generation = 0;
watch(
  () => [props.open, props.deviceId],
  () => {
    generation += 1;
    password.value = props.open ? "alpine" : "";
    pending.value = false;
    error.value = null;
  },
  { immediate: true },
);
async function check(): Promise<void> {
  if (!password.value || pending.value) return;
  const request = generation;
  const secret = password.value;
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
    <form class="space-y-4" @submit.prevent="check">
      <p class="text-sm leading-6 text-muted">
        {{ t("readiness.appSyncCheck.body") }}
      </p>
      <label class="block text-sm">
        {{ t("preparation.appSync.password") }}
        <input
          :value="password"
          :disabled="pending"
          type="password"
          autocomplete="off"
          maxlength="1024"
          class="mt-2 w-full rounded border border-line bg-canvas px-3 py-2"
          @input="password = ($event.target as HTMLInputElement).value"
        />
      </label>
      <p class="text-xs leading-5 text-muted">
        {{ t("preparation.appSync.passwordHint") }}
      </p>
      <p v-if="error" role="alert" class="text-sm text-danger">
        {{
          t(
            `readiness.appSyncCheck.errors.${error}`,
            t("readiness.appSyncCheck.errors.appSyncCheckUnavailable"),
          )
        }}
      </p>
      <div class="flex justify-end gap-3">
        <button
          type="button"
          class="rounded border border-line px-3 py-2 text-sm"
          @click="emit('close')"
        >
          {{ t("common.close") }}
        </button>
        <button
          type="submit"
          :disabled="pending || !password"
          class="rounded bg-signal px-3 py-2 text-sm text-on-signal disabled:opacity-45"
        >
          {{
            t(
              pending
                ? "readiness.appSyncCheck.checking"
                : "readiness.appSyncCheck.action",
            )
          }}
        </button>
      </div>
    </form>
  </StudioDialog>
</template>
