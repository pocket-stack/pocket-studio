<script setup lang="ts">
import { computed } from "vue";
import AppIcon from "../../shared/ui/AppIcon.vue";
const props = defineProps<{ packageId: string; size?: number }>();
const art: Record<string, { glyph: string; color: string; light: string }> = {
  "tunnel-kit": { glyph: "layers", color: "#7b99b8", light: "#9fbde6" },
  "sensor-tap": { glyph: "bolt", color: "#668478", light: "#90ab9f" },
  "batt-guard": { glyph: "battery", color: "#879da8", light: "#b4c4cc" },
  "clip-sync": { glyph: "copy", color: "#718c9a", light: "#9eb4c1" },
  "photo-pull": { glyph: "camera", color: "#658194", light: "#99aebc" },
  "font-patch": { glyph: "note", color: "#8a99a2", light: "#b8c2c9" },
  "pocket-runtime": { glyph: "pocket", color: "#3f72a7", light: "#6ca1d6" },
  "pocket-agent": { glyph: "layers", color: "#567d78", light: "#8db3a9" },
  openssh: { glyph: "terminal", color: "#38485b", light: "#72849c" },
  "legacy-ca-bundle": { glyph: "shield", color: "#8b794f", light: "#c1ad77" },
  "pocket-reader": { glyph: "book", color: "#b47749", light: "#e3b179" },
  "pocket-notes": { glyph: "note", color: "#658494", light: "#a0bcc6" },
  "pocket-arcade": { glyph: "game", color: "#6f6685", light: "#a49cb5" },
  "pocket-camera-pro": { glyph: "camera", color: "#65736d", light: "#a6b2a9" },
};
const selected = computed(() => art[props.packageId] ?? art["pocket-runtime"]!);
</script>
<template>
  <span
    class="relative inline-flex size-(--art-size) shrink-0 items-center justify-center overflow-hidden rounded-[22%] bg-(--art-light) text-white shadow-none"
    :style="{ '--art-light': selected.light, '--art-size': `${size ?? 80}px` }"
    ><span class="hidden" /><AppIcon
      class="relative opacity-85 filter-none"
      :name="selected.glyph"
      :size="(size ?? 80) * 0.46"
  /></span>
</template>
