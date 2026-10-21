<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { Check, ChevronRight, Folder, Hourglass, Loader, ShieldCheck } from "lucide-vue-next";
import PageHeader from "@/components/layout/PageHeader.vue";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import VocaSegmented from "@/components/ui/VocaSegmented.vue";
import VocaDropdown from "@/components/ui/VocaDropdown.vue";
import CreateTaskDialog from "@/features/tasks/CreateTaskDialog.vue";
import { useAppStore } from "@/stores/app";
import { useApprovalsStore } from "@/stores/approvals";
import { useTasksStore } from "@/stores/tasks";
import { useWorkspacesStore } from "@/stores/workspaces";
import { taskStateMeta } from "@/lib/status";

const router = useRouter();
const app = useAppStore();
const tasks = useTasksStore();
const approvals = useApprovalsStore();
const workspaces = useWorkspacesStore();

const description = ref("");
const mode = ref<"single-agent" | "team" | "custom">("team");
const workspace = ref("research-2025");
const dialogOpen = ref(false);

const activeTasks = computed(() =>
  tasks.tasks.filter((task) => ["running", "waiting-approval", "queued", "paused", "needs-attention"].includes(task.state)),
);
const pendingApprovals = computed(() => approvals.pending.slice(0, 2));
const doneTasks = computed(() => tasks.tasks.filter((task) => task.state === "done").slice(0, 3));

const metrics = computed(() => [
  { key: "running", label: "执行中", value: tasks.counters.running, icon: Loader, filter: "running", tone: "" },
  { key: "waiting", label: "等待你处理", value: tasks.counters.waiting, icon: ShieldCheck, filter: "waiting-approval", tone: "is-blocked" },
  { key: "queued", label: "排队中", value: tasks.counters.queued, icon: Hourglass, filter: "queued", tone: "" },
  { key: "done", label: "今日已完成", value: 8, icon: Check, filter: "done", tone: "is-success" },
]);

const modeHint = computed(() => {
  if (mode.value === "single-agent") return "使用：研究员 · 不额外启动主管";
  if (mode.value === "team") return "使用团队：调研团队 · 主管统一协调";
  return "高级：子任务、依赖与授权手动定义";
});

const workspaceItems = computed(() => workspaces.workspaces.map((item) => ({ label: item.name, value: item.name })));

onMounted(async () => {
  await Promise.all([tasks.load(), approvals.load(), workspaces.load()]);
});

async function create(): Promise<void> {
  const text = description.value.trim() || "整理 Q3 竞品资料并生成对比报告";
  const task = await tasks.create({ description: text, workspace: workspace.value, mode: mode.value });
  if (task) {
    description.value = "";
    await router.push({ name: "task-detail", params: { id: task.id } });
  }
}

function submit(): void {
  if (!description.value.trim()) {
    dialogOpen.value = true;
    return;
  }
  void create();
}

function openTask(id: string): void {
  void router.push({ name: "task-detail", params: { id } });
}

async function decide(id: string, decision: "allow-once" | "deny"): Promise<void> {
  await approvals.decide(id, decision);
  await tasks.load();
}

async function handleDialogSubmit(payload: { description: string; workspace: string; mode: "single-agent" | "team" | "custom" }): Promise<void> {
  description.value = payload.description;
  workspace.value = payload.workspace;
  mode.value = payload.mode;
  dialogOpen.value = false;
  await create();
}
</script>

<template>
  <div class="v-content">
    <PageHeader title="工作台" :subtitle="`下午好 · ${tasks.counters.running} 项任务正在执行，${pendingApprovals.length} 项等待你处理`">
      <template #actions>
        <VocaButton variant="ghost" @click="app.toast('模板库：演示环境仅提供预设示例', 'neutral')">模板库</VocaButton>
        <VocaButton variant="primary" @click="dialogOpen = true">新建任务</VocaButton>
      </template>
    </PageHeader>

    <div class="v-card" :style="{ padding: '12px', display: 'flex', flexDirection: 'column', gap: '10px' }">
      <textarea
        v-model="description"
        class="composer-input"
        aria-label="描述你要完成的工作"
        placeholder="描述你要完成的工作，例如：把 2025 Q3 三家竞品的公开资料整理成一份对比报告…"
      />
      <div class="flex flex-wrap items-center gap-[10px]">
        <VocaDropdown :items="workspaceItems" @select="(value) => (workspace = value)">
          <button class="flex h-7 items-center gap-[6px] rounded-[6px] border px-[9px]" :style="{ borderColor: 'var(--border-default)', background: 'var(--bg-surface)' }">
            <Folder :size="13" class="v-secondary" />
            <span class="v-mono v-tiny">{{ workspace }}</span>
            <ChevronRight :size="12" class="v-muted rotate-90" />
          </button>
        </VocaDropdown>
        <VocaSegmented
          v-model="mode"
          :items="[
            { value: 'single-agent', label: '快速执行' },
            { value: 'team', label: '选择团队' },
            { value: 'custom', label: '自定义' },
          ]"
        />
        <span class="v-spacer" />
        <span class="v-tiny v-muted">{{ modeHint }}</span>
        <VocaButton variant="primary" @click="submit">创建任务</VocaButton>
      </div>
    </div>

    <div class="v-metrics">
      <button
        v-for="metric in metrics"
        :key="metric.key"
        class="v-metric"
        @pointerup="tasks.filter.state = metric.filter; router.push({ name: 'tasks' })"
      >
        <span class="ib" :class="metric.tone"><component :is="metric.icon" :size="14" /></span>
        <span class="flex flex-col items-start gap-[1px]">
          <b class="v">{{ metric.value }}</b>
          <span class="l">{{ metric.label }}</span>
        </span>
      </button>
    </div>

    <div class="v-stack" :style="{ flex: '1 1 auto', minHeight: 0 }">
      <div class="v-grow v-col" :style="{ gap: '8px', minWidth: 0 }">
        <div class="v-row-between">
          <span class="v-section-title">进行中的任务</span>
          <VocaButton variant="ghost" size="sm" @click="router.push({ name: 'tasks' })">查看全部任务 <ChevronRight :size="12" /></VocaButton>
        </div>
        <button v-for="task in activeTasks" :key="task.id" class="v-task-row" @click="openTask(task.id)">
          <span class="flex min-w-0 flex-1 flex-col gap-[3px]">
            <span class="text-[13px] font-medium">{{ task.name }}</span>
            <span class="flex flex-wrap items-center gap-x-2">
              <span class="v-mono v-tiny v-muted">{{ task.workspace }}</span>
              <span class="v-tiny v-muted">· {{ task.team }}</span>
              <span class="v-tiny v-muted">· Run {{ task.runId }}</span>
              <span class="v-tiny v-muted">· {{ task.subtasks.filter((s) => s.state === "success").length }}/{{ task.subtasks.length }} 子任务</span>
            </span>
          </span>
          <VocaStatus :meta="taskStateMeta(task.state)" />
          <ChevronRight :size="13" class="v-muted" />
        </button>
        <div v-if="!activeTasks.length" class="v-card v-empty">
          <span class="text-[15px] font-semibold">开始你的第一项任务</span>
          <span class="v-small">在上方输入任务后点击“创建任务”。如果还没有可用的执行工具，先去“本地工具”完成安装与认证。</span>
          <div class="flex items-center gap-2">
            <VocaButton size="sm" @click="router.push({ name: 'tools' })">去本地工具</VocaButton>
            <VocaButton size="sm" variant="ghost" @click="router.push({ name: 'agents' })">去创建 Agent 角色</VocaButton>
          </div>
        </div>
      </div>

      <div class="v-col-rail v-col" :style="{ gap: '8px' }">
        <div class="v-row-between">
          <span class="flex items-center gap-[7px]">
            <span class="v-section-title">待审批</span>
            <span v-if="pendingApprovals.length" class="v-badge is-blocked" :style="{ padding: '0 6px' }">{{ pendingApprovals.length }}</span>
          </span>
          <VocaButton variant="ghost" size="sm" @click="router.push({ name: 'approvals' })">全部</VocaButton>
        </div>
        <div v-for="approval in pendingApprovals" :key="approval.id" class="v-card v-panel" :style="{ gap: '7px' }">
          <div class="v-row-between">
            <VocaStatus :meta="{ label: '等待审批', tone: 'blocked', icon: 'alert' }" />
            <span class="v-tiny v-muted">{{ approval.age }}</span>
          </div>
          <div class="v-mono" :style="{ fontSize: '11.5px' }">{{ approval.title }}</div>
          <div class="v-tiny v-muted">{{ approval.agent }}</div>
          <div class="v-tiny flex items-center gap-[5px]" :style="{ color: 'var(--status-blocked)' }">
            <ShieldCheck :size="11" />{{ approval.risk[0] }}
          </div>
          <div class="flex items-center justify-end gap-[6px]">
            <VocaButton size="sm" variant="ghost" @click="decide(approval.id, 'deny')">拒绝</VocaButton>
            <VocaButton size="sm" @click="decide(approval.id, 'allow-once')">仅此一次允许</VocaButton>
          </div>
        </div>
        <div v-if="!pendingApprovals.length" class="v-card v-panel"><span class="v-small v-muted">没有待处理的审批。</span></div>

        <div class="v-row-between" :style="{ marginTop: '2px' }">
          <span class="v-section-title">最近完成</span>
          <VocaButton variant="ghost" size="sm" @click="router.push({ name: 'history' })">历史记录 <ChevronRight :size="12" /></VocaButton>
        </div>
        <div class="v-card" :style="{ padding: '0 12px 4px' }">
          <div v-for="task in doneTasks" :key="task.id" class="flex items-center gap-[9px] border-t py-2" :style="{ borderColor: 'var(--border-default)' }">
            <Check :size="13" :style="{ color: 'var(--status-success)' }" />
            <span class="flex-1 truncate text-[11.5px]">{{ task.name }}</span>
            <span class="v-tiny v-muted">{{ task.runId }}</span>
          </div>
        </div>
      </div>
    </div>

    <CreateTaskDialog v-model="dialogOpen" :default-workspace="workspace" @submit="handleDialogSubmit" />
  </div>
</template>

<style scoped>
.composer-input {
  width: 100%;
  border: 0;
  background: transparent;
  resize: none;
  min-height: 42px;
  font-size: 13.5px;
  line-height: 1.55;
  color: var(--text-primary);
  outline: none;
}
.composer-input::placeholder { color: var(--text-muted); }
</style>
