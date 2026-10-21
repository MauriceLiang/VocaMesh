<script setup lang="ts" generic="T extends string">
defineProps<{ modelValue: T; items: Array<{ value: T; label: string }> }>();
defineEmits<{ (event: "update:modelValue", value: T): void }>();
</script>

<template>
  <div class="v-segmented" role="group">
    <button
      v-for="item in items"
      :key="item.value"
      type="button"
      :class="{ 'is-on': item.value === modelValue }"
      :aria-pressed="item.value === modelValue"
      @click="$emit('update:modelValue', item.value)"
    >
      {{ item.label }}
    </button>
  </div>
</template>
