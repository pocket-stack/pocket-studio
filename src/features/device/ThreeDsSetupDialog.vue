<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useI18n } from "vue-i18n";
import {
  useGateway,
  GatewayError,
  type SetupPlan,
  type SetupResult,
} from "../../shared/gateway";
import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import StudioDialog from "../../shared/ui/StudioDialog.vue";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioInput from "../../shared/ui/StudioInput.vue";
import StudioSelect from "../../shared/ui/StudioSelect.vue";
import { useThreeDsSetup } from "./useThreeDsSetup";
const setup = useThreeDsSetup();
const session = useDeviceSession();
const gateway = useGateway();
const { t, te } = useI18n();
const method = ref("sd");
const path = ref("");
const address = ref("");
const port = ref("5000");
const username = ref("anonymous");
const password = ref("");
const format = ref("cia");
const busy = ref(false);
const plan = ref<SetupPlan | null>(null);
const result = ref<SetupResult | null>(null);
const issue = ref<string | null>(null);
watch(
  () => setup.requirement.value,
  () => {
    if (!busy.value) {
      plan.value = null;
      result.value = null;
      issue.value = null;
    }
  },
);
const error = computed(() => {
  const key = `threeDs.errors.${issue.value}`;
  if (te(key)) return t(key);
  const storeKey = `store.sourceErrors.${issue.value}`;
  if (te(storeKey)) return t("threeDs.storeFailure", { reason: t(storeKey) });
  return t("threeDs.errors.unknown");
});
const canReview = computed(() =>
  method.value === "sd"
    ? !!path.value.trim()
    : !!address.value.trim() && !!port.value.trim(),
);
const addressLabel = computed(() =>
  t(method.value === "ftp" ? "threeDs.ftpAddress" : "threeDs.address"),
);
async function review() {
  busy.value = true;
  issue.value = null;
  try {
    plan.value = await gateway.setup.plan({
      runtimeRequirement: setup.requirement.value,
      hostAbi: setup.hostAbi.value,
      destination:
        method.value === "sd"
          ? { kind: "sd", path: path.value }
          : {
              kind: "ftp",
              address: address.value,
              port: Number(port.value),
              username: username.value,
              password: password.value,
            },
      address: address.value || undefined,
      format:
        format.value === "pair" ? undefined : (format.value as "cia" | "3dsx"),
    });
    password.value = "";
  } catch (e) {
    issue.value = e instanceof GatewayError ? e.code : "unknown";
  } finally {
    busy.value = false;
  }
}
async function execute() {
  if (!plan.value) return;
  busy.value = true;
  issue.value = null;
  try {
    result.value = await gateway.setup.execute(plan.value.id);
    plan.value = null;
  } catch (e) {
    issue.value = e instanceof GatewayError ? e.code : "unknown";
    plan.value = null;
  } finally {
    busy.value = false;
  }
}
async function connect() {
  const pairingId =
    result.value?.pairingId ??
    (session.device.value?.platform === "3ds" ? session.device.value.id : null);
  if (!pairingId) return;
  busy.value = true;
  issue.value = null;
  try {
    await gateway.setup.connect(pairingId, address.value || undefined);
    await useDeviceSession().refresh();
    setup.close();
    result.value = null;
  } catch (e) {
    issue.value = e instanceof GatewayError ? e.code : "unknown";
  } finally {
    busy.value = false;
  }
}
function close() {
  if (!busy.value) setup.close();
}
</script>
<template>
  <StudioDialog
    :open="setup.open.value"
    :title="t('threeDs.setupTitle')"
    @close="close"
  >
    <div class="min-h-0 overflow-y-auto pr-1 text-sm">
      <StudioCallout tone="neutral">{{
        t("threeDs.hardwareScope")
      }}</StudioCallout>
      <p class="mt-3 text-xs leading-5 text-muted">
        {{ t("threeDs.cfwGuide") }}
        <a
          href="https://3ds.hacks.guide/checking-for-cfw.html"
          target="_blank"
          rel="noopener noreferrer"
          class="text-signal underline"
          >{{ t("threeDs.checkCfw") }}</a
        >
        ·
        <a
          href="https://3ds.hacks.guide/"
          target="_blank"
          rel="noopener noreferrer"
          class="text-signal underline"
          >{{ t("threeDs.guide") }}</a
        >
      </p>
      <template v-if="!plan && !result">
        <div class="mt-4 grid grid-cols-2 gap-3">
          <label class="flex flex-col gap-1 text-xs">
            {{ t("threeDs.transport") }}
            <StudioSelect
              v-model="method"
              :label="t('threeDs.transport')"
              :disabled="busy"
            >
              <option value="sd">{{ t("threeDs.sd") }}</option>
              <option value="ftp">{{ t("threeDs.ftp") }}</option>
            </StudioSelect>
          </label>
          <label class="flex flex-col gap-1 text-xs">
            {{ t("threeDs.bootstrap") }}
            <StudioSelect
              v-model="format"
              :label="t('threeDs.bootstrap')"
              :disabled="busy"
            >
              <option value="cia">{{ t("threeDs.bootstrapCia") }}</option>
              <option value="3dsx">{{ t("threeDs.bootstrap3dsx") }}</option>
              <option value="pair">{{ t("threeDs.pairOnly") }}</option>
            </StudioSelect>
          </label>
          <p class="col-span-2 text-xs leading-5 text-muted" role="status">
            {{ t(`threeDs.formatHelp.${format}`) }}
          </p>
          <div
            v-if="method === 'sd'"
            class="col-span-2 flex flex-col gap-1 text-xs"
          >
            <span>{{ t("threeDs.cardPath") }}</span>
            <StudioInput
              v-model="path"
              :label="t('threeDs.cardPath')"
              :placeholder="t('threeDs.cardPlaceholder')"
              :disabled="busy"
            />
          </div>
          <div class="flex flex-col gap-1 text-xs">
            <span>{{ addressLabel }}</span>
            <StudioInput
              v-model="address"
              :label="addressLabel"
              :placeholder="
                t(
                  method === 'ftp'
                    ? 'threeDs.ipPlaceholder'
                    : 'threeDs.optionalIpPlaceholder',
                )
              "
              :disabled="busy"
            />
          </div>
          <template v-if="method === 'ftp'">
            <div class="flex flex-col gap-1 text-xs">
              <span>{{ t("threeDs.port") }}</span>
              <StudioInput
                v-model="port"
                :label="t('threeDs.port')"
                inputmode="numeric"
                :disabled="busy"
              />
            </div>
            <div class="flex flex-col gap-1 text-xs">
              <span>{{ t("threeDs.username") }}</span>
              <StudioInput
                v-model="username"
                :label="t('threeDs.username')"
                autocomplete="off"
                :disabled="busy"
              />
            </div>
            <div class="flex flex-col gap-1 text-xs">
              <span>{{ t("threeDs.password") }}</span>
              <StudioInput
                v-model="password"
                :label="t('threeDs.password')"
                type="password"
                autocomplete="off"
                :disabled="busy"
              />
            </div>
          </template>
        </div>
        <p class="mt-3 text-xs leading-5 text-muted">
          {{ t(method === "ftp" ? "threeDs.ftpHelp" : "threeDs.sdHelp") }}
        </p>
      </template>
      <div v-else-if="plan" class="mt-4 space-y-3">
        <p class="break-all font-medium">{{ plan.destination }}</p>
        <p v-if="plan.artifact" class="break-all text-xs text-muted">
          {{ plan.bootstrapAppId }}
        </p>
        <p>
          {{
            t(plan.existingPairing ? "threeDs.reusePair" : "threeDs.newPair")
          }}
        </p>
        <ul class="space-y-1 rounded-control bg-ink/4 p-3 text-xs">
          <li
            v-for="file in plan.files"
            :key="file"
            class="break-all font-mono"
          >
            {{ file }}
          </li>
        </ul>
        <p v-if="plan.artifact" class="text-xs">
          {{ t("threeDs.launcherVersion") }} {{ plan.version }} ·
          {{ plan.artifact.format.toUpperCase() }}
        </p>
        <p v-if="plan.artifact" class="break-all font-mono text-2xs text-muted">
          SHA-256 {{ plan.artifact.blob.sha256 }}
        </p>
        <StudioCallout tone="warning">{{
          t("threeDs.setupConfirm")
        }}</StudioCallout>
      </div>
      <div v-else-if="result" class="mt-4 space-y-3">
        <StudioCallout tone="success">{{
          t("threeDs.filesVerified")
        }}</StudioCallout>
        <p class="text-sm leading-6">
          {{ t(format === "cia" ? "threeDs.fbiNext" : "threeDs.restartNext") }}
        </p>
        <label class="flex flex-col gap-1 text-xs"
          >{{ t("threeDs.address") }}<StudioInput v-model="address"
        /></label>
      </div>
      <StudioCallout v-if="issue" tone="danger" class="mt-3">
        {{ error }}
        <span class="mt-1 block text-2xs text-muted">{{
          t("threeDs.errorCode", { code: issue })
        }}</span>
      </StudioCallout>
    </div>
    <footer class="mt-4 flex shrink-0 justify-end gap-2">
      <StudioButton :disabled="busy" @click="close">{{
        t("common.close")
      }}</StudioButton>
      <StudioButton
        v-if="!plan && !result && session.device.value?.platform === '3ds'"
        :disabled="busy"
        @click="connect"
        >{{ t("threeDs.verifyExisting") }}</StudioButton
      >
      <StudioButton v-if="plan" :disabled="busy" @click="plan = null">{{
        t("threeDs.editSetup")
      }}</StudioButton>
      <StudioButton
        variant="primary"
        :loading="busy"
        :disabled="!plan && !result && !canReview"
        @click="result ? connect() : plan ? execute() : review()"
        >{{
          t(
            result
              ? "threeDs.connect"
              : plan
                ? "threeDs.writeFiles"
                : "threeDs.reviewSetup",
          )
        }}</StudioButton
      >
    </footer>
  </StudioDialog>
</template>
