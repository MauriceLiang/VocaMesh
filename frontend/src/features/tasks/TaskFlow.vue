<script setup lang="ts">
import { ChevronRight, CircleHelp, Hourglass, Loader, Lock, ShieldCheck, X, Check } from "lucide-vue-next";
import type { Subtask } from "@/types";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import { subtaskStateMeta } from "@/lib/status";

defineProps<{ subtasks: Subtask[]; highlightId?: string | null }>();
defineEmits<{ (event: "open", id: string): void }>();

const iconFor = (state: Subtask["state"]) => {
  if (state === "success") return Check;
  if (state === "failure") return X;
  if (state === "waiting-approval") return ShieldCheck;
  if (state === "running") return Loader;
  if (state === "waiting-dependency") return Hourglass;
  if (state === "blocked") return Lock;
  return CircleHelp;
};
</script>

<template>
  <div class="v-flow">
    <button
      v-for="subtask in subtasks"
      :key="subtask.id"
      class="v-flow-node"
      :class="{
        'is-blocked': subtask.state === 'waiting-approval' || subtask.state === 'blocked',
        'is-failure': subtask.state === 'failure',
        'is-success': subtask.state === 'success',
      }"
      :style="highlightId === subtask.id ? { background: 'var(--bg-subtle)', boxShadow: 'inset 2px 0 0 var(--text-primary)' } : undefined"
      @click="$emit('open', subtask.id)"
    >
      <span class="nb">
        <component
          :is="iconFor(subtask.state)"
          :size="11"
          :class="{ 'v-spin': subtask.state === 'running' }"
          :style="{
            color:
              subtask.state === 'success'
                ? 'var(--status-success)'
                : subtask.state === 'failure'
                  ? 'var(--status-failure)'
                  : subtask.state === 'waiting-approval' || subtask.state === 'blocked'
                    ? 'var(--status-blocked)'
                    : 'var(--text-secondary)',
          }"
        />
      </span>
      <span class="flex min-w-0 flex-1 flex-col gap-[2px]">
        <span class="nt">{{ subtask.name }}</span>
        <span class="flex flex-wrap gap-x-[7px]">
          <span class="nm">{{ subtask.assignee }}</span>
          <span class="nm">· {{ subtask.result }}</span>
        </span>
      </span>
      <VocaStatus :meta="subtaskStateMeta(subtask.state)" />
      <ChevronRight :size="13" class="v-muted" />
    </button>
  </div>
</template>
