<script setup lang="ts">
import { DropdownMenuContent, DropdownMenuItem, DropdownMenuPortal, DropdownMenuRoot, DropdownMenuTrigger } from "reka-ui";

export interface DropdownItem {
  label: string;
  value: string;
  danger?: boolean;
  disabled?: boolean;
  checked?: boolean;
}

defineProps<{ items: DropdownItem[]; align?: "start" | "end" }>();
defineEmits<{ (event: "select", value: string): void }>();
</script>

<template>
  <DropdownMenuRoot>
    <DropdownMenuTrigger as-child>
      <slot />
    </DropdownMenuTrigger>
    <DropdownMenuPortal>
      <DropdownMenuContent
        :align="align ?? 'start'"
        :side-offset="4"
        class="v-menu z-[80]"
        :style="{ minWidth: '200px' }"
      >
        <DropdownMenuItem
          v-for="item in items"
          :key="item.value"
          :disabled="item.disabled"
          class="v-menu-item"
          :style="item.danger ? { color: 'var(--status-failure)' } : undefined"
          @select="$emit('select', item.value)"
        >
          <span class="v-spacer">{{ item.label }}</span>
          <span v-if="item.checked" class="v-tiny">✓</span>
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenuPortal>
  </DropdownMenuRoot>
</template>
