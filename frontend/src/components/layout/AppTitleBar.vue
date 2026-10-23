<script setup lang="ts">
import { computed } from "vue";
import { useAppStore } from "@/stores/app";
import { useServices } from "@/services/adapters";

defineProps<{ label: string }>();
const app = useAppStore();
const services = useServices();
const platform = computed(() => app.settings.platform);
const mockPreview = services.mode === "mock";
</script>

<template>
  <div class="v-titlebar">
    <div v-if="mockPreview && platform === 'mac'" class="v-lights" aria-hidden="true">
      <span class="v-light r" /><span class="v-light y" /><span class="v-light g" />
    </div>
    <span class="truncate text-[var(--fs-small)]" :style="{ color: 'var(--text-muted)', maxWidth: '45%' }">VocaMesh — {{ label }}</span>
    <span v-if="mockPreview" class="v-kbd" style="text-transform: uppercase; letter-spacing: 0.4px; border-style: dashed; white-space: nowrap">mock 演示</span>
    <div class="drag" />
    <div v-if="mockPreview && platform === 'windows'" class="v-win-controls" aria-hidden="true">
      <span class="v-win-btn">–</span>
      <span class="v-win-btn">▢</span>
      <span class="v-win-btn close">✕</span>
    </div>
  </div>
</template>
