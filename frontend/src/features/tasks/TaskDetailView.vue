<script setup lang="ts">
import { computed, nextTick, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { ChevronLeft, Folder, GitBranch, Hourglass, Info, MoreHorizontal, PanelRight, Paperclip, Pause, Play, Square, Users } from "lucide-vue-next";
import VocaButton from "@/components/ui/VocaButton.vue";
import VocaIconButton from "@/components/ui/VocaIconButton.vue";
import VocaStatus from "@/components/ui/VocaStatus.vue";
import TaskFlow from "./TaskFlow.vue";
import TaskChat from "./TaskChat.vue";
import InspectorPanel from "@/features/inspector/InspectorPanel.vue";
import { useAppStore } from "@/stores/app";
import { useApprovalsStore } from "@/stores/approvals";
import { useTasksStore } from "@/stores/tasks";
import { taskBannerTone, taskStateMeta } from "@/lib/status";

const route = useRoute();
const router = useRouter();
const app = useAppStore();
const tasks = useTasksStore();
const approvals = useApprovalsStore();

const inspectorOpen = ref(false);
const inspectorTab = ref<"activity" | "logs" | "results" | "info">("logs");
const selectedSubtask = ref<string | null>(null);
const input = ref("");
const instructionTarget = ref<string | null>(null);
const highlight = ref<string | null>(null);

const taskId = computed(() => String(route.params.id ?? ""));
const task = computed(() => tasks.tasks.find((item) => item.id === taskId.value) ?? null);
const isSingleAgent = computed(() => task.value?.mode === "single-agent");
const meta = computed(() => (task.value ? taskStateMeta(task.value.state) : null));
const pendingApproval = computed(() => approvals.pending.find((item) => item.taskId === taskId.value) ?? null);

const counts = computed(() => {
  const list = task.value?.subtasks ?? [];
  return {
    done: list.filter((item) => item.state === "success").length,
    running: list.filter((item) => item.state === "running").length,
    blocked: list.filter((item) => ["waiting-approval", "blocked", "waiting-dependency"].includes(item.state)).length,
    wait: list.filter((item) => item.state === "pending").length,
    total: list.length,
  };
});

const banner = computed(() => {
  const current = task.value;
  if (!current) return null;
  if (pendingApproval.value) {
    return { tone: "is-blocked" as const, text: `1 项子任务等待你的审批：${pendingApproval.value.title.replace("写入文件 ", "")}。在决定前，本任务不会继续派发新工作。` };
  }
  switch (current.state) {
    case "pause-pending":
      return { tone: "is-neutral" as const, text: "等待暂停：已停止派发新工作，当前执行仍在运行。到达安全边界后才会显示“已暂停”。" };
    case "paused":
      return { tone: "is-neutral" as const, text: "已暂停：所有执行均已到达安全边界，调度停止。继续前会检查权限、依赖与输入版本。" };
    case "stop-pending":
      return { tone: "is-neutral" as const, text: "正在请求停止：已向运行时发出停止请求，尚未全部确认。这不等于“已停止”。" };
    case "stopped":
      return { tone: "is-neutral" as const, text: `已停止：${counts.value.done} 项成果已保留。重试会创建新的 Run，旧记录不变。` };
    case "failed":
      return { tone: taskBannerTone(current.state), text: `${current.stateNote}。失败节点及其依赖阻塞项需要检查。` };
    case "needs-attention":
      return { tone: taskBannerTone(current.state), text: `${current.stateNote}。请检查阻塞项并选择后续操作。` };
    case "partial":
      return { tone: taskBannerTone(current.state), text: `${current.stateNote}。成果不完整，完成项与跳过项已分组标注。` };
    case "queued":
      return { tone: "is-neutral" as const, text: "排队等待资源：当前并发已满。任务未被丢弃，资源释放后自动开始。" };
    case "draft":
      return { tone: "is-neutral" as const, text: "待配置占位：尚未定义执行计划，因此不会进入调度或开始执行。" };
    default:
      return null;
  }
});

onMounted(async () => {
  if (!tasks.tasks.length) await tasks.load();
  if (!approvals.approvals.length) await approvals.load();
  tasks.select(taskId.value);
});

watch(taskId, (id) => tasks.select(id));

async function openSubtask(id: string): Promise<void> {
  selectedSubtask.value = id;
  inspectorOpen.value = true;
  inspectorTab.value = "logs";
}

async function send(): Promise<void> {
  if (!task.value) return;
  const text = input.value.trim();
  if (!text) {
    app.toast("请先输入内容", "neutral");
    return;
  }
  if (instructionTarget.value) {
    tasks.setInstruction(task.value.id, instructionTarget.value, text);
  } else {
    tasks.appendMessage(task.value.id, text);
  }
  input.value = "";
  instructionTarget.value = null;
}

async function decide(id: string, decision: "allow-once" | "deny"): Promise<void> {
  await approvals.decide(id, decision);
  await tasks.load();
}

function locate(): void {
  highlight.value = selectedSubtask.value;
  inspectorOpen.value = false;
  app.toast("已在执行流程中定位该子任务", "success");
  void nextTick(() => {
    window.setTimeout(() => (highlight.value = null), 2400);
  });
}

function exportSummary(): void {
  const current = task.value;
  if (!current) return;
  const content = [
    `# ${current.name}`,
    `- 工作空间：${current.workspace}`,
    `- 团队：${current.team}`,
    `- Run：${current.runId}`,
    `- 状态：${current.stateNote}`,
    "",
    ...current.subtasks.map((item) => `- [${item.state}] ${item.name} · ${item.assignee} · ${item.result}`),
  ].join("\n");
  const blob = new Blob([content], { type: "text/markdown;charset=utf-8" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `task-${current.id}-summary.md`;
  anchor.click();
  URL.revokeObjectURL(url);
  app.toast("已导出执行摘要（演示内容）", "success");
}
</script>

<template>
  <div v-if="task" class="v-content is-task">
    <div class="v-col" :style="{ gap: '8px', flex: '0 0 auto' }">
      <div class="flex flex-wrap items-center gap-2">
        <VocaIconButton label="返回全部任务" @click="router.push({ name: 'tasks' })"><ChevronLeft :size="16" /></VocaIconButton>
        <h1 class="min-w-0 flex-1 text-[18px] font-semibold leading-tight" :style="{ letterSpacing: '-0.3px' }">{{ task.name }}</h1>
        <VocaStatus v-if="meta" :meta="meta" />
        <span class="v-spacer" />
        <template v-if="task.nativeExecution">
          <VocaButton v-if="['queued', 'starting', 'running'].includes(task.state)" @click="tasks.stop(task.id)"><Square :size="13" />停止只读任务</VocaButton>
          <VocaButton v-else-if="task.state === 'stop-pending'" disabled>正在停止；等待进程退出确认…</VocaButton>
        </template>
        <template v-else-if="['running', 'waiting-approval'].includes(task.state)">
          <VocaButton @click="tasks.pause(task.id)"><Pause :size="13" />暂停</VocaButton>
          <VocaButton @click="tasks.stop(task.id)"><Square :size="13" />停止</VocaButton>
        </template>
        <template v-else-if="task.state === 'pause-pending'">
          <VocaButton disabled><Pause :size="13" />等待暂停…</VocaButton>
          <VocaButton @click="tasks.stop(task.id)">请求停止当前执行</VocaButton>
        </template>
        <template v-else-if="task.state === 'paused'">
          <VocaButton variant="primary" @click="tasks.resume(task.id)"><Play :size="13" />继续</VocaButton>
          <VocaButton @click="tasks.stop(task.id)">停止</VocaButton>
        </template>
        <template v-else-if="task.state === 'stop-pending'">
          <VocaButton disabled>正在请求停止…</VocaButton>
        </template>
        <template v-else-if="['needs-attention', 'partial', 'failed'].includes(task.state)">
          <VocaButton variant="primary" @click="tasks.retryFailed(task.id)">仅重试失败部分</VocaButton>
          <VocaButton @click="tasks.skipPartial(task.id)">跳过并生成部分成果</VocaButton>
        </template>
        <VocaButton v-else-if="task.state === 'stopped'" variant="primary" @click="tasks.retryFailed(task.id)">在新 Run 中重新开始</VocaButton>
        <VocaIconButton :label="inspectorOpen ? '关闭检查器' : '打开检查器'" @click="inspectorOpen = !inspectorOpen"><PanelRight :size="15" /></VocaIconButton>
        <VocaIconButton label="更多任务操作" @click="exportSummary"><MoreHorizontal :size="16" /></VocaIconButton>
      </div>
      <div class="flex flex-wrap gap-3 v-tiny v-muted">
        <span class="inline-flex items-center gap-[5px]"><Folder :size="11" />{{ task.workspace }}</span>
        <span class="inline-flex items-center gap-[5px]"><Users :size="11" />{{ task.team }}</span>
        <span class="inline-flex items-center gap-[5px]"><GitBranch :size="11" />{{ task.runId }}</span>
        <span class="inline-flex items-center gap-[5px]"><Hourglass :size="11" />{{ task.startedAt === "未开始" ? "尚未开始" : `开始于 ${task.startedAt}` }} · {{ task.stateNote }}</span>
      </div>
    </div>

    <div v-if="banner" class="v-banner" :class="banner.tone">
      <Info :size="13" />
      <span class="flex-1">{{ banner.text }}</span>
      <VocaButton v-if="pendingApproval" size="sm" @click="router.push({ name: 'approvals' })">查看审批</VocaButton>
    </div>

    <div class="v-card" :style="{ display: 'flex', alignItems: 'center', padding: '8px 4px', flex: '0 0 auto' }">
      <div class="flex flex-1 items-baseline gap-[6px] px-[14px]"><b :style="{ color: 'var(--status-success)', fontSize: '15px' }">{{ counts.done }}</b><span class="v-small v-secondary">已完成</span></div>
      <div class="flex flex-1 items-baseline gap-[6px] px-[14px]"><b :style="{ fontSize: '15px' }">{{ counts.running }}</b><span class="v-small v-secondary">执行中</span></div>
      <div class="flex flex-1 items-baseline gap-[6px] px-[14px]"><b :style="{ color: 'var(--status-blocked)', fontSize: '15px' }">{{ counts.blocked }}</b><span class="v-small v-secondary">阻塞/待审批</span></div>
      <div class="flex flex-1 items-baseline gap-[6px] px-[14px]"><b :style="{ fontSize: '15px' }">{{ counts.wait }}</b><span class="v-small v-secondary">未开始</span></div>
      <div class="flex flex-1 items-baseline gap-[6px] px-[14px]"><b :style="{ fontSize: '15px' }">{{ counts.total }}</b><span class="v-small v-secondary">子任务总数</span></div>
    </div>

    <div class="v-row-between">
      <span class="v-section-title">执行流程</span>
      <span class="v-tiny v-muted">组织关系 ≠ 任务依赖：这里表达先后依赖</span>
    </div>
    <TaskFlow :subtasks="task.subtasks" :highlight-id="highlight" @open="openSubtask" />

    <div class="v-row-between">
      <span class="v-section-title">{{ task.nativeExecution ? "只读任务" : task.mode === "custom" ? "执行计划" : isSingleAgent ? "Agent 对话" : "主管对话" }}</span>
      <span v-if="task.mode !== 'custom' && !task.nativeExecution" class="v-tiny v-muted">原始 Agent 日志在右侧检查器中按需查看</span>
    </div>
    <div v-if="task.nativeExecution" class="v-card v-panel" :style="{ flex: '1 1 auto', minHeight: '200px' }">
      <span class="v-small font-medium">本机只读执行</span>
      <span class="v-tiny v-muted">Provider：Claude Code · 权限：Read、Glob、Grep · Shell、写入、MCP 与子 Agent 不可用。</span>
      <div class="v-divider" />
      <span class="v-small whitespace-pre-wrap">{{ task.prompt }}</span>
      <span class="v-tiny v-muted">运行输出与进度位于右侧“日志”面板。暂停、恢复、重试、追加对话和文件产物本阶段未实现。</span>
    </div>
    <div v-else-if="task.mode === 'custom'" class="v-card v-empty" :style="{ flex: '1 1 auto', minHeight: '200px' }">
      <span class="text-[13px] font-semibold">执行计划尚未配置</span>
      <span class="v-small">此任务是待配置占位，不会被调度。配置自定义子任务和依赖后，才能开始执行。</span>
    </div>
    <TaskChat v-else :task="task" :approval="pendingApproval" @decide="decide" />

    <div v-if="task.mode !== 'custom' && !task.nativeExecution" class="v-card" :style="{ position: 'sticky', bottom: 0, zIndex: 3, padding: '10px 12px', display: 'flex', flexDirection: 'column', gap: '7px', boxShadow: '0 -8px 18px -14px rgb(0 0 0 / 35%)' }">
      <div class="flex items-center gap-2">
        <textarea
          v-model="input"
          rows="1"
          class="min-h-[22px] flex-1 resize-none bg-transparent text-[11.5px] leading-normal outline-none"
          :placeholder="isSingleAgent ? '向目标 Agent 发送消息，或输入 @ 指定 Agent 发送定向指令…' : '向主管提问，或输入 @ 指定某个 Agent 发送定向指令…'"
          :aria-label="isSingleAgent ? '向目标 Agent 发送消息' : '向主管发送消息'"
          @keydown.enter.exact.prevent="send"
        />
      </div>
      <div class="flex items-center gap-[10px] v-tiny v-muted">
        <span>发送给：{{ instructionTarget ? `定向 ${instructionTarget}` : isSingleAgent ? task.team.replace(/ · 单 Agent$/, "") : `主管（${task.supervisor}）` }}</span>
        <span>· 定向指令{{ isSingleAgent ? "按 Mock 演示状态更新" : "会同步主管并显示投递状态" }}</span>
        <span class="v-spacer" />
        <button class="v-tiny" :style="{ border: '1px solid var(--border-default)', borderRadius: '6px', padding: '2px 8px' }" @click="instructionTarget = instructionTarget ? null : '分析师'">
          @ {{ instructionTarget ?? "主管" }}
        </button>
        <VocaIconButton label="添加附件" @click="app.toast('附件：演示环境不读取真实文件', 'neutral')"><Paperclip :size="13" /></VocaIconButton>
        <VocaButton size="sm" @click="send">发送</VocaButton>
      </div>
    </div>
  </div>

  <div v-else class="v-content">
    <div class="v-empty"><span class="text-[15px] font-semibold">任务不存在或已被归档</span><span class="v-small">请从“全部任务”重新选择。</span></div>
  </div>

  <InspectorPanel
    v-if="inspectorOpen && task"
    :task="task"
    :subtask-id="selectedSubtask"
    :tab="inspectorTab"
    @update:tab="inspectorTab = $event"
    @close="inspectorOpen = false"
    @locate="locate"
  />
</template>
