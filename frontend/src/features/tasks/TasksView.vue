<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { MoreHorizontal, Search, SlidersHorizontal } from "lucide-vue-next";
import PageHeader from "@/components/layout/PageHeader.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaDropdown from "@/components/ui/VocaDropdown.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import CreateTaskDialog from "./CreateTaskDialog.vue";
import { useTasksStore } from "@/stores/tasks";
import { useAgentsStore } from "@/stores/agents";
import { useWorkspacesStore } from "@/stores/workspaces";
import { taskStateMeta } from "@/lib/status";
import { taskStateFilterOptions, taskTimeFilterOptions } from "@/lib/task-filters";
import { useAppStore } from "@/stores/app";
import { useServices } from "@/services/adapters";
import type { CreateTaskInput } from "@/types";

const router = useRouter();
const app = useAppStore();
const tasks = useTasksStore();
const agents = useAgentsStore();
const workspaces = useWorkspacesStore();
const services = useServices();
const isNative = services.mode === "tauri";
const dialogOpen = ref(false);

const nativeStateItems = [
  { label: "全部", value: "all" },
  { label: "排队中", value: "queued" },
  { label: "启动中", value: "starting" },
  { label: "执行中", value: "running" },
  { label: "正在停止", value: "stop-pending" },
  { label: "已停止", value: "stopped" },
  { label: "已完成", value: "done" },
  { label: "失败", value: "failed" },
  { label: "状态未知", value: "unknown" },
] as const;
const stateItems = computed(() => isNative ? [...nativeStateItems] : [...taskStateFilterOptions]);
const workspaceItems = computed(() => {
  if (!isNative) return ["全部", "research-2025", "vocab-ops", "release-2.4", "legal-docs"].map((value) => ({ label: value, value }));
  const registered = workspaces.workspaces.map((workspace) => ({ label: workspace.name, value: workspace.path }));
  const registeredPaths = new Set(registered.map((item) => item.value));
  const historical = [...new Set(tasks.tasks.map((task) => task.workspace))]
    .filter((path) => !registeredPaths.has(path))
    .map((path) => ({ label: `${path}（历史路径）`, value: path }));
  return [{ label: "全部", value: "全部" }, ...registered, ...historical];
});
const teamItems = ["全部", "调研团队", "内容团队", "开发者团队"].map((value) => ({ label: value, value }));
const timeItems = [...taskTimeFilterOptions];
const stateLabel = computed(() => stateItems.value.find((item) => item.value === tasks.filter.state)?.label ?? "未知状态");
const workspaceLabel = computed(() => workspaceItems.value.find((item) => item.value === tasks.filter.workspace)?.label ?? tasks.filter.workspace);
const timeLabel = computed(() => timeItems.find((item) => item.value === tasks.filter.time)?.label ?? "未知范围");

const rows = computed(() => tasks.visibleTasks);

onMounted(async () => {
  await Promise.all([tasks.load(), agents.load(), ...(isNative ? [workspaces.load()] : [])]);
});

function openTask(id: string): void {
  void router.push({ name: "task-detail", params: { id } });
}

async function createFromDialog(payload: CreateTaskInput): Promise<void> {
  const task = await tasks.create(payload);
  dialogOpen.value = false;
  if (task) await router.push({ name: "task-detail", params: { id: task.id } });
}

function setStateFilter(value: string): void {
  if (!tasks.setStateFilter(value)) app.toast("未知的任务状态筛选，当前筛选未更改", "neutral");
}

function setTimeFilter(value: string): void {
  if (!tasks.setTimeFilter(value)) app.toast("未知的时间筛选，当前筛选未更改", "neutral");
}

function formatCreatedAt(value: string): string {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "时间未知" : new Intl.DateTimeFormat("zh-CN", { month: "2-digit", day: "2-digit" }).format(date);
}
</script>

<template>
  <div class="v-content">
    <PageHeader
      title="全部任务"
      :subtitle="isNative
        ? `${tasks.tasks.length} 个本机任务 · ${tasks.counters.running} 个执行中 · ${tasks.counters.waiting} 个等待处理`
        : `${tasks.tasks.length} 个任务 · ${tasks.counters.running} 个执行中 · ${tasks.counters.waiting} 个等待你处理（当前工作在此管理；历史记录用于追溯已结束的 Run）`"
    >
      <template #actions>
        <VocaDropdown :items="stateItems" @select="setStateFilter">
          <VocaButton><SlidersHorizontal :size="12" />筛选：{{ stateLabel }}</VocaButton>
        </VocaDropdown>
        <VocaButton variant="primary" @click="dialogOpen = true">新建任务</VocaButton>
      </template>
    </PageHeader>

    <div v-if="isNative && workspaces.lastError" class="v-banner is-failure"><span class="flex-1">{{ workspaces.lastError }}</span><VocaButton size="sm" @click="workspaces.load()">重试工作空间</VocaButton></div>

    <div class="flex flex-wrap items-center gap-2">
      <label class="v-searchbox" :style="{ minWidth: '260px' }">
        <Search :size="13" />
        <input
          :value="tasks.filter.query"
          class="w-full bg-transparent text-[12px] outline-none"
          placeholder="搜索任务名称、工作空间或 Run"
          aria-label="搜索任务"
          @input="tasks.filter.query = ($event.target as HTMLInputElement).value"
        />
      </label>
      <VocaDropdown :items="workspaceItems" @select="(value) => (tasks.filter.workspace = value)">
        <VocaButton size="sm">工作空间：{{ workspaceLabel }}</VocaButton>
      </VocaDropdown>
      <VocaDropdown v-if="!isNative" :items="teamItems" @select="(value) => (tasks.filter.team = value)">
        <VocaButton size="sm">执行者：{{ tasks.filter.team }}</VocaButton>
      </VocaDropdown>
      <VocaDropdown :items="timeItems" @select="setTimeFilter">
        <VocaButton size="sm">时间：{{ timeLabel }}</VocaButton>
      </VocaDropdown>
      <span class="v-spacer" />
      <VocaButton variant="ghost" size="sm" @click="tasks.resetFilter()">清除筛选</VocaButton>
    </div>

    <div class="v-table-wrap" :style="{ flex: '1 1 auto', overflow: 'auto' }">
      <table class="v-table">
        <thead>
          <tr><th>状态</th><th>任务</th><th>工作空间</th><th>执行者</th><th>Run</th><th>创建</th><th></th></tr>
        </thead>
        <tbody>
          <tr v-for="task in rows" :key="task.id" class="clickable" @click="openTask(task.id)">
            <td><VocaStatus :meta="taskStateMeta(task.state)" /></td>
            <td>
              <div class="font-medium">{{ task.name }}</div>
              <div class="v-tiny v-muted">{{ task.nativeExecution ? task.stateNote : `${task.subtasks.filter((s) => s.state === "success").length}/${task.subtasks.length} 子任务 · ${task.stateNote}` }}</div>
            </td>
            <td class="v-mono v-tiny v-secondary">{{ task.workspace }}</td>
            <td class="v-small v-secondary">{{ task.team }}</td>
            <td class="v-mono v-tiny v-muted">{{ task.runId }}</td>
            <td class="v-small v-muted">{{ formatCreatedAt(task.createdAt) }}</td>
            <td><MoreHorizontal :size="14" class="v-muted" /></td>
          </tr>
          <tr v-if="!rows.length">
            <td colspan="7">
              <div class="v-empty">
                <span class="text-[15px] font-semibold">{{ isNative && !tasks.tasks.length ? "暂无本机任务" : "没有匹配的任务" }}</span>
                <span class="v-small">{{ isNative && !tasks.tasks.length ? "本机安全策略当前阻止创建并执行任务；此列表不会填入演示记录。" : "试试清除筛选，或创建第一项任务。" }}</span>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <div v-if="app.isMock" class="v-tiny v-muted">演示数据：所有任务、Run 与路径均为虚构。点击行进入任务详情；审批入口仍以“待审批”为准，避免双份记录。</div>
    <div v-else class="v-tiny v-muted">本机 Task、Run 与路径来自 SQLite；团队编排和审批仍未接入真实任务执行。</div>

    <CreateTaskDialog v-model="dialogOpen" @submit="createFromDialog" @native-created="openTask($event.id)" />
  </div>
</template>
