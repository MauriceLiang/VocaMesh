<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { VueFlow, useVueFlow, type Edge, type Node } from "@vue-flow/core";
import { Background } from "@vue-flow/background";
import { Maximize2, Minus, Plus, RefreshCw } from "lucide-vue-next";
import AgentNode from "./AgentNode.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaIconButton from "@/components/ui/VocaIconButton.vue";
import { NODE_HEIGHT, NODE_WIDTH, useTeamsStore, type GraphPosition } from "@/stores/teams";
import "@vue-flow/core/dist/style.css";
import "@vue-flow/core/dist/theme-default.css";

const emit = defineEmits<{
  (event: "add-child", agentId: string): void;
  (event: "edit", agentId: string): void;
  (event: "superior", agentId: string): void;
  (event: "make-supervisor", agentId: string): void;
  (event: "remove", agentId: string): void;
  (event: "new-agent"): void;
}>();

const teams = useTeamsStore();
const { fitView, zoomIn, zoomOut } = useVueFlow({ id: "team-graph" });

const graphHost = ref<HTMLElement | null>(null);
const graphWidth = ref(0);
const compactGraph = computed(() => graphWidth.value > 0 && graphWidth.value < 640);
const compactPositions = ref<Record<string, GraphPosition>>({});
let graphResizeObserver: ResizeObserver | null = null;

const menu = ref<{ agentId: string; x: number; y: number } | null>(null);

const nodes = computed<Node[]>(() =>
  teams.members.map((member, index) => ({
    id: member.agentId,
    type: "agent",
    position: compactGraph.value
      ? compactPositions.value[member.agentId] ?? {
          x: Math.max(24, (graphWidth.value - NODE_WIDTH) / 2),
          y: 40 + index * (NODE_HEIGHT + 28),
        }
      : teams.positions[member.agentId] ?? { x: 40, y: 40 },
    draggable: true,
    data: {
      agentId: member.agentId,
      name: member.name,
      role: member.role,
      state: member.state,
      isRoot: member.agentId === teams.selected?.supervisorId,
      selected: teams.selectedMemberId === member.agentId,
      onAddChild: (agentId: string) => emit("add-child", agentId),
      onMenu: (agentId: string, event: MouseEvent) => {
        menu.value = { agentId, x: event.clientX, y: event.clientY };
      },
    },
  })),
);

const edges = computed<Edge[]>(() =>
  teams.members
    .filter((member) => member.reportsTo && member.reportsTo !== "user")
    .map((member) => ({
      id: `${member.reportsTo}->${member.agentId}`,
      source: member.reportsTo,
      target: member.agentId,
      type: "smoothstep",
      style: {
        stroke: teams.selectedMemberId === member.agentId ? "var(--text-primary)" : "var(--border-strong)",
        strokeWidth: teams.selectedMemberId === member.agentId ? 2 : 1.4,
      },
    })),
);

function onDragStop(event: { node: Node }): void {
  teams.moveNode(event.node.id, { x: event.node.position.x, y: event.node.position.y });
  if (compactGraph.value) {
    compactPositions.value = {
      ...compactPositions.value,
      [event.node.id]: { x: event.node.position.x, y: event.node.position.y },
    };
  }
}

function tidy(): void {
  teams.layout(true);
  window.setTimeout(() => void fitView({ padding: 0.25 }), 60);
}

function closeMenu(): void {
  menu.value = null;
}

function pick(action: "edit" | "superior" | "make-supervisor" | "remove"): void {
  if (!menu.value) return;
  const agentId = menu.value.agentId;
  closeMenu();
  switch (action) {
    case "edit":
      emit("edit", agentId);
      break;
    case "superior":
      emit("superior", agentId);
      break;
    case "make-supervisor":
      emit("make-supervisor", agentId);
      break;
    case "remove":
      emit("remove", agentId);
      break;
  }
}

onMounted(() => {
  if (!graphHost.value) return;
  graphResizeObserver = new ResizeObserver(([entry]) => {
    graphWidth.value = entry.contentRect.width;
  });
  graphResizeObserver.observe(graphHost.value);
});

onBeforeUnmount(() => graphResizeObserver?.disconnect());

watch(compactGraph, async () => {
  await nextTick();
  await fitView({ padding: 0.25 });
});
</script>

<template>
  <div
    class="v-card flex min-h-0 flex-1 flex-col overflow-hidden"
    :style="{ minHeight: compactGraph ? `${Math.max(420, 40 + teams.members.length * (NODE_HEIGHT + 28) + 120)}px` : '320px' }"
  >
    <div class="flex flex-wrap items-center gap-2 border-b px-3 py-2" :style="{ borderColor: 'var(--border-default)' }">
      <span class="v-tiny v-muted">拖动空白平移 · 滚轮缩放 · 拖动节点调整位置</span>
      <span class="v-spacer" />
      <VocaIconButton label="缩小" @click="zoomOut"><Minus :size="12" /></VocaIconButton>
      <VocaIconButton label="适应视图" @click="fitView({ padding: 0.25 })"><Maximize2 :size="13" /></VocaIconButton>
      <VocaIconButton label="放大" @click="zoomIn"><Plus :size="12" /></VocaIconButton>
      <VocaButton size="sm" @click="fitView({ padding: 0.25 })">适应视图</VocaButton>
      <VocaButton size="sm" @click="tidy"><RefreshCw :size="12" />自动整理</VocaButton>
      <VocaButton size="sm" variant="primary" @click="emit('new-agent')"><Plus :size="12" />添加 Agent</VocaButton>
    </div>

    <div ref="graphHost" class="relative min-h-0 flex-1" @click="closeMenu">
      <VueFlow
        id="team-graph"
        :nodes="nodes"
        :edges="edges"
        :min-zoom="0.45"
        :max-zoom="1.6"
        :nodes-connectable="false"
        :elements-selectable="false"
        :pan-on-scroll="false"
        fit-view-on-init
        :default-viewport="{ zoom: 0.9 }"
        @node-drag-stop="onDragStop"
        @node-click="(event) => teams.selectMember(event.node.id)"
      >
        <template #node-agent="{ data }">
          <AgentNode :data="data" />
        </template>
        <Background :gap="16" :size="1" pattern-color="var(--border-default)" />
      </VueFlow>

      <div
        v-if="menu"
        class="fixed z-[85] w-[180px] rounded-[7px] border p-1"
        :style="{ left: `${menu.x}px`, top: `${menu.y}px`, background: 'var(--bg-surface)', borderColor: 'var(--border-default)', boxShadow: 'var(--shadow-overlay)' }"
        @click.stop
      >
        <button class="v-menu-item" @click="pick('edit')">编辑 Agent 信息</button>
        <button class="v-menu-item" @click="emit('add-child', menu.agentId); closeMenu()">添加下属</button>
        <button class="v-menu-item" @click="pick('superior')">添加直属上级…</button>
        <button class="v-menu-item" @click="pick('make-supervisor')">设为团队主管</button>
        <button class="v-menu-item" :style="{ color: 'var(--status-failure)' }" @click="pick('remove')">从此团队移除</button>
      </div>

      <div class="pointer-events-none absolute bottom-[10px] left-3 v-tiny v-muted">
        共 {{ teams.members.length }} 个节点 · 主管：{{ teams.selected?.supervisorId ? teams.members.find((m) => m.agentId === teams.selected?.supervisorId)?.name : "—" }} · 实线 = 直属汇报关系
      </div>
    </div>
  </div>
</template>
