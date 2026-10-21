<script setup lang="ts">
defineProps<{ title: string; subtitle?: string }>();
</script>

<template>
  <header class="v-page-head">
    <div class="min-w-0">
      <h1 class="v-page-title">{{ title }}</h1>
      <p v-if="subtitle" class="v-page-sub">{{ subtitle }}</p>
    </div>
    <div v-if="$slots.actions" class="v-head-actions">
      <slot name="actions" />
    </div>
  </header>
</template>
