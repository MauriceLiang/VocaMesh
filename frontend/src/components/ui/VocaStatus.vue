<script setup lang="ts">
import { computed } from "vue";
import {
  Check,
  CircleHelp,
  Dot,
  Hourglass,
  Loader,
  Pause,
  ShieldCheck,
  Square,
  TriangleAlert,
  X,
} from "lucide-vue-next";
import type { StatusMeta } from "@/lib/status";
import VocaBadge from "./VocaBadge.vue";

const props = defineProps<{ meta: StatusMeta }>();

const icons = {
  check: Check,
  x: X,
  alert: TriangleAlert,
  dot: Dot,
  loader: Loader,
  help: CircleHelp,
  pause: Pause,
  square: Square,
  hourglass: Hourglass,
  shield: ShieldCheck,
} as const;

const icon = computed(() => icons[props.meta.icon]);
const spin = computed(() => props.meta.icon === "loader");
</script>

<template>
  <VocaBadge :tone="meta.tone" :label="meta.label">
    <template #icon>
      <component :is="icon" :size="11" :class="{ 'v-spin': spin }" aria-hidden="true" />
    </template>
  </VocaBadge>
</template>
