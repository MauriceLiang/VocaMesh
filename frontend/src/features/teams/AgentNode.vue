<script setup lang="ts">
import { Handle, Position } from "@vue-flow/core";
import { Ellipsis, Plus } from "lucide-vue-next";
import { agentStateMeta } from "@/lib/status";

const props = defineProps<{
  data: {
    agentId: string;
    name: string;
    role: string;
    state: "ready" | "auth-required" | "not-ready" | "unknown";
    isRoot: boolean;
    selected: boolean;
    onAddChild: (agentId: string) => void;
    onMenu: (agentId: string, event: MouseEvent) => void;
  };
}>();

const meta = () => agentStateMeta(props.data.state);
const dotColor = () => {
  const tone = meta().tone;
  return tone === "success" ? "var(--status-success)" : tone === "blocked" ? "var(--status-blocked)" : "var(--text-muted)";
};
</script>

<template>
  <div
    class="group relative flex w-[196px] items-center gap-2 rounded-[8px] px-[10px] py-[9px]"
    :style="{
      background: 'var(--bg-surface)',
      border: data.selected ? '2px solid var(--text-primary)' : '1px solid var(--border-default)',
      padding: data.selected ? '8px 9px' : '9px 10px',
      boxShadow: 'var(--shadow-selected)',
      cursor: 'grab',
    }"
  >
    <Handle type="target" :position="Position.Top" style="opacity: 0" />
    <span
      class="grid h-[26px] w-[26px] shrink-0 place-items-center rounded-[7px] text-[11px] font-semibold"
      :style="{
        background: data.isRoot ? 'var(--btn-primary-bg)' : 'var(--bg-subtle)',
        color: data.isRoot ? 'var(--btn-primary-text)' : 'var(--text-secondary)',
      }"
    >
      {{ data.name.slice(0, 1) }}
    </span>
    <span class="flex min-w-0 flex-1 flex-col gap-[1px]">
      <b class="truncate text-[12px] font-semibold">{{ data.name }}</b>
      <span class="truncate text-[10px]" :style="{ color: 'var(--text-muted)' }">
        {{ data.isRoot ? "主管 · " : "" }}{{ data.role }}
      </span>
    </span>
    <span class="h-[6px] w-[6px] shrink-0 rounded-full" :style="{ background: dotColor() }" :title="meta().label" />

    <span class="absolute -bottom-[11px] left-1/2 flex -translate-x-1/2 gap-1 opacity-0 transition-opacity group-hover:opacity-100" :class="{ 'opacity-100': data.selected }">
      <button
        class="grid h-[22px] w-[22px] place-items-center rounded-full border"
        :style="{ background: 'var(--bg-surface)', borderColor: 'var(--border-default)', boxShadow: 'var(--shadow-selected)' }"
        :aria-label="`为 ${data.name} 添加下属`"
        @click.stop="data.onAddChild(data.agentId)"
      >
        <Plus :size="11" />
      </button>
      <button
        class="grid h-[22px] w-[22px] place-items-center rounded-full border"
        :style="{ background: 'var(--bg-surface)', borderColor: 'var(--border-default)', boxShadow: 'var(--shadow-selected)' }"
        :aria-label="`${data.name} 的更多操作`"
        @click.stop="(event) => data.onMenu(data.agentId, event as MouseEvent)"
      >
        <Ellipsis :size="11" />
      </button>
    </span>
    <Handle type="source" :position="Position.Bottom" style="opacity: 0" />
  </div>
</template>
