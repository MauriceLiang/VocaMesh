<script setup lang="ts">
import { Check, Info, TriangleAlert, X } from "lucide-vue-next";
import { useAppStore } from "@/stores/app";

const app = useAppStore();

const iconFor = (tone: string) => (tone === "success" ? Check : tone === "failure" ? X : tone === "blocked" ? TriangleAlert : Info);
const colorFor = (tone: string) =>
  tone === "success" ? "var(--status-success)" : tone === "failure" ? "var(--status-failure)" : tone === "blocked" ? "var(--status-blocked)" : "var(--text-secondary)";
</script>

<template>
  <div class="v-toasts" aria-live="polite">
    <div v-for="item in app.toasts" :key="item.id" class="v-toast">
      <component :is="iconFor(item.tone)" :size="13" :style="{ color: colorFor(item.tone), marginTop: '1px' }" aria-hidden="true" />
      <span>{{ item.text }}</span>
    </div>
  </div>
</template>
