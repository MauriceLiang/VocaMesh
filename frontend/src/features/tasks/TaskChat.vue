<script setup lang="ts">
import { computed } from "vue";
import { ShieldCheck } from "lucide-vue-next";
import type { ApprovalRequest, Task } from "@/types";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import { instructionStateMeta } from "@/lib/instruction";

const props = defineProps<{ task: Task; approval?: ApprovalRequest | null }>();
const emit = defineEmits<{ (event: "decide", id: string, decision: "allow-once" | "deny"): void }>();

const instruction = computed(() => props.task.instruction ?? null);
</script>

<template>
  <div class="v-chat" :style="{ flex: '1 1 auto', minHeight: '200px' }">
    <div v-if="approval" class="flex gap-[10px] rounded-[8px] p-[11px]" :style="{ background: 'var(--bg-surface)', border: '1px solid var(--status-blocked)' }">
      <span class="flex min-w-0 flex-1 flex-col gap-[5px]">
        <span class="flex items-center gap-2">
          <VocaStatus :meta="{ label: '等待审批', tone: 'blocked', icon: 'alert' }" />
          <span class="v-tiny v-secondary">写作者请求写入文件</span>
        </span>
        <span class="v-mono" :style="{ fontSize: '11px' }">{{ approval.target }}</span>
        <span class="v-tiny v-muted">{{ approval.scope }}</span>
      </span>
      <span class="flex flex-col items-end gap-[6px]">
        <VocaButton size="sm" variant="ghost" @click="emit('decide', approval.id, 'deny')">拒绝</VocaButton>
        <VocaButton size="sm" variant="primary" @click="emit('decide', approval.id, 'allow-once')">仅此一次允许</VocaButton>
      </span>
    </div>

    <div v-if="instruction" class="v-card v-panel" :style="{ gap: '6px', background: 'var(--bg-inset)' }">
      <div class="v-row-between">
        <span class="flex items-center gap-[6px]"><ShieldCheck :size="13" /><b class="v-small">定向指令 · 目标 {{ instruction.target }}</b></span>
        <VocaStatus :meta="instructionStateMeta(instruction.state)" />
      </div>
      <div class="v-small v-secondary">{{ instruction.text }}</div>
      <div class="v-tiny v-muted">已同步主管（结构化事件，不代表主管实时读取全部日志）· {{ instruction.impact }}</div>
    </div>

    <div
      v-for="message in task.messages"
      :key="message.id"
      class="v-msg"
      :class="{ me: message.author === 'user' }"
    >
      <span v-if="message.author !== 'user'" class="v-avatar-sq">主</span>
      <div class="bubble">
        <div v-if="message.author !== 'user'">
          <span class="text-[10.5px] font-semibold">{{ message.authorName }}</span>
          <span class="v-tiny v-muted" :style="{ marginLeft: '6px' }">{{ message.time }}</span>
        </div>
        <div>{{ message.text }}</div>
        <div v-if="message.author === 'user'" class="v-tiny v-muted" :style="{ marginTop: '4px' }">你 · {{ message.time }}</div>
      </div>
    </div>
  </div>
</template>
