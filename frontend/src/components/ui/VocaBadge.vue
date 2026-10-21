<script setup lang="ts">
import { computed } from "vue";
import type { StatusTone } from "@/types";

const props = withDefaults(
  defineProps<{ tone?: StatusTone; label: string }>(),
  { tone: "neutral" },
);

const classes = computed(() => {
  const map: Record<StatusTone, string> = {
    success: "is-success",
    failure: "is-failure",
    blocked: "is-blocked",
    neutral: "is-neutral",
    unknown: "is-neutral",
  };
  return ["v-badge", map[props.tone]];
});
</script>

<template>
  <span :class="classes">
    <slot name="icon" />
    {{ label }}
  </span>
</template>
