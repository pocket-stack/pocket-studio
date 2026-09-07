<script setup lang="ts">
import IpodTouch from "../../assets/illustrations/ipod-touch.svg?component";
/**
 * Stylised iPod touch (4th generation): power button on the top edge,
 * Home button under the screen. Used for the empty state and DFU guide.
 */
const props = defineProps<{
  pressHome?: boolean;
  pressPower?: boolean;
  screen?: "off" | "home" | "apple" | "recovery";
  cable?: boolean;
  width?: number;
}>();
</script>

<template>
  <IpodTouch
    aria-hidden="true"
    class="inline-block h-auto select-none w-(--device-width) [--device-body:color-mix(in_srgb,var(--color-ink)_85%,transparent)] dark:[--device-body:var(--color-raised)] motion-safe:[--device-pulse:var(--animate-device-pulse)]"
    :style="{
      '--device-width': `${width ?? 180}px`,
      '--device-cable': cable ? 'inline' : 'none',
      '--device-power-color': pressPower
        ? 'var(--color-signal)'
        : 'var(--color-muted)',
      '--device-home-color': pressHome
        ? 'var(--color-signal)'
        : 'var(--color-muted)',
      '--device-power-pulse': pressPower ? 'inline' : 'none',
      '--device-home-pulse': pressHome ? 'inline' : 'none',
      '--device-screen-home': props.screen === 'home' ? 'inline' : 'none',
      '--device-screen-apple': props.screen === 'apple' ? 'inline' : 'none',
      '--device-screen-recovery':
        props.screen === 'recovery' ? 'inline' : 'none',
    }"
  />
</template>
