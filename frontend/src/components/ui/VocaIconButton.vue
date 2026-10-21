<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string;
    disabled?: boolean;
  }>(),
  { disabled: false },
);
</script>

<template>
  <button class="v-icon-btn" :aria-label="label" :title="label" :disabled="disabled">
    <slot />
  </button>
</template>
