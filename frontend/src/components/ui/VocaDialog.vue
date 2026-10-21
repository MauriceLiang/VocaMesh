<script setup lang="ts">
import { DialogContent, DialogDescription, DialogOverlay, DialogPortal, DialogRoot, DialogTitle, DialogClose } from "reka-ui";
import { X } from "lucide-vue-next";

withDefaults(
  defineProps<{
    modelValue: boolean;
    title: string;
    subtitle?: string;
    width?: number;
  }>(),
  { subtitle: "", width: 760 },
);

defineEmits<{ (event: "update:modelValue", value: boolean): void }>();
</script>

<template>
  <DialogRoot :open="modelValue" @update:open="$emit('update:modelValue', $event)">
    <DialogPortal>
      <DialogOverlay class="v-scrim">
        <DialogContent
          class="v-dialog"
          :style="{ width: `min(${width}px, 100%)` }"
          @escape-key-down="$emit('update:modelValue', false)"
        >
          <div class="dh">
            <div class="flex flex-col gap-1">
              <DialogTitle class="text-[17px] font-semibold leading-tight">{{ title }}</DialogTitle>
              <DialogDescription v-if="subtitle" class="v-tiny v-secondary">{{ subtitle }}</DialogDescription>
            </div>
            <DialogClose class="v-icon-btn" aria-label="关闭">
              <X :size="15" />
            </DialogClose>
          </div>
          <div class="db"><slot /></div>
          <div v-if="$slots.footer" class="df">
            <span class="v-tiny v-muted">演示环境：不会写入真实系统</span>
            <span class="v-spacer" />
            <slot name="footer" />
          </div>
        </DialogContent>
      </DialogOverlay>
    </DialogPortal>
  </DialogRoot>
</template>
