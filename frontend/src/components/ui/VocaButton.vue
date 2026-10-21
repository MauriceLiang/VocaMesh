<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger";
    size?: "md" | "sm";
    disabled?: boolean;
    loading?: boolean;
    type?: "button" | "submit";
  }>(),
  { variant: "secondary", size: "md", disabled: false, loading: false, type: "button" },
);

const classes = computed(() => [
  "v-btn",
  `is-${props.variant}`,
  props.size === "sm" ? "sm" : "",
]);
</script>

<template>
  <button :class="classes" :disabled="disabled || loading" :type="type">
    <span v-if="loading" class="v-spin" aria-hidden="true">
      <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.8">
        <path d="M12 3a9 9 0 1 0 9 9" stroke-linecap="round" />
      </svg>
    </span>
    <slot name="icon" />
    <slot />
  </button>
</template>
