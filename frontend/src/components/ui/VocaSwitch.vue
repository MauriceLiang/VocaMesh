<script setup lang="ts">
import { SwitchRoot, SwitchThumb } from "reka-ui";

withDefaults(
  defineProps<{
    modelValue: boolean;
    label: string;
    disabled?: boolean;
  }>(),
  { disabled: false },
);

defineEmits<{ (event: "update:modelValue", value: boolean): void }>();
</script>

<template>
  <SwitchRoot
    :model-value="modelValue"
    :disabled="disabled"
    :aria-label="label"
    class="v-switch"
    :class="{ 'is-on': modelValue }"
    @update:model-value="$emit('update:modelValue', $event)"
  >
    <SwitchThumb class="sr-only" />
  </SwitchRoot>
</template>
