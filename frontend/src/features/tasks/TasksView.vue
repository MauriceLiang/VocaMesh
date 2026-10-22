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
import { taskStateMeta } from "@/lib/status";
import { taskStateFilterOptions, taskTimeFilterOptions } from "@/lib/task-filters";
import { useAppStore } from "@/stores/app";
import type { CreateTaskInput } from "@/types";

const router = useRouter();
const app = useAppStore();
const tasks = useTasksStore();
const agents = useAgentsStore();
const dialogOpen = ref(false);

const stateItems = [...taskStateFilterOptions];
const workspaceItems = ["全部", "research-2025", "vocab-ops", "release-2.4", "legal-docs"].map((value) => ({ label: value, value }));
const teamItems = ["全部", "调研团队", "内容团队", "开发者团队"].map((value) => ({ label: value, value }));
const timeItems = [...taskTimeFilterOptions];
const stateLabel = computed(() => stateItems.find((item) => item.value === tasks.filter.state)?.label ?? "未知状态");
const timeLabel = computed(() => timeItems.find((item) => item.value === tasks.filter.time)?.label ?? "未知范围");

const rows = computed(() => tasks.visibleTasks);

onMounted(async () => {
  await Promise.all([tasks.load(), agents.load()]);
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
      :subtitle="`${tasks.tasks.length} 个任务 · ${tasks.counters.running} 个执行中 · ${tasks.counters.waiting} 个等待你处理（当前工作在此管理；历史记录用于追溯已结束的 Run）`"
    >
      <template #actions>
        <VocaDropdown :items="stateItems" @select="setStateFilter">
          <VocaButton><SlidersHorizontal :size="12" />筛选：{{ stateLabel }}</VocaButton>
        </VocaDropdown>
        <VocaButton variant="primary" @click="dialogOpen = true">新建任务</VocaButton>
      </template>
    </PageHeader>

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
        <VocaButton size="sm">工作空间：{{ tasks.filter.workspace }}</VocaButton>
      </VocaDropdown>
      <VocaDropdown :items="teamItems" @select="(value) => (tasks.filter.team = value)">
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
              <div class="v-tiny v-muted">{{ task.subtasks.filter((s) => s.state === "success").length }}/{{ task.subtasks.length }} 子任务 · {{ task.stateNote }}</div>
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
                <span class="text-[15px] font-semibold">没有匹配的任务</span>
                <span class="v-small">试试清除筛选，或创建第一项任务。</span>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <div class="v-tiny v-muted">演示数据：所有任务、Run 与路径均为虚构。点击行进入任务详情；审批入口仍以“待审批”为准，避免双份记录。</div>

    <CreateTaskDialog v-model="dialogOpen" @submit="createFromDialog" />
  </div>
</template>
