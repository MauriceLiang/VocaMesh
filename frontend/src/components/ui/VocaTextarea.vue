<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string;
    placeholder?: string;
    rows?: number;
    ariaLabel?: string;
    disabled?: boolean;
  }>(),
  { placeholder: "", rows: 3, ariaLabel: "", disabled: false },
);

defineEmits<{ (event: "update:modelValue", value: string): void }>();
</script>

<template>
  <textarea
    class="v-textarea"
    :value="modelValue"
    :rows="rows"
    :placeholder="placeholder"
    :aria-label="ariaLabel || placeholder"
    :disabled="disabled"
    @input="$emit('update:modelValue', ($event.target as HTMLTextAreaElement).value)"
  />
</template>
