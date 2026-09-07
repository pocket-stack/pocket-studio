<script setup lang="ts">
import { ref } from "vue";

defineProps<{ packageId: string; size?: number }>();
const failedPackageId = ref<string>();
</script>

<template>
  <img
    :src="`/vectors/packages/${failedPackageId === packageId ? 'pocket-runtime' : packageId}.svg`"
    :width="size ?? 80"
    :height="size ?? 80"
    alt=""
    class="inline-block shrink-0"
    @error="failedPackageId = packageId"
  />
</template>
