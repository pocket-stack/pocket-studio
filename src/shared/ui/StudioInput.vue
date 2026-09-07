<script setup lang="ts">
import { computed, ref, useAttrs } from "vue";
import { useI18n } from "vue-i18n";

defineOptions({ inheritAttrs: false });
const model = defineModel<string>({ default: "" });
const props = withDefaults(
  defineProps<{
    type?: "text" | "search" | "password";
    placeholder?: string;
    label?: string;
    disabled?: boolean;
    size?: "sm" | "md";
    /** Value used when the field is left empty; always shown as plain grey text. */
    secret?: string;
  }>(),
  { type: "text", size: "md", placeholder: "", label: "", secret: "" },
);
const { t } = useI18n();
const attrs = useAttrs();
const rootClass = computed(() => attrs.class);
const inputAttrs = computed(() =>
  Object.fromEntries(Object.entries(attrs).filter(([key]) => key !== "class")),
);
const revealed = ref(false);
const inputType = computed(() =>
  props.type === "password" && revealed.value ? "text" : props.type,
);
// The default stays readable so the user knows exactly what will be sent;
// the reveal toggle only affects what they typed.
const hint = computed(() => props.secret || props.placeholder);
</script>

<template>
  <label
    class="flex items-center gap-1.5 rounded-control bg-track/70 px-2.5 text-ink shadow-inset transition-[background-color,box-shadow] focus-within:bg-raised focus-within:shadow-[0_0_0_2px_var(--color-signal)] has-[input:disabled]:opacity-45"
    :class="[size === 'sm' ? 'h-6 text-xs' : 'h-7 text-sm', rootClass]"
  >
    <span v-if="$slots.icon" class="flex shrink-0 text-muted"
      ><slot name="icon"
    /></span>
    <input
      v-bind="inputAttrs"
      :value="model"
      :type="inputType"
      :placeholder="hint"
      :aria-label="label"
      :disabled="disabled"
      class="min-w-0 flex-1 bg-transparent outline-none placeholder:text-muted"
      @input="model = ($event.target as HTMLInputElement).value"
    />
    <button
      v-if="type === 'password'"
      type="button"
      class="-mr-1 flex shrink-0 rounded p-0.5 text-muted hover:text-ink"
      :aria-label="t(revealed ? 'common.hidePassword' : 'common.showPassword')"
      :aria-pressed="revealed"
      @click="revealed = !revealed"
    >
      <IconPhEyeSlash v-if="revealed" width="14" height="14" />
      <IconPhEye v-else width="14" height="14" />
    </button>
  </label>
</template>
