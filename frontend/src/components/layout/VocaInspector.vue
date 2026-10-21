<script setup lang="ts">
import { X } from "lucide-vue-next";

defineProps<{ title: string; subtitle?: string }>();
defineEmits<{ (event: "close"): void }>();
</script>

<template>
  <aside class="v-inspector" aria-label="检查器" :style="{ width: 'var(--inspector-w)', flex: '0 0 var(--inspector-w)', background: 'var(--bg-surface)', borderLeft: '1px solid var(--border-default)', display: 'flex', flexDirection: 'column', minHeight: 0 }">
    <div class="flex items-center gap-2 px-[14px] py-3">
      <span class="flex min-w-0 flex-1 flex-col gap-[1px]">
        <b class="v-small truncate">{{ title }}</b>
        <span class="v-tiny v-muted truncate">{{ subtitle }}</span>
      </span>
      <button class="v-icon-btn" aria-label="关闭检查器" title="关闭检查器" @click="$emit('close')">
        <X :size="15" />
      </button>
    </div>
    <slot name="tabs" />
    <div class="min-h-0 flex-1 overflow-hidden"><slot /></div>
  </aside>
</template>

<style scoped>
.v-inspector { border-left: 1px solid var(--border-default); }
@media (max-width: 1080px) {
  .v-inspector { position: fixed; right: 0; top: 40px; bottom: 0; width: 320px; flex-basis: 320px; z-index: 60; box-shadow: var(--shadow-overlay); }
}
</style>
