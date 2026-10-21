<script setup lang="ts">
withDefaults(
  defineProps<{
    modelValue: string;
    placeholder?: string;
    type?: string;
    ariaLabel?: string;
    invalid?: boolean;
    disabled?: boolean;
  }>(),
  { placeholder: "", type: "text", ariaLabel: "", invalid: false, disabled: false },
);

defineEmits<{ (event: "update:modelValue", value: string): void }>();
</script>

<template>
  <input
    class="v-input"
    :value="modelValue"
    :type="type"
    :placeholder="placeholder"
    :aria-label="ariaLabel || placeholder"
    :aria-invalid="invalid || undefined"
    :disabled="disabled"
    @input="$emit('update:modelValue', ($event.target as HTMLInputElement).value)"
  />
</template>
