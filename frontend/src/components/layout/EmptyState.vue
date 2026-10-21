<script setup lang="ts">
defineProps<{ title: string; description: string }>();
</script>

<template>
  <div class="v-empty">
    <span class="text-[15px] font-semibold" :style="{ color: 'var(--text-primary)' }">{{ title }}</span>
    <span class="v-small">{{ description }}</span>
    <div v-if="$slots.default" class="flex items-center gap-2">
      <slot />
    </div>
  </div>
</template>
