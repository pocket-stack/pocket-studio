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
import {
  DEFAULT_FTP_USERNAME,
  useThreeDsConnection,
} from "../../shared/preferences/threeDsConnection";
import StudioDialog from "../../shared/ui/StudioDialog.vue";
import StudioButton from "../../shared/ui/StudioButton.vue";
import StudioCallout from "../../shared/ui/StudioCallout.vue";
import StudioInput from "../../shared/ui/StudioInput.vue";
import StudioSegmented from "../../shared/ui/StudioSegmented.vue";
import { useDeviceConnect } from "./useDeviceConnect";

/**
 * Manual connection for devices discovery cannot see, and launcher preparation
 * for a known 3DS. The console's address and ftpd account come from
 * Preferences; the dialog only asks what cannot be remembered.
 */
const emit = defineEmits<{ openSettings: [] }>();
const connect = useDeviceConnect();
const session = useDeviceSession();
const gateway = useGateway();
const connection = useThreeDsConnection();
const { t, te } = useI18n();
const kind = ref<"3ds" | null>(null);
const transport = ref<"ftp" | "sd">("sd");
const path = ref("");
const format = ref<"cia" | "3dsx">("cia");
const busy = ref(false);
const plan = ref<SetupPlan | null>(null);
const result = ref<SetupResult | null>(null);
const issue = ref<string | null>(null);
const launcher = computed(() => connect.mode.value === "launcher");
const configured = computed(
  () => !!connection.address.value && connection.ftpPort.value !== null,
);
watch(
  () => connect.open.value,
  (open) => {
    if (!open) return;
    // The launcher flow already knows which console it prepares.
    kind.value = launcher.value ? "3ds" : null;
    transport.value = configured.value ? "ftp" : "sd";
    format.value = "cia";
    plan.value = null;
    result.value = null;
    issue.value = null;
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
  transport.value === "ftp" ? configured.value : !!path.value.trim(),
);
const transportOptions = computed(() => [
  { value: "ftp", label: t("threeDs.wireless") },
  { value: "sd", label: t("threeDs.sdReader") },
]);
const formatOptions = computed(() => [
  { value: "cia", label: t("threeDs.bootstrapCia") },
  { value: "3dsx", label: t("threeDs.bootstrap3dsx") },
]);
async function review() {
  busy.value = true;
  issue.value = null;
  try {
    plan.value = await gateway.setup.plan({
      runtimeRequirement: connect.requirement.value,
      hostAbi: connect.hostAbi.value,
      destination:
        transport.value === "sd"
          ? { kind: "sd", path: path.value.trim() }
          : {
              kind: "ftp",
              address: connection.address.value,
              port: connection.ftpPort.value!,
              username:
                connection.settings.value.username.trim() ||
                DEFAULT_FTP_USERNAME,
              password: connection.settings.value.password,
            },
      address: connection.address.value || undefined,
      format: launcher.value ? format.value : undefined,
    });
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
async function connectConsole() {
  const pairingId =
    result.value?.pairingId ??
    (session.device.value?.platform === "3ds" ? session.device.value.id : null);
  if (!pairingId) return;
  busy.value = true;
  issue.value = null;
  try {
    await gateway.setup.connect(
      pairingId,
      connection.address.value || undefined,
    );
    await session.refresh();
    connect.close();
    result.value = null;
  } catch (e) {
    issue.value = e instanceof GatewayError ? e.code : "unknown";
  } finally {
    busy.value = false;
  }
}
function close() {
  if (!busy.value) connect.close();
}
</script>
<template>
  <StudioDialog
    :open="connect.open.value"
    :title="t(launcher ? 'threeDs.launcherTitle' : 'device.connect.title')"
    @close="close"
  >
    <!-- Horizontal padding keeps focus rings inside the scroll container. -->
    <div class="-mx-1 min-h-0 overflow-y-auto px-1 text-sm">
      <template v-if="!kind">
        <p class="text-xs leading-5 text-muted">
          {{ t("device.connect.intro") }}
        </p>
        <button
          type="button"
          class="mt-3 flex w-full items-center gap-3 rounded-control bg-ink/4 px-3 py-2.5 text-left transition-colors hover:bg-ink/8"
          @click="kind = '3ds'"
        >
          <span
            class="grid size-9 shrink-0 place-items-center rounded-[10px] bg-raised text-signal shadow-control"
            ><IconPhGameController width="18" height="18"
          /></span>
          <span class="min-w-0 flex-1">
            <b class="block font-medium">{{
              t("device.connect.kinds.3ds.title")
            }}</b>
            <span class="block text-xs text-muted">{{
              t("device.connect.kinds.3ds.body")
            }}</span>
          </span>
          <IconPhCaretRight width="14" height="14" class="text-muted" />
        </button>
      </template>
      <template v-else-if="!plan && !result">
        <StudioCallout tone="neutral">{{
          t("threeDs.hardwareScope")
        }}</StudioCallout>
        <p class="mt-3 text-xs leading-5 text-muted">
          {{ t(launcher ? "threeDs.launcherIntro" : "threeDs.cfwGuide") }}
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
        <div class="mt-4 flex flex-col gap-3">
          <div class="flex items-center justify-between gap-3">
            <span class="text-xs text-muted">{{ t("threeDs.transport") }}</span>
            <StudioSegmented
              v-model="transport"
              size="sm"
              :label="t('threeDs.transport')"
              :options="transportOptions"
            />
          </div>
          <div v-if="launcher" class="flex items-center justify-between gap-3">
            <span class="text-xs text-muted">{{
              t("threeDs.launcherForm")
            }}</span>
            <StudioSegmented
              v-model="format"
              size="sm"
              :label="t('threeDs.launcherForm')"
              :options="formatOptions"
            />
          </div>
          <p v-if="launcher" class="text-xs leading-5 text-muted" role="status">
            {{ t(`threeDs.formatHelp.${format}`) }}
          </p>
          <template v-if="transport === 'ftp'">
            <div
              v-if="configured"
              class="flex items-center justify-between gap-3 rounded-control bg-ink/4 px-3 py-1.5"
            >
              <span class="min-w-0 truncate font-mono text-xs">{{
                t("threeDs.ftpSummary", {
                  address: connection.address.value,
                  port: connection.ftpPort.value,
                  username:
                    connection.settings.value.username.trim() ||
                    DEFAULT_FTP_USERNAME,
                })
              }}</span>
              <StudioButton
                variant="link"
                size="sm"
                @click="emit('openSettings')"
                >{{ t("threeDs.editInSettings") }}</StudioButton
              >
            </div>
            <StudioCallout v-else tone="warning">
              {{ t("threeDs.noAddress") }}
              <StudioButton
                variant="link"
                size="sm"
                class="ml-1"
                @click="emit('openSettings')"
                >{{ t("threeDs.openSettings") }}</StudioButton
              >
            </StudioCallout>
            <p class="text-xs leading-5 text-muted">
              {{ t("threeDs.ftpHelp") }}
            </p>
          </template>
          <template v-else>
            <label class="flex flex-col gap-1 text-xs">
              <span>{{ t("threeDs.cardPath") }}</span>
              <StudioInput
                v-model="path"
                :label="t('threeDs.cardPath')"
                :placeholder="t('threeDs.cardPlaceholder')"
                :disabled="busy"
              />
            </label>
            <p class="text-xs leading-5 text-muted">
              {{ t("threeDs.sdHelp") }}
            </p>
          </template>
        </div>
      </template>
      <div v-else-if="plan" class="space-y-3">
        <p class="font-medium break-all">{{ plan.destination }}</p>
        <p v-if="plan.artifact" class="text-xs break-all text-muted">
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
            class="font-mono break-all"
          >
            {{ file }}
          </li>
        </ul>
        <p v-if="plan.artifact" class="text-xs">
          {{ t("threeDs.launcherVersion") }} {{ plan.version }} ·
          {{ plan.artifact.format.toUpperCase() }}
        </p>
        <p v-if="plan.artifact" class="font-mono text-2xs break-all text-muted">
          SHA-256 {{ plan.artifact.blob.sha256 }}
        </p>
        <StudioCallout tone="warning">{{
          t("threeDs.setupConfirm")
        }}</StudioCallout>
      </div>
      <div v-else-if="result" class="space-y-3">
        <StudioCallout tone="success">{{
          t("threeDs.filesVerified")
        }}</StudioCallout>
        <p class="text-sm leading-6">
          {{
            t(
              launcher && format === "cia"
                ? "threeDs.fbiNext"
                : "threeDs.restartNext",
            )
          }}
        </p>
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
        v-if="
          kind &&
          !plan &&
          !result &&
          !launcher &&
          session.device.value?.platform === '3ds'
        "
        :disabled="busy"
        @click="connectConsole"
        >{{ t("threeDs.verifyExisting") }}</StudioButton
      >
      <StudioButton v-if="plan" :disabled="busy" @click="plan = null">{{
        t("threeDs.editSetup")
      }}</StudioButton>
      <StudioButton
        v-if="kind"
        variant="primary"
        :loading="busy"
        :disabled="!plan && !result && !canReview"
        @click="result ? connectConsole() : plan ? execute() : review()"
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
