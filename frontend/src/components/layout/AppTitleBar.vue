<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "@/stores/app";

defineProps<{ label: string }>();
const app = useAppStore();
const platform = computed(() => app.settings.platform);
</script>

<template>
  <div class="v-titlebar">
    <div v-if="platform === 'mac'" class="v-lights" aria-hidden="true">
      <span class="v-light r" /><span class="v-light y" /><span class="v-light g" />
    </div>
    <span class="truncate text-[var(--fs-small)]" :style="{ color: 'var(--text-muted)', maxWidth: '45%' }">VocaMesh — {{ label }}</span>
    <span class="v-kbd" style="text-transform: uppercase; letter-spacing: 0.4px; border-style: dashed; white-space: nowrap">mock 演示</span>
    <div class="drag" />
    <div v-if="platform === 'windows'" class="v-win-controls">
      <button class="v-win-btn" aria-label="最小化">–</button>
      <button class="v-win-btn" aria-label="最大化">▢</button>
      <button class="v-win-btn close" aria-label="关闭">✕</button>
    </div>
  </div>
</template>
