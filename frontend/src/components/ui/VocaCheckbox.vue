<script setup lang="ts">
import { CheckboxIndicator, CheckboxRoot } from "reka-ui";
import { Check } from "lucide-vue-next";

defineProps<{ modelValue: boolean; label: string; disabled?: boolean }>();
defineEmits<{ (event: "update:modelValue", value: boolean): void }>();
</script>

<template>
  <label class="flex items-center gap-2 cursor-pointer">
    <CheckboxRoot
      :model-value="modelValue"
      :disabled="disabled"
      :aria-label="label"
      class="grid place-items-center rounded-[4px] border"
      :style="{
        width: '16px',
        height: '16px',
        borderColor: modelValue ? 'var(--btn-primary-bg)' : 'var(--border-strong)',
        background: modelValue ? 'var(--btn-primary-bg)' : 'var(--bg-surface)',
      }"
      @update:model-value="$emit('update:modelValue', $event === true)"
    >
      <CheckboxIndicator>
        <Check :size="12" :style="{ color: 'var(--btn-primary-text)' }" />
      </CheckboxIndicator>
    </CheckboxRoot>
    <span class="v-small">{{ label }}</span>
  </label>
</template>
