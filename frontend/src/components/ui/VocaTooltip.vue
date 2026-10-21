<script setup lang="ts">
import { TooltipContent, TooltipPortal, TooltipProvider, TooltipRoot, TooltipTrigger } from "reka-ui";

defineProps<{ text: string }>();
</script>

<template>
  <TooltipProvider :delay-duration="300">
    <TooltipRoot>
      <TooltipTrigger as-child>
        <slot />
      </TooltipTrigger>
      <TooltipPortal>
        <TooltipContent
          side="bottom"
          :side-offset="6"
          class="z-[95] rounded-[6px] px-2 py-1 text-[11px]"
          :style="{ background: 'var(--text-primary)', color: 'var(--bg-surface)', boxShadow: 'var(--shadow-overlay)' }"
        >
          {{ text }}
        </TooltipContent>
      </TooltipPortal>
    </TooltipRoot>
  </TooltipProvider>
</template>
