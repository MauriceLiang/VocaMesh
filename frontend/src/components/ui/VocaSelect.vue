<script setup lang="ts" generic="T extends string">
import { SelectContent, SelectIcon, SelectItem, SelectItemText, SelectPortal, SelectRoot, SelectTrigger, SelectValue, SelectViewport } from "reka-ui";
import { ChevronDown } from "lucide-vue-next";

defineProps<{
  modelValue: T;
  items: Array<{ value: T; label: string }>;
  ariaLabel: string;
  disabled?: boolean;
}>();

defineEmits<{ (event: "update:modelValue", value: T): void }>();
</script>

<template>
  <SelectRoot :model-value="modelValue" :disabled="disabled" @update:model-value="$emit('update:modelValue', $event as T)">
    <SelectTrigger class="v-select" :aria-label="ariaLabel">
      <SelectValue />
      <SelectIcon><ChevronDown :size="14" class="v-muted" /></SelectIcon>
    </SelectTrigger>
    <SelectPortal>
      <SelectContent class="v-menu z-[80]" position="popper" :side-offset="4">
        <SelectViewport>
          <SelectItem
            v-for="item in items"
            :key="item.value"
            :value="item.value"
            class="v-menu-item"
          >
            <SelectItemText>{{ item.label }}</SelectItemText>
          </SelectItem>
        </SelectViewport>
      </SelectContent>
    </SelectPortal>
  </SelectRoot>
</template>
