<script setup lang="ts">
withDefaults(defineProps<{ padded?: "none" | "panel" | "detail" }>(), { padded: "none" });
</script>

<template>
  <div class="v-card" :class="{ 'v-panel': padded === 'panel', 'v-detail-panel': padded === 'detail' }">
    <slot />
  </div>
</template>
