<script setup lang="ts">
import { useI18n } from "vue-i18n";

import { useDeviceSession } from "../../shared/composables/useDeviceSession";
import { useGateway } from "../../shared/gateway";
import DeviceIllustration from "../../shared/ui/DeviceIllustration.vue";
import AppIcon from "../../shared/ui/AppIcon.vue";
import { usePreparation } from "../preparation/usePreparation";
import DeviceInfoCard from "./DeviceInfoCard.vue";
import ReadinessPanel from "./ReadinessPanel.vue";

const emit = defineEmits<{ openStore: [] }>();
const { t } = useI18n();
const { device, readiness, checking, checkReadiness } = useDeviceSession();
const preparation = usePreparation();
const gateway = useGateway();

function startPreparation(): void {
  if (device.value) void preparation.open(device.value.id);
}
</script>

<template>
  <div class="mx-auto flex max-w-4xl flex-col gap-5">
    <header>
      <h1 class="text-2xl font-semibold tracking-tight">
        {{ t("device.title") }}
      </h1>
      <p class="mt-1 text-sm text-muted">{{ t("device.subtitle") }}</p>
    </header>

    <section
      v-if="!device"
      class="card rise flex flex-col items-center gap-6 px-8 py-12 text-center md:flex-row md:text-left"
    >
      <DeviceIllustration :width="140" screen="home" cable class="opacity-90" />
      <div class="flex-1">
        <h2 class="text-lg font-semibold">{{ t("device.empty.title") }}</h2>
        <p class="mt-2 text-sm text-muted">{{ t("device.empty.body") }}</p>
        <ol class="mt-4 space-y-2 text-sm">
          <li class="flex gap-3">
            <span class="font-mono text-muted">1</span
            >{{ t("device.empty.step1") }}
          </li>
          <li class="flex gap-3">
            <span class="font-mono text-muted">2</span
            >{{ t("device.empty.step2") }}
          </li>
          <li class="flex gap-3">
            <span class="font-mono text-muted">3</span
            >{{ t("device.empty.step3") }}
          </li>
        </ol>
        <div class="mt-5 flex items-center gap-3">
          <span class="pulse flex items-center gap-2 text-xs text-muted">
            <AppIcon name="usb" :size="14" />
            {{ t("device.empty.listening") }}
          </span>
          <button
            class="btn btn-secondary text-xs"
            @click="gateway.demo.attachDevice()"
          >
            <AppIcon name="lab" :size="14" />
            {{ t("demo.attach") }}
          </button>
        </div>
      </div>
    </section>

    <template v-else>
      <DeviceInfoCard class="rise" :device="device" />
      <ReadinessPanel
        class="rise"
        :report="readiness"
        :checking="checking"
        @recheck="checkReadiness"
        @prepare="startPreparation"
      />
      <section
        v-if="readiness?.status === 'ready'"
        class="card rise flex items-center justify-between gap-4 p-5"
      >
        <div>
          <h3 class="text-base font-semibold">{{ t("device.next.title") }}</h3>
          <p class="mt-1 text-sm text-muted">{{ t("device.next.body") }}</p>
        </div>
        <button class="btn btn-primary" @click="emit('openStore')">
          <AppIcon name="store" :size="16" />
          {{ t("device.next.action") }}
        </button>
      </section>
    </template>
  </div>
</template>
